# Nord Flight — static optimization review

**Date:** October 20, 2026  
**Reviewed source:** current working tree in `/Users/Austin/Downloads/GripenFlight`, based on commit `f4d19b6`, including the uncommitted AIM-9X/AIM-120D changes.  
**Method:** source inspection only. No application launch, diagnostic run, test, build, benchmark, profiler or runtime capture was performed for this review. The only file created is this report. Existing implementation and validation documents were read for context; their old results are not presented as current measurements.

## Assessment

There are still useful optimization opportunities. The most promising source-level targets are **presentation work and data movement**, followed by **terrain publication and repeated simulation bookkeeping**. The code already contains substantial optimization work; the next pass should refine specific remaining paths rather than repeat a general “add more threads” effort.

This review covers the scheduling and presentation paths, aircraft/missile geometry, cockpit displays and uploads, combat/AI, collision/terrain, Earth/atmosphere, orbital map, menu, audio and packaging. It is a targeted static review of these paths, not a claim that every line or engine dependency has been exhaustively audited.

**Priority means investigation/implementation order, not measured milliseconds.** The operations described below are visible in source; their contribution to frame time, GPU time, allocations or battery use remains unmeasured in this review. No defensible FPS improvement estimate follows from source alone.

## Ranked findings

| Rank | Candidate | Most affected workload | Confidence in redundant/expensive work | Change risk |
| --- | --- | --- | --- | --- |
| 1 | CPU rebuilding and republishing detailed missile smoke | Multiple missiles; moving cameras | High | Medium–high |
| 2 | Coarse cockpit invalidation and full CPU display redraw | Live cockpit, especially F-22 | High | Medium |
| 3 | Broad scheduling dependencies and asset-resource conflicts | CPU-limited frames | High that constraints exist; benefit unknown | High |
| 4 | Fine-grained scene entities and duplicated mesh assets | Cockpit/external submission; startup memory | High | Medium |
| 5 | Terrain completion publishes an entire replacement batch at once | Low-level flight and scene transitions | High | High |
| 6 | Unchanged component/asset writes in presentation | Paused and steady views | High | Low–medium |
| 7 | Repeated assignment scans inside BVR target comparisons | Largest BVR scenario, high step counts | High | Medium |
| 8 | Temporary vectors and heavyweight collision snapshots | Simulation throughput near terrain/combat | High | Low–medium |
| 9 | Camera-dependent environment work remains intrinsically active | Atmospheric camera movement | High dependency confidence; optimization uncertain | High |
| 10 | Repeated map prediction and orbit-line construction | Orbital map, including paused map | High | Medium |
| 11 | Broad/exclusive start-menu presentation traversal | Animated start screen; small closed-menu overhead | High | Medium |
| 12 | Constant voice pan math repeated at audio sample rate | Many audio effects | High | Low–medium |

### 1. Detailed smoke still rebuilds mesh attributes on the CPU

**Evidence:** [smoke dispatch and cache](/Users/Austin/Downloads/GripenFlight/src/combat.rs:1847), [detailed smoke builder](/Users/Austin/Downloads/GripenFlight/src/combat/aim9x.rs:496).

The smoke cache includes missile age/position and, for detailed trails, camera position and rotation. Those normally change during flight or camera movement. A cache miss calls `write_smoke`, which reconstructs camera-facing positions, colors and UVs for every retained segment, scans the result for bounds, and reinserts four vertex attributes into an `Assets<Mesh>` asset. Index contents are reused when the size is unchanged, and the vectors are recovered/reused: this is **not** a fresh allocation of every vector every frame. It nevertheless repeats substantial geometry work and marks the mesh modified.

The code generates six quads per segment. At the current maximum of 48 trail segments with the powered endpoint, that is 1,152 vertices per detailed trail. Position/color/UV/normal arrays represent about 54 KiB of attribute data per trail. Across 64 simultaneous full trails, that is roughly 3.4 MiB of attributes rebuilt per update. These are **source-derived capacity calculations**, not observed populations or measured transfer bytes.

**Recommendation:** retain trail centers/ages and instance soft smoke cards; calculate camera-facing orientation, expansion and fading in a vertex shader. Keep the same texture, overlap, population, lifetime, drift and transparency. A smaller interim change is to separate trail-history changes from camera-facing changes, retain immutable UV/index data, and avoid rebuilding data unrelated to the changed input.

**Important constraint:** moving billboards to the GPU needs conservative CPU bounds and the existing camera-relative precision. Transparent ordering must remain correct. Do not reduce smoke count or lifetime as a shortcut.

### 2. Cockpit redraw invalidation is much broader than individual display changes

**Evidence:** [cockpit cadence/signature](/Users/Austin/Downloads/GripenFlight/src/cockpit.rs:1120), [full pixel clear and display routing](/Users/Austin/Downloads/GripenFlight/src/cockpit.rs:1155), [F-22 second radar page](/Users/Austin/Downloads/GripenFlight/src/cockpit.rs:1216), [upload comparison](/Users/Austin/Downloads/GripenFlight/src/display_upload.rs:28), [producer lock](/Users/Austin/Downloads/GripenFlight/src/display_upload.rs:141).

The cockpit already limits ordinary display work to a 0.10-second presentation cadence and skips non-cockpit views. However, one shared signature includes `f.elapsed`, so live simulation generally invalidates the whole active display group at every due refresh. Each applicable display is then fully cleared and repainted. Numeric pages also build vectors of formatted strings. The F-22 can draw a tactical radar page on screen kind 1 as well as draw the radar interface on screen kind 2; the HUD separately draws track information.

The persistent upload system is already an improvement: it compares pixels, skips identical output, uploads changed rectangles and reuses staging storage. But it discovers those changes **after** clearing/repainting the CPU image, and scans/copies while holding the producer mutex. Partial upload savings do not eliminate CPU rasterization cost.

**Recommendation:** use per-display state/revisions; preserve the current cadence and immediate control response. Cache static bezels/backgrounds and unchanged text. Build radar/contact projections once per display refresh and let both radar pages/HUD consume the shared result. Where practical, let the rasterizer report dirty regions directly rather than discovering all changes through a full-frame comparison.

**Guardrail:** retain the existing complete comparison path as a fallback for displays whose output changes everywhere. Do not replace correct rasterization with an incomplete signature that leaves stale instruments.

### 3. Scheduling still prevents potentially independent CPU work from overlapping

**Evidence:** [Update scheduling](/Users/Austin/Downloads/GripenFlight/src/main.rs:1111), [parallel AI integration](/Users/Austin/Downloads/GripenFlight/src/combat/bvr.rs:474).

The first Update group is broadly chained through controls, simulation, world updates, camera/Earth and audio. A second presentation group follows it. Systems inside the second group are not all explicitly chained, but several request mutable access to the same mesh/material asset resources, which prevents overlap even when they modify disjoint assets. The missile presentation path explicitly chains generic rendering, AIM-9X animation and radar-fin animation.

This does **not** mean the entire engine or simulation is single-threaded. BVR aircraft integration already partitions disjoint targets into worker tasks for formations of at least eight, and terrain generation already uses the async pool.

**Recommendation:** express actual dependencies with named sets: input → authoritative simulation → camera/origin → view-dependent presentation. Separate CPU calculation of desired transforms/display data from asset publication, so calculations can overlap and a compact commit stage performs mutations. Remove ordering edges only after establishing the required data dependencies.

**Guardrail:** preserve serial successive physics steps, stable weapon/damage/collision commits and scene-transition ordering. Adding threads without restructuring shared writes will not solve resource conflicts.

### 4. Small scene parts and identical meshes remain separate assets/entities

**Evidence:** [per-part mesh allocation helper](/Users/Austin/Downloads/GripenFlight/src/aircraft/visual.rs:567), [eager fleet setup](/Users/Austin/Downloads/GripenFlight/src/aircraft/visual.rs:655), [all cockpit interiors](/Users/Austin/Downloads/GripenFlight/src/aircraft/visual.rs:1116), [missile pool setup](/Users/Austin/Downloads/GripenFlight/src/combat.rs:1080), [duplicate radar geometry construction](/Users/Austin/Downloads/GripenFlight/src/combat/radar_visual.rs:141).

The aircraft helper adds a new mesh asset for each supplied mesh. Many physical details are separate parts; all four additional aircraft and their cockpit interiors are created at startup. Hidden models avoid ordinary visible rendering, but still occupy assets/entities and participate in relevant queries. Separately generated identical geometry also makes automatic instancing harder because asset identities differ.

Every missile slot installs both AIM-9X controls and radar controls. Across 64 slots, this creates 768 actuator entities plus 64 seeker-finish entities before counting missile roots, flames, decorations and smoke. Most remain hidden, but animation systems still visit their pools. The two radar body/tail variants now generate identical D-style geometry under different handles.

**Recommendation:** first share canonical primitives, repeated stores, identical radar meshes and compatible materials. Merge rigid parts sharing a material within the same moving/damage attachment group. Consider typed missile visual pools or activate only the relevant decoration family. Cache reusable prototypes for vehicle selection; optional lazy creation should prepare completely before switching.

**Guardrail:** do not merge across independently moving/detachable controls, clickable cockpit targets, incompatible transparency or shadow behavior. Some F-22 details already use batching; extend that selectively rather than assume none exists. Draw-call reduction is a candidate, not a measured count in this review.

### 5. Background terrain generation ends in a main-thread publication burst

**Evidence:** [terrain update and publication](/Users/Austin/Downloads/GripenFlight/src/terrain.rs:443), [async generation job](/Users/Austin/Downloads/GripenFlight/src/terrain.rs:471).

Terrain geometry is built asynchronously, but when the job finishes, the Update system retires the old layout and installs **every** completed replacement tile in one update. That can concentrate entity creation, mesh insertion, bounds work and subsequent render preparation in a single frame. The missing tiles are also built serially inside one async task; using an async pool does not automatically parallelize that iterator.

**Recommendation:** prepare pending tiles and mesh resources incrementally under a bounded budget; publish only complete replacement coverage, with current camera-relative transforms. If generation is significant, parallelize independent tile builds with bounded fan-out and stable collection.

**Guardrail:** retain complete old coverage until new resources are ready. This area previously required explicit transition-order fixes; a faster but incomplete publication would risk reintroducing screen artifacts. Static inspection cannot establish the current size or frequency of the publication hitch.

### 6. Several presentation loops still mark unchanged state modified

**Evidence:** [aircraft surface writes](/Users/Austin/Downloads/GripenFlight/src/aircraft/visual.rs:1390), [bay-door/scar writes](/Users/Austin/Downloads/GripenFlight/src/aircraft/visual.rs:1546), [missile transforms](/Users/Austin/Downloads/GripenFlight/src/combat.rs:1514), [impact color/material regeneration](/Users/Austin/Downloads/GripenFlight/src/combat.rs:1602).

Some paths use `set_if_neq`, but others assign rotations, visibility, translations and scales every rendered frame. The impact flash path also allocates a color vector and modifies its mesh/material whenever an impact exists, even if pause makes age/camera unchanged. Mutable component writes and asset access can trigger downstream work despite equal values.

**Recommendation:** finish the desired-value/equality pattern for stable components. Cache flash appearance by impact age and view direction; preferably compute view-facing glow in a shader. Skip presentation updates only when their complete input set is unchanged.

**Expected scope:** an especially sensible candidate for paused/static-view efficiency; it may improve less during continuously changing motion. Do not equate every change tick with a full GPU upload without checking the downstream path.

### 7. BVR assignment counts are repeatedly recomputed inside comparisons

**Evidence:** [candidate selection and assignment scoring](/Users/Austin/Downloads/GripenFlight/src/combat/bvr.rs:527).

For each aircraft, candidate filtering evaluates visibility, then `min_by` repeatedly scores pairs. Each score scans the AI list to count friendly aircraft assigned to that candidate. This can grow toward cubic work with formation size, in addition to visibility/sensor costs. The present formation limit is small, so this is a throughput/scaling candidate rather than proof of a major FPS problem.

**Recommendation:** maintain per-team assignment counts keyed by target and compute each candidate score once. Because memories are updated during the outer loop, update counts as each aircraft changes its assignment; a single frozen count table could change behavior. Preserve candidate ordering and tie handling.

### 8. Recurring simulation bookkeeping still allocates or clones more than needed

**Evidence:** [practice/close-combat snapshots](/Users/Austin/Downloads/GripenFlight/src/combat.rs:467), [vegetation vector construction](/Users/Austin/Downloads/GripenFlight/src/terrain.rs:65), [collision caller](/Users/Austin/Downloads/GripenFlight/src/collision.rs:257), [near-contact aircraft clones](/Users/Austin/Downloads/GripenFlight/src/combat/contact.rs:49).

`Combat::advance` creates fresh `alive_before` and `old_targets` vectors each active step. Near terrain, obstacle lookup constructs another vector and regenerates local tree bounds. Aircraft contacts pass the broad-phase test before cloning full `Flight` values, but an AI–AI pair starts by cloning the player state and then replaces it with an AI clone.

**Recommendation:** reuse fixed-capacity/per-Combat scratch storage; expose vegetation bounds through a caller-owned buffer or iterator; construct the required collision snapshots directly, preferably from a small immutable geometry/pose view. Cache deterministic vegetation geometry by cell only if it preserves the full swept candidate set.

**Guardrail:** preserve contact order, geometry, damage state and seeded randomness. These affect simulation throughput; they should not be sold as a guaranteed cockpit GPU improvement.

### 9. Indirect-lighting inputs remain tied to camera orientation

**Evidence:** [exact scene inputs](/Users/Austin/Downloads/GripenFlight/src/atmospheric_render.rs:41), [environment filter shader](/Users/Austin/Downloads/GripenFlight/vendor/bevy_pbr/src/light_probe/environment_filter.wgsl:116).

The environment source version includes the camera transform, and the source comment explains why: the current atmosphere basis is view-dependent. Moving/rotating the camera therefore prevents treating the scene as a permanently unchanged environment. The existing slicing/cache implementation already reduces peak/redundant work; this is an architectural remaining dependency, not a missing basic cache.

**Recommendation:** consider a world-oriented environment capture independent of the visible-sky camera, while retaining view-dependent sky/aerial perspective every frame. This requires reformulating the source consistently, not simply deleting camera orientation from a cache key.

**Guardrail:** retain complete-generation publication, exact resource readiness and transition cancellation. This is high-risk renderer work and needs later visual validation. No new GPU measurements were taken here, and prior documents' atmosphere timings are historical evidence only.

### 10. Orbital map work repeats when its inputs can be unchanged

**Evidence:** [orbit/moon line construction](/Users/Austin/Downloads/GripenFlight/src/map.rs:424), [prediction refresh](/Users/Austin/Downloads/GripenFlight/src/map.rs:504), [768-step predictor](/Users/Austin/Downloads/GripenFlight/src/space_navigation.rs:101).

The map rebuilds orbital and lunar lines each rendered frame. Its prediction cache also expires after 0.2 seconds of wall time, even when paused and the underlying state has not changed. Prediction allocates a new point vector and can execute up to 768 propagation iterations.

**Recommendation:** invalidate predictions using complete state/node/epoch revisions, retaining the existing live cadence. Reuse forecast storage. Cache body-relative line geometry and apply current frame/rotation separately. In a paused unchanged map, wall time alone should not require recomputing the same forecast.

### 11. The start screen uses a broad exclusive-world presentation system

**Evidence:** [menu presentation and cameras](/Users/Austin/Downloads/GripenFlight/src/start_screen.rs:574), [broad props query](/Users/Austin/Downloads/GripenFlight/src/start_screen.rs:608), [preview resource substitution](/Users/Austin/Downloads/GripenFlight/src/start_screen.rs:634).

`presentation(&mut World)` is an exclusive system. It constructs query states during calls, writes camera activation each frame and even updates the hidden menu camera before the closed-menu return. While open, its props query has optional markers but no restricting filter, so it visits entities with `Visibility` and `Transform` beyond just menu props. Vehicle changes additionally substitute global Flight/ViewState resources and run visual systems to build a copied preview.

**Recommendation:** split transition-only setup from ordinary animation; use cached/typed queries restricted to menu entities. Update camera activation on menu-state changes, and animate only the visible menu camera/props. A dedicated render pose would avoid temporary global simulation-resource substitution.

**Scope:** principally menu CPU work and architecture; preview cloning occurs on selection refresh, not every frame.

### 12. Constant audio-voice pan gains are recomputed at 48 kHz

**Evidence:** [sample rate/pool bounds](/Users/Austin/Downloads/GripenFlight/src/sound.rs:15), [voice mixing](/Users/Austin/Downloads/GripenFlight/src/sound.rs:1118), [pan calculation](/Users/Austin/Downloads/GripenFlight/src/sound.rs:1182).

Each voice mix computes two square roots for equal-power panning on every sample. Voice pan is stable between event/control updates, making this a specific hoisting opportunity. The eight engine emitters smooth their pan per sample, so they do not have the same constant-input property.

**Recommendation:** cache left/right gains for effect voices when their pan changes. Keep emitter smoothing, tone synthesis, event timing and callback nonblocking behavior unchanged. Avoid a broad “skip muted synthesis” change without considering phase/state continuity.

## Smaller issues that should not lead the next pass

- [Missile trail `remove(0)`](/Users/Austin/Downloads/GripenFlight/src/combat/guidance.rs:735) shifts up to 48 points at emission time, not every frame. A ring buffer is cleaner, but this bounded copy is less important than whole smoke-mesh reconstruction.
- Identical radar bodies/tails are generated twice under different handles, and static store meshes are generated repeatedly. Sharing them is straightforward startup/memory work; do not expect this alone to transform FPS.
- String formatting in UI pages and repeated orbital/heading derivations can be consolidated after larger costs. Replacing every `format!` indiscriminately is not a strategy.
- Release ThinLTO in [Cargo.toml](/Users/Austin/Downloads/GripenFlight/Cargo.toml:25) and the executable copy/strip/sign sequence in [build-mac.sh](/Users/Austin/Downloads/GripenFlight/build-mac.sh:24) concern iteration/packaging time. They do not explain a frame-rate drop while flying. Keep the development profile for iteration and avoid unnecessary release builds.

## Existing work worth preserving

- GPU Earth shading is the normal path: [material conversion](/Users/Austin/Downloads/GripenFlight/src/earth_gpu.rs:83) replaces the standard-material layers. The CPU color loop in `earth.rs` is a fallback/reference path; it should not be reported as the default 400,000-vertex per-frame bottleneck.
- Cockpit images already use persistent textures, changed-region uploads, reusable staging buffers and hidden-view/cadence guards.
- Environment lighting already has reusable inputs, sliced filtering and complete-generation publication. Do not undo the scene-corruption fixes for a simpler cache.
- Terrain already generates geometry asynchronously and keeps complete coverage until replacement; the remaining concern is bounded preparation/publication.
- AI physics already has a disjoint parallel integration path with stable shared processing. Small formations deliberately remain serial.
- Material helpers such as [render_cache::optics](/Users/Austin/Downloads/GripenFlight/src/render_cache.rs:19) and several transform/visibility paths already skip identical writes.
- Audio callbacks use `try_lock` and bounded voice pools. A mutex found in source is not by itself evidence of a blocking audio callback.
- Camera-relative rendering, f64 physics, bounded pools and the Cargo cache policy are existing correctness/resource safeguards.

## Suggested implementation order

1. **Low-risk cleanup:** unchanged writes, scratch-vector reuse, shared identical radar/store meshes, restricted menu queries and constant voice pan coefficients.
2. **Cockpit work:** per-page invalidation, shared radar projections, retained static raster layers and producer-side dirty regions.
3. **Render submission:** merge/share static details only within compatible movement, damage, material and shadow groups.
4. **Smoke data path:** retained GPU billboard instances and conservative bounds, preserving the current appearance.
5. **Terrain publication:** bounded pending preparation and coherent activation, preserving complete coverage.
6. **AI/map bookkeeping:** behavior-preserving score reuse and paused-state forecast caching.
7. **Scheduling and atmosphere redesign:** explicit dependencies, split calculation/commit stages, then consider a world-oriented environment source.

When implementation is authorized, each change still needs targeted correctness/visual checks and a matched performance comparison. Those are future acceptance requirements, **not work run during this review**. Preserve graphics, timestep, physics, AI cadence, collision precision, seeded outcomes and controls. No source-only argument justifies promising 60 FPS, reducing quality or moving engines.
