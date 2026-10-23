# NORD Flight — Atmospheric Low-Altitude Combat Performance Review

**Date:** 2026-10-23
**Scope:** in-atmosphere flight, especially low-altitude combat close to terrain. Orbital and lunar paths are excluded.
**Target:** Apple M1 MacBook Air, 8 GB unified memory. Steady 60 FPS (16.67 ms frame budget). Fullscreen cockpit and external views are equally important. Visual and simulation fidelity must be kept.
**Method:** read-only review of the current working tree (Bevy 0.18.1 with patched `vendor/bevy_pbr`) plus the saved benchmark and Metal artifacts under `tests/`. No new benchmarks or profiling were run. Nothing was implemented.

> **How to read the numbers.** Every FPS, frame-time or GPU-interval figure here comes from a **historical** run (mostly `tests/optimization8/`). None measures the current working tree. Metal System Trace channel intervals **overlap**: they are never summed into a frame total and never treated as exclusive pass durations. Any sizes marked "derived" are arithmetic from source constants, not measurements. No speedups are predicted.

---

## 1. Executive summary

### Why low altitude is slow — the short version

1. **Too many full-resolution passes for 2880×1800 on an M1.** Each frame runs about eight full-screen passes over an Rgba16F HDR target: opaque, sky/aerial perspective, transparent, bloom, tonemapping, FXAA, UI, then an upscale blit. This is the steady cost for both views, and FPS scales strongly with resolution: 36 FPS fullscreen versus 51 FPS at 1440×900 for the same cockpit binary.
2. **The render origin follows the aircraft every frame.** That one design choice has three costs:
   - every terrain and forest root transform is rewritten, propagated and re-extracted to the GPU each frame;
   - the environment probe's input key changes every frame, so cubemap capture and filtering never reach their cache-hit path;
   - shadow cascades are rebuilt around a moving origin.
3. **Shadow cascades are large and redraw everything every frame.** There are 2 cascades out to 2.5 km. The outer cascade box is about 5–7 km across and redraws all forest batches plus the whole own aircraft and cockpit, using fat 48-byte unshared vertices with `cull_mode: None`.
4. **Terrain publication flips a whole layout at once.** Each 512 m cell crossing makes 9–32 tiles visible and hides 3–29 in a single frame, with all their forest batches. This is the main spike source while cruising low.
5. **Combat adds per-frame asset churn.** Up to 256 particle materials, impact-flash meshes, CPU smoke meshes, F-22 engine-glow materials and many text blocks are rewritten every frame. Systems holding `ResMut<Assets<…>>` serialize the Presentation stage.
6. **The simulation is frame-locked.** It advances a fixed `warp.timestep` (default 1/60 s) per rendered frame, so 45 FPS plays at 0.75× speed and 30 FPS at 0.5×. Players feel the FPS drop twice: once as low frame rate, once as slow motion.
7. **VSync quantization hides partial gains.** On the 60 Hz panel any frame over 16.67 ms is shown for 33.3 ms. The historical p95 of about 34.6 ms is exactly this two-vblank step. Fixes must be validated with `NORD_PRESENT_MODE=no-vsync` p50/p95/p99, not only mean FPS.

### Top priorities

| Rank | Cockpit view | External view |
|---|---|---|
| 1 | **R1** Fuse the full-resolution post chain and sky pass | **R1** Fuse the full-resolution post chain and sky pass |
| 2 | **R2a** Mirror-mip dispatch fix (S, identical output); **R2b** probe-key tolerance (needs sign-off) | **R3** Shadow-only forest and aircraft proxies |
| 3 | **R3** Outer cascade: own aircraft/cockpit and forest proxies | **T1** Quantized render anchor (removes per-frame terrain re-extraction and probe churn) |
| 4 | **T1** Quantized render anchor | **T2** Swap-group terrain publication (spikes) |
| 5 | **R4** Combiner single layer; attribute the transparent cost per draw before building a glass shader | **C1–C4** Combat asset churn and LOS gate ordering |
| 6 | **C1–C4** Combat churn; **R5** F-22 glow materials | **R6** Additive-effect overdraw, **R7** globe patching |
| Always | **S1** decide the real-time simulation contract | **S1** same |

The cheap, exact, low-risk items worth doing first, before any large work:
- R2a (mirror-mip lanes);
- C1 (cheap gates before terrain LOS);
- C3 (change-only text writes);
- T3 (pre-sized mesh slabs);
- T7 (single-bundle forest spawn);
- R8 and T8 (drop the dead forest UV);
- R5 star-material change check;
- T6a (LOD hysteresis).

### Engine recommendation (detail in §9)

**Stay on Bevy for now, and pull the simulation out into an engine-independent crate either way.** None of the engine-specific bottlenecks found here is something another engine fixes for free. The big costs come from the project's own choices: pass structure, the moving origin, cascade setup, publication granularity, per-frame asset mutation and frame-locked sim. Any engine would need the same fixes.
- **Ruled out for the M1/8 GB target:**
  - Unreal: Epic lists 16 GB as the Mac minimum, and Nanite and Virtual Shadow Maps need M2 or newer.
  - Unity HDRP: in maintenance mode.
  - Fyrox: OpenGL on macOS.
- **The only credible alternatives:** Unity 6 URP and Godot 4.x with godot-rust. Each means rewriting roughly 35k lines of rendering, UI and effects. Neither should be chosen without the limited prototype in §10.

---

## 2. Evidence base and its limits

### 2.1 What exists

From `tests/optimization8/` (the latest pass). Default preset, Gripen, terrain-forest scene, scripted `terrain-stream` route at 50 m AGL and 100 m/s, **simulation paused**, one 25 s sample after a 10 s warm-up, battery power:

| Run (`tests/optimization8/…`) | Baseline → candidate | p50 ms | p95 ms | p99 ms | Frames >50 ms |
|---|---|---|---|---|---|
| External 2880×1800 (B→C) | 35.36 → 46.74 FPS | 32.6 → 16.9 | 36.6 → 34.6 | 49.7 → 52.9 | 8 → 13 |
| External 2880×1800 (C→B) | 35.92 → 46.18 FPS | 26.9 → 16.7 | 33.8 → 34.6 | 37.8 → 35.0 | 3 → 1 |
| Cockpit 2880×1800 (B→C) | 35.16 → 36.42 FPS | 33.3 → 33.2 | 34.3 → 34.7 | 34.9 → 36.5 | 2 → 5 |
| Cockpit 2880×1800 (C→B) | 18.04 → 28.46 FPS | 50.5 → 33.3 | 71.1 → 50.7 | 149 → 51.6 | 259 → 68 |
| Live cockpit, 350 m AGL (B→C) | 28.27 → 26.37 FPS | 33.4 | 52.9 | ~54 | 104 → 136 |
| Windowed cockpit 1440×900 (B→C) | 43.31 → 51.49 FPS | 17.1 → 16.8 | 34.1 → 33.7 | 35.5 → 34.4 | 2 → 2 |
| `stability/streaming-120s.json` (candidate cockpit) | 38.63 FPS | — | 34.12 | 35.31 | 14 |

Metal System Trace, fullscreen cockpit (`tests/optimization8/metal/aggregate.json`): median active interval per frame, **overlapping channels, not additive**.
- Main transparent fragment 17.51 ms.
- Main opaque fragment 13.64.
- Radiance compute 4.46.
- Opaque vertex 3.91.
- FXAA 3.34.
- Outer-cascade shadow vertex 2.59.
- Sky fragment 2.53.
- Tonemapping 2.01.
- Bloom final upsample 1.58.
- Cascade-0 vertex 1.55.
- Irradiance 1.40.
- Upscaling 0.91.
- Atmosphere LUTs 0.79.

Other things the capture shows:
- **Presentation waits:** 127 "next drawable" waits, median 37.97 ms. This is GPU backpressure, not CPU work.
- **Staging-buffer churn:** about 44 `(wgpu internal) Staging` buffer creations per frame (`resource-summary.json`). The source is unattributed; cockpit display uploads are tiny (83 KB per 10 s).

### 2.2 What is missing or not representative

- **No measurement of the current binary or working tree.** The installed app's hash differs from the opt8 candidate. The near-terrain render files are unchanged since opt8. Combat, F-22 visuals, `main.rs` and `world.rs` have changed.
- **No benchmark covers low-altitude combat.** Combat starts at 2,000 m (`src/combat.rs:211-213`) and BVR at 11 km (`src/combat/bvr.rs:139-141`). Every near-terrain run is the terrain-forest scene with no combat. **The primary workload in this review has never been measured.**
- **No opt7/opt8 Metal capture of the external view.** The only external captures are debug, pre-opt7 builds with the environment unrecorded.
- **Sampling limits:**
  - one 25 s run per cell, with run order reversed only once and no variance estimate;
  - battery power throughout opt8;
  - single `ps` snapshots showing heavy background load (Creative Cloud up to 95%, Codex 70%, Messages 64%);
  - the same baseline binary measured about 40 FPS in opt7 but 35.4–35.9 FPS in opt8.
- **The live cockpit (26–28 FPS) is slower than the paused cockpit (35–36 FPS) and nothing explains why.** FPS-only runs turn the simulation counters off. Live runs used an empty script, so altitude, forest coverage and whether the aircraft crashed are unknown.
- **Unresolved tail regression:** `tests/atmospheric6/final/` pair 2 has p99 25.5 → 141.7 ms and 0 → 49 frames over 50 ms.

### 2.3 Superseded claims in older docs

| Old claim | Current truth | Evidence |
|---|---|---|
| `TERRAIN_PERFORMANCE.md`: fine forest batching "disabled by default" | 512 m partition **on** by default; `NORD_TERRAIN_LARGE_BATCHES` restores the old path | `src/terrain.rs:363-369` |
| `TERRAIN_PERFORMANCE.md`: depth prepass "removed" | Re-added behind `NORD_TERRAIN_DEPTH_PREPASS` (off) | `src/main.rs:1394-1398` |
| `NEAR_TERRAIN_FPS_ANALYSIS.md` §3: 864 environment writes vs 144 captures | Fixed by P1 source gating (40/40, 773/773) | `vendor/bevy_pbr/src/light_probe/generate.rs:816-841` |
| ANALYSIS §5 / §6.4: separate forest transforms, no stale-cell coalescing | Fixed | `src/terrain.rs:390-399`, `:546-556` |
| `OPTIMIZATION_REPORT` / `OPTIMIZATION3`: environment filtering dominates (~20–22 ms radiance) | Superseded: 4.46 ms median in opt8 | `tests/optimization8/metal/aggregate.json` |
| `PERFORMANCE.md` / `ATMOSPHERIC_OPTIMIZATION.md`: 59.7–59.8 FPS at 1440×900; "low terrain 40.26 → 59.79" | Not reproduced later (atmospheric6 baseline 31.5–43.6). The opt4 row used the 260 m preset, not the 50 m route | `tests/atmospheric6/final/accepted-summary.json` |
| Opt5: "one replacement tile per frame" | Replaced by 4 meshes / 2 MiB per frame | `src/terrain.rs:211-221` |
| `CODE_OPTIMIZATION_REVIEW` #9: camera orientation in the environment key | Resolved by the canonical probe | `src/atmospheric_render.rs:44-46` |
| `CODE_OPTIMIZATION_REVIEW` #5: publication burst | Partly resolved: staging is bounded, publication is still atomic | `src/terrain.rs:577-592` |
| `README.md:294`: five-second warm-up | Default warm-up is 20 s, sample 40 s | `src/benchmark.rs:37-38` |
| Opt7: "no consistent fullscreen FPS gain" | Contradicted for external view (opt8 repeat); cockpit/live still unproven | `tests/optimization8/results.json` |

---

## 3. Existing optimizations (verified present — not recommendations)

| Optimization | Location | Default |
|---|---|---|
| Async terrain build on the async compute pool, `poll_once` | `src/terrain.rs:544`, `:609-612` | On (1 async thread, `src/main.rs:1123-1128`) |
| Stale-work coalescing / one retarget per completion | `src/terrain.rs:180-190`, `:546-558` | On |
| Bounded staging (4 meshes / 2 MiB source bytes) and bounded retirement | `src/terrain.rs:211-221`, `:420-458` | On |
| GPU-ready gate before publication (`TerrainReady`) | `src/terrain_ready.rs:39-50`; `src/terrain.rs:571-576` | On |
| Atomic, hole-free layout publication; first-visible-frame transforms | `src/terrain.rs:577-592` | On |
| Forest 512 m partition | `src/terrain.rs:363-369` | On |
| Forest children inherit the tile root transform | `src/terrain.rs:390-399` | On |
| Procedural tree collision (no per-tile collision mesh) | `src/terrain.rs:69-94` | On |
| Static atmosphere LUT cache; exact sky/aerial LUT reuse | `vendor/bevy_pbr/src/atmosphere/node.rs:165`, `:224-256` | On |
| Environment bind-group and content caches | `generate.rs:134-139`, `:1155` | On |
| Six-stage sliced environment filtering; parallel filter kernel | `generate.rs:843-853` | On |
| P1 environment source gated to the filter's immutable capture | `generate.rs:816-841`; `atmosphere/environment.rs:398-407` | On |
| Active extracted-camera gate | `generate.rs:643-656` | On |
| Canonical (orientation-free) indirect probe | `src/atmospheric_render.rs:44-46` | On |
| Zero-alpha discard only in alpha-blend pipelines (opaque specialization) | `vendor/bevy_pbr/src/render/mesh.rs:2372-2387`, `render/pbr.wgsl:76` | On |
| Zero-contribution point/spot light skips | `vendor/bevy_pbr/src/render/pbr_lighting.wgsl:635-648` | On |
| Hide zero-opacity haze/cloud shells | `src/earth.rs:28-35` | On |
| HUD coverage crop / hide-when-empty | `src/cockpit.rs:603-606`; `src/cockpit/coverage.rs:9-31` | On |
| Persistent dirty-row display uploads | `src/display_upload.rs:26-52` | On |
| Rigid opaque batching; identical-mesh sharing | `src/render_batch.rs:7-30`; `src/mesh_reuse.rs:35` | On |
| Earth vertex colours on GPU | `src/earth_gpu.rs:83` | On |
| Exact 256-entry terrain-LOS cache | `src/combat/guidance.rs:183-226` | On (thread-local, exact keys) |
| BVR AI integration parallel at ≥ 8 targets | `src/combat/bvr.rs:489-530` | On |
| Reused BVR work buffers | `src/combat/bvr.rs:126-132` | On |
| GPU missile smoke | `src/combat/smoke_gpu.rs:39-43` | **Off** (opt-in `NORD_SMOKE_GPU`) |
| Generic depth prepass | `src/main.rs:1394-1398` | **Off** (experiment was not a win) |

---

## 4. Findings overview

Categories: **SUS** = sustained frame cost, **SPK** = intermittent spike, **SIM** = perceived simulation lag, **MEM** = memory pressure on 8 GB.
Evidence: **H** = measured historically, **S** = confirmed by source, **Hyp** = hypothesis.
Effort: S / M / L.

| ID | Finding | Cat. | Views | Evidence | Effort |
|---|---|---|---|---|---|
| R1 | Full-resolution HDR pass chain (sky, post, UI, blit) | SUS | Both | S + H (resolution scaling, FXAA/sky intervals) | L |
| R2 | Environment probe: mirror-mip lane waste; source changes every frame | SUS | Both | S + H (radiance 4.46 ms) | S (a) / S–M (b) |
| R3 | Outer shadow cascade redraws forest + own aircraft/cockpit with fat vertices | SUS | Both | S + H (outer-cascade vertex 2.59 ms) | M |
| R4 | Cockpit transparency: F-22 combiner box is double-layer lit; other canopies full-PBR blended | SUS | Cockpit | S; attribution unresolved | S / M |
| R5 | Per-frame material/mesh mutation (engine glow, stars, decoy, flashes) | SUS/SPK | Both | S | S–M |
| R6 | Combat transparency overdraw (smoke, particles, vapour, exhaust) | SPK | External mostly | S | S–M |
| R7 | 590k-triangle globe drawn at low altitude | SUS | Both | S | S–M |
| R8 | Forest vertex format and culling | SUS/MEM | Both | S | S (L for instancing) |
| T1 | Render origin follows the aircraft; all terrain is re-extracted each frame | SUS | Both | S | L |
| T2 | Whole-layout visibility flip on commit | SPK | Both | S (magnitude Hyp) | M |
| T3 | Mesh-allocator slab growth copies whole slabs | SPK | Both | S (frequency Hyp) | S |
| T4 | Terrain meshes keep CPU copies and Extract clones them | MEM/SPK | Both | S | S–M |
| T5 | Serial, non-overlapped build pipeline on one async thread | LOD lag | Both | S | M |
| T6 | No LOD hysteresis / no retired-tile cache | SPK | Both | S | S–M |
| T7 | Multi-step archetype churn per staged forest entity | SPK (small) | Both | S | S |
| T8 | Hidden staged/retiring entities still get transform writes | SUS (minor) | Both | S | S |
| T9 | Staging budget is source bytes, not frame-time aware | SPK | Both | S | S |
| T10 | Synchronous startup layout build | Startup | — | S | S |
| T11 | Thread oversubscription / P-core contention | SUS jitter | Both | Hyp | S |
| C1 | Terrain LOS before cheap gates (AI visibility, warning, seeker, RWR) | SUS | Both | S | S |
| C2 | Particle pool: 256 unique materials mutated per frame, unstable slots, no culling | SUS/SPK | Both | S | M |
| C3 | Text re-assigned every frame | SUS | Both | S | S |
| C4 | Impact-flash mesh and material rebuilt per frame | SPK | Both | S | S–M |
| C5 | CPU smoke trail meshes rebuilt every frame; GPU path opt-in and still mutates materials | SUS | Both | S | M |
| C6 | Asset-writer systems serialize Presentation | SUS | Both | S | M |
| C7 | HUD as 540 absolutely positioned UI nodes | SUS | Cockpit / HUD | S | S / M |
| P1 | Repeated `terrain_height` calls per aircraft per substep | SUS | — | S | S |
| P2 | Conservative terrain upper bound for LOS / projectile / fragment ground tests | SUS | — | S | M |
| P3 | Near-ground hull check has no per-hull terrain broad phase | SUS (taxi/crash/NOE) | — | S | M |
| P4 | Small per-frame/per-substep allocations | SUS (minor) | — | S | S |
| S1 | Simulation advances a fixed amount per rendered frame | SIM | Both | S | M |

---

## 5. Rendering findings

### R1 — Full-resolution HDR pass chain (sustained, both views)

**Triggers.** Always. The cost scales with pixel count, so fullscreen 2880×1800 (about 5.2 Mpx) is the critical case.

**Source.**
- Camera spawn, `src/main.rs:1361-1393`:
  - HDR, required by `Bloom::NATURAL` (`:1385`), so the view target is Rgba16Float;
  - `Tonemapping::AcesFitted` (`:1384`); DebandDither is on by default;
  - `Msaa::Off` and FXAA (`:1391-1392`);
  - UI renders on the 3D camera (`IsDefaultUiCamera`, `:1364`).
- Runtime AA policy: `src/main.rs:2356-2369`.
- The atmosphere sky/aerial-perspective pass is a separate full-screen pass between opaque and transparent. It samples depth and uses dual-source blending (`vendor/bevy_pbr/src/atmosphere/render_sky.wgsl:32-79`; graph at `vendor/bevy_pbr/src/atmosphere/mod.rs:199-216`).

**Pass chain per frame:**
1. Opaque, storing colour and depth.
2. Sky (load and store).
3. Transparent (load and store).
4. Bloom: the first downsample reads full resolution, and the final upsample composites into it.
5. Tonemapping to the ping-pong target.
6. FXAA.
7. UI (load and store).
8. Upscale blit to the swapchain.

A 2880×1800 Rgba16F target is about 41 MB (derived). This chain moves several hundred MB per frame through the M1's shared memory bandwidth.

**Evidence.**
- Structure: confirmed by source.
- Historical: the same cockpit candidate ran 36.42 FPS fullscreen versus 51.49 FPS at 1440×900.
- Metal median intervals: FXAA 3.34 ms, tonemapping 2.01 ms, sky 2.53 ms, upscaling 0.91 ms, bloom final upsample 1.58 ms. These overlap; do not sum them.

**Existing mitigation.** None on the post side.

**Proposed fix (fidelity-preserving):**
- **(a) Fused post node** (effort L). One compute or fragment pass that:
  - applies the bloom final composite;
  - applies AcesFitted and the deband dither;
  - runs FXAA, tonemapping and dithering FXAA's HDR neighbourhood taps in-shader, so FXAA sees the same LDR luma it sees today;
  - writes straight to the swapchain.

  This removes the tonemapping ping-pong and the separate FXAA and upscale passes, about 2–3 full-resolution HDR read+write passes.
- **(b) Aerial perspective in the PBR fragment shader** (effort L). Apply the same aerial-view and transmittance LUT maths at the end of the opaque shader. Draw the sky as a depth-tested skybox-style draw inside the opaque pass. This removes the sky pass's colour load/store.
  - Keep transparent items (canopy, HUD) free of aerial perspective, exactly as now.
  - Differences against today's output should be depth-reconstruction noise only.
- **(c) Hypothesis** (effort M): composite the UI after tonemapping into the LDR/swapchain stage rather than loading and storing the HDR target.
- **Separate, not fidelity-preserving:** a main-pass render scale of about 0.8–0.85 with sharpening, offered only as an optional preset.

**Benefit.** Sustained frame cost, both views; likely the largest single lever at fullscreen.

**Risks.**
- Exact parity of dither and FXAA inputs.
- The vendored `bevy_pbr` patch surface grows, which raises the cost of Bevy upgrades. Bevy 0.19 replaced the render graph with ECS schedules.

**Verify.**
- Xcode GPU frame capture: per-encoder GPU time and memory-bandwidth counters before and after.
- Pixel diff against the current output (cockpit, external, dusk, fog).
- no-vsync p50/p95/p99 at 2880×1800.

### R2 — Environment probe filtering (sustained, both views)

**(a) Mirror (roughness-0) mip wastes most of its lanes.**
- **Source:** `vendor/bevy_pbr/src/light_probe/generate.rs:1739-1748`.
  - With the parallel kernel on, every radiance mip dispatches `(mip_size/8, mip_size, faces)` groups of 64 lanes.
  - For mip 0 of the 512² probe that is about 2.1 M invocations per face. Mips 1–9 together are about 0.7 M (derived).
- **Shader:** `vendor/bevy_pbr/src/light_probe/environment_filter.wgsl:90` and `:100`. Every lane runs `sample_cube_dir` and a blue-noise `sample_noise` texture fetch *before* the roughness-0 early-out (`:106-112`). Only one lane in eight then samples and stores.
- **Fix (S, identical output):**
  - Move `sample_noise` below the mirror branch.
  - For the roughness-0 mip, dispatch `ceil(size/8)²` groups and map `local_invocation_index` to an 8×8 texel tile.
  - Optional: a `copy_texture_to_texture` from the intermediate's mip 0, only if a test proves bit-equality.
- **Evidence:** confirmed by source. Radiance median interval is 4.46 ms historically; this mip's share is unmeasured.
- **Verify:** the existing one/three/six-stage equivalence tests in `generate.rs`; texture readback equality; capture of the radiance encoder.

**(b) The probe source changes every frame in flight.**
- **Source:** `src/atmospheric_render.rs:44-53`.
  - The key includes exact bits of the camera's local translation and of the sun direction in the local frame.
  - Because the render origin and basis follow the aircraft (`src/world.rs:98-121`), both change every frame while moving.
  - So every six-frame cycle rewrites the 6×512² capture, the probe sky LUT and the copy/downsample passes (`atmosphere/environment.rs:388-445`; `generate.rs:1534-1610`), and never reaches the cache-hit path (`generate.rs:1717-1729`).
- **Fix options:**
  - Preferred: fold this into **T1**. A quantized render anchor makes the local frame stable between rebases.
  - Alternative (S–M): key the probe on the quantities its content actually depends on (altitude, sun elevation relative to local up, density, weather, quality), with tight tolerances such as a few metres and about 1e-4 rad.
  - Either way the update cadence changes, which **needs sign-off**, because the existing safeguards deliberately keep exact keys.
- **Benefit:** removes most capture and filter work in level flight; it stays periodic while climbing or descending.
- **Risk:** a stale probe at the tolerance edge. The tolerance must be below visible thresholds; verify with screenshot diffs at dawn and dusk.

**(c) Irradiance sampling.** 1024 scattered taps per texel from level 0 (`environment_filter.wgsl:238`; `generate.rs:1349`, `:1771`). Sampling a pre-filtered mip would be cheaper but changes the output, so it is listed as fidelity-changing and **not recommended** by default.

### R3 — Outer shadow cascade redraws forest and the own aircraft every frame (sustained, both views)

**Source.**
- `src/main.rs:1406-1411`: 2 cascades, first far bound 180 m, maximum 2,500 m, 1024 map (`:1069`, `:1097`); 2048 with high quality or `NORD_DETAIL_QUALITY`.
- Bevy sizes each cascade around the slice AABB using `max(body diagonal, far-plane diagonal)` (bevy_light `cascade.rs:261-300`).
- Shadow culling ignores the near plane (bevy_light `lib.rs:408-411`).

**Mechanism.**
- In the cockpit (75° FOV) the far-plane diagonal at 2.5 km is about 7.2 km, or about 7 m per texel at 1024 (derived). External views at 60° give about 5.4 km.
- The camera is inside every cascade box, so the whole own aircraft and cockpit and every forest batch within range draw into both cascades every frame. For the aircraft that is shadow detail only a few texels wide.
- Trees are 31 triangles with 93 unshared 48-byte vertices, drawn with `cull_mode: None` (`src/terrain.rs:243-317`, `:497-502`).

**Evidence.** Confirmed by source. Historical outer-cascade vertex median 2.59 ms and cascade-0 1.55 ms (overlapping).

**Proposed fix (M, identical shadow maps).**
- Build **shadow-only proxies** with exactly the same triangles: indexed, position-only meshes.
  - Shared vertices bring a tree to about 34 vertices instead of 93, at 12 bytes each instead of 48.
  - Put the proxies on a light-only `RenderLayers` layer (sun on 0|1, camera on 0, proxies on 1), and mark the visible forest batches `NotShadowCaster`.
  - Proxies must spawn, retire and toggle visibility together with their batch: hook stage/retire/reclaim at `src/terrain.rs:420-457`, `:620-640`.
- Do the same for the own aircraft: one merged position-only proxy per rigid or moving assembly, so animated surfaces stay exact.
- Optional, with light-leak risk: `NotShadowCaster` on fully enclosed interior geometry such as intake ducts and closed bays.

**Static shadow reuse** (caching a cascade across frames) is **not recommended** until T1 exists. Today the render origin, sun-relative basis and cascade matrices change every frame, so complete invalidation cannot be guaranteed. After T1 it becomes possible, with invalidation on rebase, sun change, cascade change, terrain commit and any moving or damaged caster.

**Verify.** Shadow-map readback diff (should be zero); shadow-pass vertex time in a GPU capture; the `noshadows` benchmark ablation (`src/main.rs:1288-1302`) as an upper bound.

### R4 — Cockpit transparency (sustained, cockpit only)

**Source facts.**
- **F-22 canopy: not visible from inside.** It uses `cull_mode: Back` (`src/aircraft/visual.rs:1239`) with outward front faces (`:1269`), so from the eye every canopy fragment is a back face and is culled. That is not a cost, but it is a fidelity gap: no inner reflections.
- **F-22 combiner: a cuboid drawn with both faces.** It uses the glass material with `AlphaMode::Blend`, `cull_mode: None` and full lighting (`src/cockpit.rs:140-146`, `:537-554`). Its large faces both get full PBR shading (shadows, IBL, clustered lights, fog). The HUD plane (`:555-602`) adds a third blended lit layer; the existing crop and zero-alpha discard reduce it.
- **Gripen, Su-27, F-35B and Hornet canopies are drawn from inside.**
  - Gripen: `src/world.rs:682-688`, `:705-711`, α 0.30, `cull_mode: None`. The others: `visual.rs:1239`.
  - Each is a full-PBR blended layer over most of the view, and for these aircraft the dominant transparent cost.

**The historical attribution is unresolved.** The 17.51 ms "main transparent fragment" interval in the F-22 cockpit capture does not fit the small F-22 transparent coverage found in source. Likely contributors are the transparent pass's full-resolution load/store (R1) and overlap with concurrent compute (R2). **Measure per draw before building anything here.** Use an Xcode frame capture and `--benchmark-render cockpit-noglass` (`src/main.rs:1295-1300`) to bound the possible gain.

**Fixes:**
- **(S) F-22 combiner as a single quad**, or cull back faces, with α′ = 1 − (1 − α)² so transmission matches the two layers. Glass specular is then counted once instead of twice. That is a small visible difference; confirm with a screenshot diff and accept or reject.
- **(M) Glass-specialised `MaterialExtension`.** Keep the BRDF, IBL, fog and blending; remove unused texture and feature paths at compile time. Only after per-draw attribution shows the canopies matter.

### R5 — Per-frame material and mesh mutation (sustained in flight, spikes in combat)

Every modified asset sends `AssetEvent::Modified`. That triggers render-world re-preparation: a uniform or bind-group rewrite, possibly re-specialisation.

| Source | Location | Note |
|---|---|---|
| F-22 engine glow / shock-cell materials (~14 per engine) | `src/aircraft/visual/raptor_detail.rs:1719-1768` | Change-gated, but the colour includes the `plume_motion` pulse, so it changes almost every frame, cockpit view included |
| Starfield material | `src/world.rs:484-497` | `get_mut` every frame **with no change check**, even when hidden at low altitude |
| Combat particles | `src/combat/effects.rs:557-573`, `:905-935` | See C2 |
| Aero vapour (28 effects) | `src/aero_fx.rs:75-100`, `:153-201` | Unconditional `get_mut`; visibility written without `set_if_neq` (`:145`) |
| Impact flashes | `src/combat.rs:1766-1803` | See C4 |
| CPU missile smoke | `src/combat.rs:2028-2099`; `src/combat/aim9x.rs:497-577` | See C5 |
| Decoy mesh | `src/combat/dogfight.rs:1429-1433` | Rewritten every frame |

**Fixes:**
- **(S)** Add a change check to the star material. `starfield` is a trivially safe fix.
- **(M)** Move per-frame animation (pulse, fade, colour) into the shader: a shared time uniform plus per-entity data through `MeshTag` or a storage buffer, with one material per effect kind. The look is identical.

**Verify.** Count `AssetEvent::Modified` per frame (`src/performance.rs` already has `material_modifications` / `mesh_modifications` at `:306-331`).

### R6 — Combat transparency overdraw (spikes, mostly external)

**Sources.**
- Smoke cards: 6 per segment, `cull_mode: None`.
- Particle spheres: alpha-blended and growing as 1 + 1.5·age (`src/combat/effects.rs:561-566`, `:898-903`).
- Vapour: 25 additive spheres, cone and strands (`src/aero_fx.rs:82-88`).
- Exhaust: 7–8 additive layers per engine (`src/aircraft/visual.rs:958-983`; `raptor_detail.rs:866-880`).

All are `cull_mode: None`.

**Fixes.**
- **(S, mathematically identical)** For closed convex meshes with unlit, uniform-colour **additive** blending: cull back faces and double the colour. Verify each mesh is closed.
- **(S–M, small look change)** For alpha-blended smoke and particles: a near-camera fade or a screen-size cap. Needs sign-off.

**Verify.** Capture at missile launch with the chase camera close behind.

### R7 — Globe mesh drawn every frame at low altitude (sustained)

**Source.** `src/world.rs:223-240`, `:623-653`: a 768×384 sphere, about 590k triangles and 296k vertices (derived), drawn even when almost entirely hidden under terrain. Apple's hidden-surface removal saves its fragments, but not vertex shading and tiling.

**Fix (S–M, identical triangles).** Split it into about 16×8 patches with their own AABBs so frustum culling drops most of them near the ground.

**Verify.** Main-pass vertex time in a capture.

### R8 — Forest vertex format and culling (sustained, memory)

- **Dead UV:** forest meshes carry `UV_0 = [0,0]` (`src/terrain.rs:309-312`), but the tree material has no texture (`:497-502`). Drop it: 8 of 48 bytes per vertex and one interpolant, identical output.
- **Pointless indices:** the index buffer only enumerates vertices in order (`:250-257`). Draw non-indexed, or share vertices as in R3.
- **Optional, small look change:** cap the cones and back-face cull to halve rasterisation setup.
- **Instancing** (L): one prototype plus per-instance data. It cuts memory and submission, **not** vertex invocations. Treat it as a memory win, not a guaranteed frame-time win.

---

## 6. Terrain streaming findings

Derived layout facts (from a scratch copy of `layout()`/`leaves()`):
- Below about 1 km AGL the layout is 231 tiles: 32 × 512 m, 20 × 1 km, 17 × 2 km, 10 × 4 km, 20 × 8 km, 18 × 16 km, 114 × 32 km.
- Each 512 m cell crossing adds 9–32 tiles (mean about 14) and retires 3–29.
- Forest exists only on tiles of 2,048 m or smaller (`src/terrain.rs:244`).
- A ground tile is about 320 KB (derived; `src/world.rs:513-589`). A fully forested 512 m batch is about 0.8 MB (derived).

### T1 — Render origin follows the aircraft; all terrain is re-extracted every frame (sustained, both views)

**Source.**
- `src/world.rs:98-121`: origin = `f.up() * RADIUS` and basis = `frame_uncached(f)`, cached only on exact position bits, so they are recomputed every moving frame.
- `update_world` (`src/world.rs:796-850`) `par_iter_mut`s every `WorldObject` and writes `Transform` whenever it differs. While moving it always differs: the compare at `:834-839` never hits.
- Transform propagation then changes `GlobalTransform` on every terrain root and every forest child. `extract_meshes_for_gpu_building` re-extracts each changed entity and rebuilds its `MeshInputUniform` (`vendor/bevy_pbr/src/render/mesh.rs:1487-1574`).
- Hidden staged and retiring entities are written too (T8).
- Downstream: the probe key churn (R2b) and per-frame cascade movement (R3) come from the same root cause.

**Evidence.** Mechanism confirmed by source; cost not measured. The scene has about 1.5k meshes and 8.5k entities historically.

**Fix (L): quantized, rebased render anchor.**
- Keep origin A and basis R fixed until the aircraft is more than a threshold (about 1–4 km) from A. The camera and aircraft then carry small local offsets; f32 resolution at 4 km is about 0.5 mm.
- Between rebases, terrain transforms stop changing, so the existing change-avoidance compares skip them and nothing is propagated or re-extracted.
- A rebase costs what every frame costs today, but only once. Align rebases with terrain commits.

**Risks.** Everything that assumes "camera ≈ origin" or "+Y = local up" must be audited:
- the Bevy atmosphere and aerial-view LUT;
- earth shells, celestial bodies and cockpit/HUD;
- shadow cascades and the environment probe.

Local vertical tilts by at most threshold/RADIUS, about 0.036° at 4 km. Keep the spacecraft and Moon paths unchanged (`src/world.rs:99-101`).

**Verify.**
- Tracy or `RenderDiagnosticsPlugin` timings for extract and `propagate_transforms`.
- Count `Changed<GlobalTransform>` per frame.
- Pixel-compare stationary captures.
- Run the jitter and z-fighting checks at the rebase threshold.

### T2 — Whole-layout visibility flip on commit (spike, both views)

**Source.**
- `retire()` hides every obsolete root and child (`src/terrain.rs:620-640`).
- In the same frame, every new root and forest child gets `Visibility::Inherited` plus its first transform (`:577-586`), then reclaim starts (`:589`).
- The entity count per commit is unbounded by the staging budget.
- Newly visible entities need full extraction, specialisation in the main view and both shadow cascades, and bin insertion. Hidden ones need removal.

**Evidence.** All-at-once flip confirmed by source; millisecond magnitude is a hypothesis.

**Fix (M): independent swap groups.**
- Partition the delta into connected components of area overlap between retired and new tiles; each is one quadtree split or merge.
- Each group covers identical area, so swapping one group alone never leaves a hole or an overlap.
- Gate GPU readiness per group, and commit at most N entities per frame, nearest groups first.
- This also shortens visible LOD latency: near 512 m tiles are already built first (sort at `:186`, `:610`) but currently wait for the whole delta.

**Risks.** `TerrainReady` needs per-group id sets, and retarget must work per group. Keep the first-visible-transform test (`src/terrain.rs:757-871`).

**Verify.** Per-frame log of commit events and entity counts, correlated with frames over 25 ms.

### T3 — Mesh-allocator slab growth (spike)

**Source.**
- Bevy defaults: 1 MiB minimum slab, ×1.5 growth, 512 MiB maximum (`bevy_render` `mesh/allocator.rs:127-136`).
- Growth allocates a new buffer and GPU-copies the whole old slab (`:842-891`).
- Ground and forest share a vertex layout, so they share slabs that grow with peak resident terrain (active + preparing + retiring).

**Fix (S).** Insert `MeshAllocatorSettings` with `min_slab_size` sized from the measured steady-state peak, so growth happens once at start-up. Also record the maximum slab size and count; `MeshAllocatorDiagnosticPlugin` is wired at `src/performance.rs:174-175`, but `report()` keeps only averages.

**Risk.** Reserving memory up front on 8 GB; size it from measurement.

### T4 — Terrain meshes keep CPU copies and Extract clones them (memory, sync-point cost)

**Source.**
- `RenderAssetUsages::default()` (main + render world) is used for forest (`src/terrain.rs:305`, `:342`) and ground (`src/world.rs:566`).
- With MAIN_WORLD set, `extract_render_asset` deep-clones the mesh inside Extract (`bevy_render` `render_asset.rs:318`), which locks both worlds.
- The CPU copy stays resident: about 74 MB for ground alone (derived), plus about 4.8 KB per tree.

**Fix (S–M).** Use `RENDER_WORLD` only, so Bevy moves the data instead of cloning it (`render_asset.rs:303-307`). This requires:
- (a) storing each mesh's byte size in `Entry`, because reclaim currently reads it from the mesh (`src/terrain.rs:448-449`);
- (b) computing the `Aabb` on the worker and inserting it at spawn, since `calculate_bounds` reads the CPU mesh;
- (c) caching vertex counts for the profiling forest survey (`src/performance.rs:262-263`);
- (d) adjusting tests that inspect mesh attributes after upload.

**Verify.** RSS and peak memory over a long route; Extract duration.

### T5 — Serial, non-overlapped build pipeline (LOD lag; a fidelity issue, not frame time)

**Source.**
- `async_compute` is fixed at 1 thread (`src/main.rs:1123-1128`).
- A whole delta is built by one serial `.map(build)` (`src/terrain.rs:612`).
- A new job starts only after pending, staging, readiness and reclaim all finish (`:540-594`). The worker idles while staging and reclaim trickle at 4 meshes per frame.
- There is no per-tile cancellation.
- At high ground speed the 512 m ring lags the aircraft. Coverage stays complete, but coarse tiles (32 km roots have 512 m vertex spacing) and forest-less tiles can sit under the aircraft, so the rendered surface diverges from the analytic `terrain_height` used by physics.

**Fix (M).**
- Allow the next CPU job to run while the current layout is staging or reclaiming.
- Split jobs into per-tile tasks so stale ones can be dropped, ordered by distance.
- Optionally add a second async thread, together with T11.

**Verify.** Log job-to-commit latency and distance to the nearest non-512 m tile on a 250 m/s low pass.

### T6 — No LOD hysteresis or retired-tile cache (spikes during turning fights)

**Source.**
- The split test `dist < size*0.85` has no hysteresis (`src/terrain.rs:130`).
- Retired tiles are reclaimed immediately (`:589`, `:451-454`).
- Circling dogfights cross the same boundaries again and again, paying T2, T3 and T5 each time.

**Fix.**
- **(a, S)** Hysteresis in `leaves()` against the previous layout: split below 0.85·s, stay split until about 0.95·s. Decisions are per quadtree node, so the layout stays a valid partition, and detail is only ever kept longer.
- **(b, M)** Optionally keep a byte-capped LRU of hidden, GPU-resident retired tiles that a commit can re-show with a visibility flip.

**Verify.** Commits per minute on a scripted circling route.

### T7 — Several archetype moves per staged forest entity (small spike contributor)

**Source.** Spawn with `WorldObject` → `insert(ForestBatch)` → `remove::<WorldObject>` + `insert(Transform)` + `add_child` → later `Aabb` from `calculate_bounds` (`src/terrain.rs:394-417`).

**Fix (S).** Spawn the final bundle once: `ChildOf(root)`, `ForestBatch`, `Transform::IDENTITY` and a precomputed `Aabb`. The result is identical.

### T8 — Hidden staged and retiring entities still get transform writes (minor sustained)

**Source.** `src/world.rs:802`, `:831`: no visibility filter.

**Fix (S).** Skip `Visibility::Hidden` *terrain* roots only. Commit already writes the correct transform (`src/terrain.rs:579-580`). Other `WorldObject` users that toggle visibility (`src/earth.rs:78`, `:127`; `src/earth_region.rs:65`) must keep updating. T1 makes this fix unnecessary.

### T9 — Staging budget is not frame-time aware (spike risk)

**Source.** `src/terrain.rs:211-221`: 2 MiB of source bytes or 4 meshes per frame, whatever the frame headroom. It ignores the Extract clone (T4), slab growth (T3) and per-entity render work (T2).

**Fix (S).**
- Skip or halve staging on the frame after a slow frame, or on a commit or rebase frame.
- Log the per-frame maximum frame time next to staged bytes (telemetry currently records only bytes and meshes, `src/performance.rs:49-57`, `:118-123`).

### T10 — Synchronous start-up layout (start-up only)

**Source.** `src/terrain.rs:512-515` (called from `src/world.rs:247`) builds all 231 tiles plus forest serially on the main thread. Everything is then uploaded in one frame.

**Fix (S).** Build in parallel with a `ComputeTaskPool` scope (there is no frame to protect yet), and pre-size slabs (T3).

**Note.** In-flight teleports and resets use the async path: no stall, but a longer coarse-LOD period (T5).

### T11 — Thread oversubscription on a 4P+4E chip (hypothesis)

**Source.** `src/performance.rs:147-153` and `src/main.rs:1109-1128`. That is 6 compute threads plus 1 io, 1 async, the main thread and the render thread on 8 cores, so a busy terrain worker competes with the critical path.

**Fix (S).** Use the existing `on_thread_spawn` hook to give the async worker utility QoS (`pthread_set_qos_class_self_np`). Pair with T5, since builds get slower.

**Verify.** Frame-time variance while builds run versus idle.

**Not found / non-issues:**
- No O(n²) per-frame terrain loops.
- Opacity and alpha-mode switches only happen around 18 km and above.
- `src/warp.rs` is time warp, not origin rebasing. It caps the Gripen at 2× above 100 m AGL (`:113-138`), which halves the time between cell crossings.

---

## 7. Combat and low-altitude CPU findings

### C1 — Terrain LOS runs before the cheap gates (sustained; large in BVR, small in dogfight)

**LOS cost.**
- `guidance::terrain_visible_uncached` (`src/combat/guidance.rs:228-241`) samples 8–96 points along the chord, each calling `terrain_height` (`src/physics.rs:580-615`): about 11 sin/cos, an exp and a lake lookup.
- The 256-entry cache is exact. Positions change every substep, so it only removes repeats *within* a substep.

**Ordering problems.**
- **`ai::visible`** (`src/combat/ai.rs:36-38`) runs LOS first, then the cheap gates at `:40-63` (alive, visual range < 9 km, cone, sweep, elevation, range < 170 km).
  - In BVR it runs for every candidate pair every 1/120 s substep (`src/combat/bvr.rs:542-596`).
  - Derived count: an 8-vs-4 formation needs 56 checks per substep, 112 per frame at 2 substeps, each with 40–60 samples at BVR range.
- **`ai::warning`** (`ai.rs:76-90`): LOS at `:77` before the RF and optical range gates at `:82-89`. Called per target × player missile (`src/combat/dogfight.rs:148`, `:445`). In BVR it is also called before the cheap `friendly` and distance filters (`bvr.rs:626-639`).
- **Missile seeker** (`guidance.rs:541`): `quality()`, which includes LOS, is computed even when the gimbal or range checks at `:544-553` will zero the signal.
- **RWR** (`src/combat/rwr.rs:78`, `:114`): LOS before the azimuth, elevation, sweep and gimbal gates. It runs at 10 Hz and only in the cockpit view, so this is low priority.

**Fix (S, exact).** Evaluate every non-LOS term first. Return false when no gate can pass, and only then run LOS. Because LOS is pure and appears only as an AND factor, decisions, diagnostic reasons and event order are unchanged.
- For the seeker, set the same `(0, reason)` that the gimbal or range branch would set.

**Follow-up (M, exact):** precompute the BVR visibility matrix (≤ 12×12) in parallel. Visibility depends only on state that stays constant during the sequential scoring loop.

**Verify.**
- A property test comparing old and new `visible`, `warning` and seeker output over random states (extend `visibility_cache_tests`).
- The BVR serial/parallel equality test.
- Seeded `ai::study`, `bvr::study` and `guidance::study` runs with identical results.

### C2 — Particle pool: 256 unique materials mutated every frame (sustained during and after kills; spikes on mass emission)

**Source.**
- `src/combat/effects.rs:557-573`: 256 entities, each with its own `mats.add(StandardMaterial{AlphaMode::Blend…})` and `NoFrustumCulling`.
- `:905-935`: `mats.get_mut` for every live particle every frame, rewriting `unlit`, `base_color` and `emissive`.
- **Unstable slots:** `particles.remove(0)` at the cap (`:40-42`) and `retain` (`:164`) shift indices, so the entity bound to slot i changes kind and mesh handle often.

**Consequences.**
- Every live particle triggers re-preparation.
- Unique materials prevent batching, so every particle is its own transparent draw.
- There is no culling at all.

**Fix (M).**
- **(a)** A fixed set of shared materials per kind × quantized fade (16–32 levels), assigned by handle swap. This is visual-only, with slight alpha banding, so check it; or use one custom instanced material reading per-instance colour through `MeshTag` or a storage buffer, which gives identical colours.
- **(b)** A stable slot pool (array plus free list or generation ring). It keeps the "oldest first" eviction semantics.
- **(c)** Remove `NoFrustumCulling`, or give particles correct `Aabb`s. The authoritative particle simulation continues regardless.

**Verify.** The `material_modifications` counter in the `effects` soak (`src/benchmark.rs:255-257`), plus a frame-time trace over a scripted multi-kill.

### C3 — Text re-assigned every frame (sustained)

**Source.**
- DEFENSE panel: `src/combat/dogfight.rs:1347-1366`.
- Missile telemetry: `src/combat.rs:1846-1871`, plus `text.0.clear()` every frame while hidden (`:1873`).
- Target labels: `:1939-1944`.
- Combat readout: `:1971-1997`, `:1934`.
- `bvr/ui.rs:483`.

Each assignment marks the text changed, forcing cosmic-text re-shaping and re-layout.

**Existing good pattern.** `src/combat/hud.rs:536-538`, `src/combat.rs:789` and `src/main.rs:1768-1770` compare first, then `clone_from`.

**Fix (S).** Format into a reused `Local<String>`, assign only if it differs, and clear only if non-empty. Optionally rate-limit to 10–20 Hz, as `src/cockpit.rs:1174` does.

### C4 — Impact flashes rebuilt every frame (spike, 4 s after impacts)

**Source.** `src/combat.rs:1766-1803`. Each frame, `meshes.get_mut` collects a new colour `Vec` over all of a 693-vertex sphere (`:1313`), then re-uploads it; the material is mutated too (`:1790`).

**Fix (S–M).** The glow is only a function of normal·eye, so use a shared unlit fresnel shader and drive fade and heat through a uniform or quantized material set.

### C5 — Missile smoke (sustained with live missiles)

**Source.**
- CPU path (the default): `src/combat.rs:2028-2099`. The `TrailState` key includes the camera pose (`:2064-2071`), so the early-out at `:2073` only triggers when paused.
- The mesh is rewritten through `get_mut` each frame (`write_smoke_mesh` `:2102-2192`; `aim9x::write_smoke` `src/combat/aim9x.rs:497-577`). That is up to 48 points, about 940–1,150 vertices per trail.
- Vec buffers are already reused, and indices are rebuilt only on count change.

**GPU path** (`src/combat/smoke_gpu.rs:39-43`): opt-in. It rebuilds the mesh only when the trail key changes, but it still calls `materials.get_mut` per trail per frame to update `time`/`head` (`:149-154`).

**Fix (M).**
- **(a)** In the CPU path, build in anchor-relative space and move the anchor through `Transform`, as `smoke_gpu.rs:138-140` already does. The vertex buffer then changes only when a trail point is added (every 0.075 s).
- **(b)** In the GPU path, use one shared material with a single sim-clock uniform, and pass the head through `Transform`, `MeshTag` or a storage buffer.
- **(c)** Make the GPU path the default only after a pixel-parity check, especially for the camera-facing AIM-9X cards.

### C6 — Asset-writer systems serialize Presentation (sustained)

**Source.** Presentation systems holding `ResMut<Assets<StandardMaterial>>` (`src/main.rs:1241-1280`):
- `skin_damage::update`;
- `aircraft_detail::update`;
- `raptor_detail::update`;
- `combat::render`;
- `aero_fx::update`;
- `cockpit::update`;
- `effects::render`;
- `bvr::cockpit_radar::visuals`.

Mesh writers: `combat::render`, `combat::smoke` and `dogfight::draw`. Image writers: `cockpit::update` and `effects::render`.

The new `raptor_bay::update` and `raptor_main_bay::update` were added to the chained group in the uncommitted `main.rs`.

**Fix (M).** First cut the per-frame mutations (R5, C2, C4, C5) so these systems are short. Then split the read-only parts (transforms, visibility) into separate systems that can run in parallel.

**Verify.** Add `performance::span` coverage. There are currently no spans on `effects::render`, `combat::render`/`smoke`, `hud::draw`, `dogfight::draw`, telemetry or `aero_fx` (`src/performance.rs:13-31`).

### C7 — HUD drawn with 540 UI nodes (sustained, HUD visible)

**Source.** `src/combat/hud.rs:209-221`, `:506-526`. Every used stroke writes `Node` geometry, `UiTransform` and `BackgroundColor` every frame, which drives Taffy layout and UI extraction.

**Fix.**
- **(S)** Assign only if the value differs. This helps for strokes that stay put, such as the reticle and rings.
- **(M)** Render HUD strokes as one dynamic mesh or UI material.

### Very-low-altitude physics

- **P1 (S, exact) — repeated `terrain_height`.** It is evaluated at the same (x, z) several times per aircraft per substep: `src/physics.rs:693`, `:912`, `:942`, `:945`, `:1010`; `src/collision.rs:264-265`; `src/terrain.rs:78`; `src/combat/ai.rs:174-178`. Use a one-entry exact-bits memo or compute it once per step.
- **P2 (M, exact if proven) — conservative terrain upper bound.**
  - `terrain_height` has an analytic maximum: hills ≤ 80, ridge ≤ 230, detail ≤ 1.7, mountains ≤ theater(x)·3050, runway 55.17, meadow 62.
  - Skip `terrain_height` samples (or the whole LOS) when the chord's lowest point is above the bound. Apply the same bound to round, missile and fragment ground tests (`src/combat/dogfight.rs:705-710`, `src/combat/effects.rs:155-161`, `src/combat.rs:726-728`, `src/combat/bvr.rs:955`).
  - Needs a randomized or exhaustive test that `terrain_height ≤ bound` with floating-point slack.
  - Helps above about 320 m AGL, not nap-of-earth.
- **P3 (M, exact) — near-ground hull check.** Below 12 m AGL, `src/collision.rs:339-353` loops every hull × sample × vertex through `terrain_height`, and `above_flat_ground` (`:370-391`) only short-cuts on the runway. Add a per-hull reject using `terrain_height(centre)` + maximum gradient × horizontal extent. This affects taxi, landing, crash and NOE flight.
  - Already well handled: vegetation queries are gated to ≤ 30 m AGL with grid cells and scratch reuse; rounds have a swept-sphere broad phase; suspension is 3 wheels; gear visuals are change-gated.
- **P4 (S) — small allocations.**
  - `Combat::threats` collects a `Vec` and is called twice per frame (`src/combat/dogfight.rs:785-792`).
  - `hud::funnel` returns a `Vec` (`hud.rs:93-117`).
  - `rwr::emissions` returns a `Vec` (`rwr.rs:65-128`).
  - The dogfight `neighbours` Vec is collected and cloned per target per substep (`dogfight.rs:122-126`, `:212`).

  Reuse buffers, as BVR's `WorkBuffers` already does.
- **AI parallelism.** Only BVR integration is parallel (≥ 8 targets). Sensing, decisions and missiles are serial. Dogfight runs 3 targets serially, and parallelizing that is unlikely to pay. Do not change it without measuring.
- **Render LOD geometry must never replace the analytic collision terrain.** No such use was found.

---

## 8. Simulation timing and perceived lag

### S1 — The simulation advances a fixed amount per rendered frame (perceived lag)

**Source.**
- `src/main.rs:2012-2060`: `step` takes `Res<Time>` but never reads it. `clock.accumulator = warp.timestep * warp.effective;` (`:2047`) *overwrites* the accumulator each frame, then it is consumed in substeps of at most 1/120 s.
- `src/warp.rs:33`: default `timestep = 1/60`; user-editable and persisted (`:91-95`, `:299-304`). In combat, warp is capped at 2× (`:131-135`).
- Tests lock this in (`src/main.rs:2849-2853`, `:3066-3072`).

**Consequences.**
- Wall-clock simulation speed = FPS × timestep. At 45 FPS that is 0.75×, at 30 FPS 0.5×.
- A hitch freezes the simulation for its duration with no catch-up. This is **not** a catch-up spiral; it is slow motion.
- With `AutoVsync`, a 120 or 144 Hz external display runs the simulation at 2× or 2.4× real time.
- **Mixed clocks:** throttle input ramps with real `time.delta_secs` (`src/main.rs:1796`, ~`1942-1945`) and `aero_fx` uses real elapsed time (`src/aero_fx.rs:136-140`), while flight runs on frame time. At low FPS the controls feel twitchier relative to the aircraft response.
- **Determinism is excellent today:** fixed step, no wall-clock input.

**Optional behavioural change (M, needs an explicit decision):** real-time fixed step with interpolated presentation.
- `acc += min(real_dt, 0.25) * warp`, then `while acc >= h { step(h); acc -= h }` with the same h = 1/120 and the same physics.
- Render with alpha = acc/h between the previous and current snapshots of the player, targets, missiles and particles.
- **Overload:** cap substeps per frame. When the cap is hit, clamp `acc`, so time dilates *only* under overload instead of spiralling. Never coarsen or skip physics.
- **Determinism:** each step is identical. Replay determinism needs inputs logged per sim step rather than per frame.
- **Costs and risks:**
  - up to one substep (8.3 ms) of extra visual latency;
  - every pose reader must use the interpolated pose, or jitter appears;
  - event ordering within a frame is unchanged, but the number of steps per frame varies;
  - existing tests must run under a frame-locked flag.
- **Keep the current mode** as a flag for tests, benchmarks and captures.
- **Verify.** `simulated_seconds()` versus wall time (`src/performance.rs:58-65`) should be about 1.0 below the cap; check pose continuity in slow-motion captures.

This change fixes *perceived* lag and the refresh-rate dependence. It does not make frames faster, and under overload it adds simulation work per frame. Do it only alongside the frame-time fixes above.

---

## 9. Engine comparison

### 9.1 How tied the code is to Bevy

These estimates come from grep-level inspection.
- **Largely engine-independent: about 12–15k lines.**
  - `src/physics.rs`: `simulate(f: &mut Flight, e: &Environment, dt: f64)` at `:617`. `Flight` and `Environment` are plain structs whose only Bevy link is `#[derive(Resource)]`.
  - Aerodynamics, propulsion, airframe, `combat/guidance.rs` and `combat/ai.rs` use only `bevy::math` (glam f64 types).
  - `combat::Combat` has plain methods.
  - `warp::coast`.
- **Bevy-bound and rewritten in any other engine: about 35k lines.**
  - 37 files use `Query<`.
  - 13 files build procedural meshes with `Mesh::new`.
  - About 293 `StandardMaterial` references.
  - Custom render code: `earth_gpu`, `display_upload`, `atmospheric_render`, `smoke_gpu` and the F-22 visual modules.
  - The cockpit, HUD and instruments (Bevy UI and render-to-texture), terrain streaming and the patched `bevy_pbr`.
- **Known couplings to untangle:** `Combat.audio_events` uses `crate::sound::Event`, and the `effects::State` presentation state sits inside `Combat`. Hidden `Time`, input or UI reads inside simulation code have not been fully audited.

### 9.2 Comparison

All official-doc claims were checked on 2026-10-23. Some licensing pages were not fully fetchable; see 9.4.

| | **Bevy (stay, upgrade to 0.20)** | **Unity 6 URP** | **Unity 6 HDRP** | **Unreal Engine 5.8** | **Godot 4.x + godot-rust (gdext)** | **Fyrox 1.0** | **Custom wgpu/Metal renderer** |
|---|---|---|---|---|---|---|---|
| M1 / 8 GB near-ground potential | Current baseline. GPU-driven features improved in 0.19 (≈15% on Bistro, M2 Max); Metal multi-draw/GPU culling status unverified | Good. Metal first-class; 8 GB is the editor minimum | Fair. Metal macOS-only, compute required, maintenance mode | **Poor.** 16 GB Mac minimum (32 GB recommended); Nanite and VSM need M2+ (Beta); Lumen software RT works on M1 | Fair–good. Native Metal backend (since 4.4), default on Apple Silicon | Poor. OpenGL 3.3 path on macOS | Whatever you build |
| Terrain / vegetation | DIY (already built) | Unity Terrain + DIY spherical streaming | Same | Landscape, PCG, Nanite foliage (M2+) | Terrain3D (≤ 65 km, MIT), so planetary terrain is DIY | Basic | DIY |
| Lighting / atmosphere | PBR, atmosphere, SSR, contact shadows; Solari with no Metal denoiser | Physical sky / real-time GI on the roadmap | Full feature set, frozen | Lumen, VSM (M2+) | Forward+, area lights (4.7) | PBR, no GI | DIY |
| Profiling | Tracy (`trace_tracy`), Perfetto, Xcode | Unity Profiler, Frame Debugger, Xcode | Same | Unreal Insights, Xcode | Built-in profiler, Xcode | Basic | Tracy, Xcode |
| Editor / assets | None (BSN groundwork, Feathers widgets) | Mature | Mature | Mature, heavy | Mature, light | Exists, small team | None |
| Double precision / large world | Native f64 sim + custom camera-relative render (kept as is) | **No camera-relative rendering in URP**: DIY floating origin | Camera-relative rendering built in | Large World Coordinates (f64; documented as Beta) | Requires **self-built** `precision=double` editor + templates; gdext custom-API builds unsupported | Not verified | DIY |
| Rust interop | Native | C ABI native plug-in (`DllImport`) | Same | C ABI into a C++ module | gdext (Rust-native extension) | Native | Native |
| Migration effort | 0.18 → 0.20: render graph → ECS schedules (re-port the `bevy_pbr` patches); WGSL → WESL | Rewrite ~35k lines in C#, plus C ABI | Same, into a dead end | Largest (C++/Blueprint) | Large rewrite (Rust via gdext), plus a custom double build | Large | Very large |
| Windows / macOS | Both | Both | Both (Metal macOS only) | Both (Mac needs M2+ for Nanite) | Both | Both, weak Mac | Both |
| Licensing | MIT / Apache-2.0 | Personal free ≤ $200k; Pro ≈ $2,310/seat/yr; Runtime Fee cancelled | Same | 5% royalty above $1M lifetime gross per product (games); seat licensing for non-game use | MIT | MIT | Yours |
| Maintenance | Breaking release every ~4–6 months | Yearly versions, pipeline churn | Frozen | Heavy; UE6 coming | Moderate; double builds DIY | Small-team risk | Highest |

### 9.3 Interpretation

- **No engine removes this report's top findings for free.** Pass count at fullscreen, cascade sizing, publication granularity, the moving origin, per-frame asset mutation and the frame-locked simulation are all project-level design choices. Each candidate engine has its own version of every one, and would still need its own floating-origin scheme and spherical terrain streamer.
- **What an engine *would* add:** a mature editor, artist-facing terrain, vegetation and VFX tools, and better first-party profilers. These are real productivity gains, not proven FPS gains.
- **Unreal** is the strongest renderer, but on this machine it is below Epic's own minimum and loses Nanite and VSM. It is ruled out until the dev machine is at least an M2 with 16 GB or more.
- **Unity HDRP** is in maintenance mode: no new features. Unity is consolidating on URP, and HDRP is not a sensible new target.
- **Unity URP** is the most practical *alternative*. Costs: no camera-relative rendering (build a floating origin), a C# rewrite of all presentation code, and a C ABI to the Rust simulation.
- **Godot + gdext** keeps more Rust. However, double precision requires a custom engine build, and gdext's custom-API path has no compatibility guarantee, so long-term maintenance risk is real.
- **Fyrox** (OpenGL on macOS) and a **custom wgpu renderer** (all of Bevy's PBR, shadows, UI and atmosphere to re-own) are not recommended. Forking or patching Bevy's renderer, as the project already does, is cheaper.

### 9.4 Sources and unverified items

- Unity native plug-ins: https://docs.unity3d.com/6000.0/Documentation/Manual/plug-ins-native.html
- Unity system requirements: https://docs.unity3d.com/Manual/system-requirements.html
- Metal requirements: https://docs.unity3d.com/Manual/metal-requirements-and-compatibility.html
- HDRP system requirements: https://docs.unity3d.com/Packages/com.unity.render-pipelines.high-definition@17.0/manual/System-Requirements.html
- Unity render-pipeline strategy for 2026 (HDRP maintenance): https://discussions.unity.com/t/render-pipelines-strategy-for-2026/1710004
- Unity pricing updates: https://unity.com/products/pricing-updates
- Unreal macOS requirements: https://dev.epicgames.com/documentation/en-us/unreal-engine/macos-development-requirements-for-unreal-engine
- Unreal hardware specs: https://dev.epicgames.com/documentation/en-us/unreal-engine/hardware-and-software-specifications-for-unreal-engine
- Unreal Large World Coordinates: https://dev.epicgames.com/documentation/en-us/unreal-engine/large-world-coordinates-in-unreal-engine-5-5
- Unreal license: https://www.unrealengine.com/license
- Godot large world coordinates: https://docs.godotengine.org/en/stable/tutorials/physics/large_world_coordinates.html
- Godot renderers: https://docs.godotengine.org/en/stable/tutorials/rendering/renderers.html
- Godot 4.4 release (Metal): https://godotengine.org/releases/4.4/
- godot-rust: https://github.com/godot-rust/gdext and https://godot-rust.github.io/book/toolchain/godot-version.html
- Bevy news (0.19, 0.20): https://bevy.org/news/
- Bevy profiling: https://github.com/bevyengine/bevy/blob/main/docs/profiling.md
- Fyrox: https://fyrox.rs/blog/ and https://fyrox.rs/download.html

**Not verified:**
- Unreal license and FAQ text (HTTP 403; figures are from search snippets of the official page).
- Whether Bevy 0.19/0.20 enable multi-draw indirect and GPU culling on Metal.
- Whether the M1 exposes the 64-bit texture atomics that Bevy meshlets need.
- HDRP ray tracing on Metal.
- Any built-in Unity 6 floating origin.
- gdext's official double-precision feature documentation.
- Terrain3D on Metal.

---

## 10. Recommendation and limited comparison prototype

### Recommendation

1. **Stay on Bevy.** Do the frame-time work in this report in this order:
   - the cheap exact items (R2a, C1, C3, T3, T7, R8/T8 UV drop, R5 star check, T6a hysteresis);
   - R1 (fused post);
   - R3 (shadow proxies);
   - T1 (quantized render anchor; also unblocks R2b and static shadow reuse);
   - T2 (swap groups);
   - the combat churn (C2, C4, C5, C6).

   Decide S1 (the real-time simulation contract) as a separate product decision.
2. **Plan the Bevy 0.18 → 0.20 upgrade deliberately.** The render graph became ECS schedules in 0.19, so re-port the `vendor/bevy_pbr` atmosphere and light-probe patches. WGSL becomes WESL in 0.20. Avoid deepening the vendor patch more than R1 and R2 require until the upgrade is planned.
3. **Pull the simulation out into a `nord_sim` crate regardless of engine.** Estimated effort: 3–6 weeks, from grep-level inspection only.
   - Depend on `glam` directly and put `derive(Resource)` behind a feature.
   - Move the `step`/warp/coast orchestration in.
   - Expose a coarse C ABI, crossing the boundary in batches per frame, never per entity:
     - `#[repr(C)]` commands (stick, throttle, weapons, warp);
     - `nord_step(ctx, dt)` with the fixed 1/120 substep;
     - batched `#[repr(C)]` snapshots of ownship f64 pose and controls, targets, missiles, smoke emitters, impacts, audio events and HUD/MFD scalars;
     - headers generated with cbindgen.
   - The f64 state stays authoritative, and conversion to camera-relative f32 happens at the presentation boundary.
   - This pays off even without a migration: headless tests and benchmarks, and insulation from Bevy upgrades.

### Limited prototype (only if a switch is still being considered; about 2 weeks per engine; not built in this review)

Run one alternative (Unity 6 URP first, then Godot 4.x) against a **Bevy 0.20 baseline**, on this M1 Air.

**Matched content:**
- **Terrain:** a 16×16 km tile baked from `physics::terrain_height` (runway and lake included), with forest at the same density and tree geometry.
- **Lighting:** one sun, 2 cascades out to 2.5 km at 1024, plus atmosphere or sky.
- **Cockpit:** the Gripen cockpit mesh, 3 MFDs as render-to-texture at the current update rate, and the HUD.
- **Effects:** 8 missiles with smoke trails, 2 explosions with the 256-particle cap, and afterburner plumes.
- **Simulation:** `nord_sim` through the C ABI at 120 Hz with 1 ownship, 8 AI aircraft and 16 missiles; snapshots consumed every frame.
- **Precision:** fly 50 m AGL at 250 m/s, then repeat 1,000 km from the origin. Check jitter, z-fighting and shadow swimming.
- **Resolution:** 2880×1800 fullscreen and 1440×900 windowed, default-equivalent quality.

**Metrics:**
- CPU and GPU frame time p50/p95/p99 and long-frame counts, uninstrumented runs kept separate from profiler captures.
- Peak memory (game and editor separately).
- Sustained performance after 15 minutes (the Air is fanless).
- Shader-compile hitches; build and iteration time; days to reach parity.

**Go / no-go:** switch only if the alternative clearly beats the Bevy 0.20 baseline at p95/p99 under identical content and conditions, or if editor productivity clearly outweighs the rewrite, with no unresolved precision artefacts.

---

## 11. Validation plan for future implementation

**Scenarios:**
1. **Stationary forest:** paused, 50 m AGL, cockpit and external.
2. **Moving terrain-cell crossings:** the `terrain-stream` script at 100 and 250 m/s, 50 m AGL.
3. **Circling low-altitude dogfight:** a new scene at 150–300 m AGL over forest, with guns, missiles and `diagnostic_damage`. Nothing like it exists today; add it to `src/benchmark.rs`.
4. **Larger formations:** BVR 8v4 and 11-target cases.
5. **Simultaneous missile trails:** 8 or more live trails, chase camera close behind.
6. **Near-ground impacts and crashes:** debris, fragments, flashes.
7. **Camera transitions:** cockpit ↔ external ↔ map, and F9 quality toggling.

**Run each scenario in** the Gripen and the F-22, default preset primary, high quality separately.

**Recording requirements for every comparison:**
- **Machine state:** actual pixel resolution, preset, power state (AC or battery, and charge), background load sampled *during* the run, and binary hash.
- **Frame statistics:** p50/p95/p99, frames over 16.7, 25 and 50 ms, mean FPS, and sample length. Run at least 3 alternating repetitions.
- **Run types:**
  - FPS-only runs (`NORD_BENCH_FPS_ONLY=1`), kept separate from instrumented and Metal runs;
  - no-vsync runs (`NORD_PRESENT_MODE=no-vsync`) to see gains below the vblank step;
  - VSync runs for the shipping experience.
- **Counters:** commits per frame, entities flipped, slab size and count, `material_modifications`, `mesh_modifications`, and the ratio of `simulated_seconds` to wall time.

**Correctness gates:**
- **Terrain:** no holes; first-visible-frame transform tests pass (`src/terrain.rs:757-871`).
- **Glass and HUD:** screenshot diffs within agreed tolerance; zero diff where "identical" is claimed.
- **Shadows:** readback diff of zero for proxy work.
- **Effects:** visual parity at launch, impact and kill.
- **Simulation:** seeded `ai`/`bvr`/`guidance` studies, collision outcomes and event order byte-identical for every "exact" change.

**Example direct run** (from the opt8 harness):

```
NORD_BENCH_SCENE=terrain-forest NORD_BENCH_SCRIPT=terrain-stream NORD_BENCH_LIVE=0 \
NORD_TERRAIN_AGL=50 NORD_TERRAIN_SPEED=100 NORD_BENCH_WARMUP=10 NORD_BENCH_SECONDS=25 \
NORD_BENCH_FPS_ONLY=1 NORD_BENCH_FULLSCREEN=1 NORD_TERRAIN_COCKPIT=1 \
NORD_BENCH_REPORT=out.json "Nord Flight.app/Contents/MacOS/gripen-flight" --benchmark-render cockpit-x
```

Harnesses: `python3 tests/optimization8/validate.py {matrix|repeat|…}` and `python3 tests/optimization8/trace.py` (needs a fresh output folder).

---

## 12. Implementation status (2026-10-23, follow-up)

| Item | Status | Where |
|---|---|---|
| R5 star material | Done: material written only when alpha mode or colour differ; visibility via `set_if_neq` | `src/world.rs` `update_stars` |
| C3 text churn | Done: DEFENSE panel, missile telemetry, target labels, combat readout and BVR map assign text only when it changes; clears and `display` writes guarded | `src/combat/dogfight.rs`, `src/combat.rs`, `src/combat/bvr/ui.rs` |
| C1 LOS ordering | Done: terrain LOS after cheap gates in `ai::visible`, `ai::warning`, the missile seeker (gimbal/range), RWR emitters and missiles, and the BVR incoming-missile filter. Decisions and reasons unchanged | `src/combat/ai.rs`, `src/combat/guidance.rs`, `src/combat/rwr.rs`, `src/combat/bvr.rs` |
| T7 forest spawn | Done: forest batches spawn directly as identity `ChildOf` tile-root children (reference path kept under `NORD_TERRAIN_TRANSFORM_REFERENCE`) | `src/terrain.rs` `spawn_forest` |
| R8/T8 forest UV | Done: `UV_0` removed from forest meshes and batches | `src/terrain.rs` |
| T6a hysteresis | Done: split at 0.85·size, stay split until 0.95·size, against the published layout; new partition/refinement test | `src/terrain.rs` `leaves`/`layout_from` |
| R2a mirror mip | Done: roughness-0 mip dispatched as dense 8×8 tiles in the parallel kernel; blue-noise fetch moved after the mirror branch | `vendor/bevy_pbr/src/light_probe/generate.rs`, `environment_filter.wgsl` |
| T3 slab pre-sizing | **Measurement only.** `min_slab_size` applies to every vertex layout, so it must be sized from data. Added whole-run `mesh_slab_peak {bytes, slabs}` to the benchmark report. A 10 s windowed terrain-forest smoke run reported 337,299,320 bytes over 8 slabs; size from a full low-altitude route before setting it | `src/performance.rs` |
| S1 real-time sim | Not implemented (by request) | — |

**Verification:**
- `cargo test --release`: 441 passed.
- `cargo test --release -p bevy_pbr`: passed.
- A short release benchmark launch showed no shader or validation errors.

**Not yet verified:**
- Pixel equality of the radiance mirror mip.
- Any FPS effect of these changes.
