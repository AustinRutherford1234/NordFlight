# Nord Flight optimization implementation — October 20, 2026

This pass implements the source-review recommendations in `CODE_OPTIMIZATION_REVIEW_2026-10-20.md`. Code changes came first; compilation checks ran during development, with consolidated correctness, native visual and performance checks at the end. The starting installed optimized executable and SHA-256 source manifest are retained locally under `tests/optimization5/baseline/`. Existing uncommitted missile work was preserved.

**Acceptance decision:** ship the existing CPU smoke renderer by default. The
new retained GPU renderer is implemented and visually validated, but its FPS
benefit was not established. It is available for developer comparison with
`NORD_SMOKE_GPU=1`; `NORD_SMOKE_CPU=1` overrides that switch. No automatic quality
reduction or renderer selection is applied during flight. The experimental GPU
measurements below are labeled separately from the accepted shipping build.

## Implemented changes

| Area | Implementation | Fidelity constraints |
| --- | --- | --- |
| Detailed missile smoke | Developer opt-in GPU path: retain emission samples in a nearby double-precision anchor frame; calculate card orientation, expansion, drift, overlap and alpha in a vertex shader. History meshes update when emitted samples change, rather than on every camera movement. | Same six cards per segment, procedural texture, five-second fade, drift equations and retained population. The moving powered endpoint updates through a uniform. Conservative bounds include growth and endpoint motion. Shipping default remains the existing CPU path. |
| Cockpit displays | Retain unchanged numeric rows, repaint changed rows, and publish the dirty row span directly. Share matching tactical radar rasterization within a display refresh. | Existing display resolution, cadence, glyphs, HUD/RWR rendering and inputs. Power-off invalidates numeric content; power-on performs a full redraw. Other dynamic pages retain the complete pixel-comparison fallback. |
| Texture uploads | Add authoritative dirty-row publication, with pending-region union and staging reuse. | Changes remain pending until a matching GPU image is ready; multiple CPU updates cannot erase unflushed regions. Full raster comparisons remain available. |
| Terrain streaming | Prepare at most one replacement tile and its forest batches per frame, initially hidden. Render-world readiness checks every pending mesh. Publish complete replacement coverage together, with current origin-relative transforms; retain old coverage until then. | No terrain triangle, tree population, shadow, contact or LOD reduction. Active and pending coverage are bounded by the existing layout. Initial startup remains synchronous. |
| Aircraft/missile assets | Share bit-identical immutable aircraft meshes, confirming attributes and indices after hashing. Reuse identical radar missile body and tail handles. | Individual animation, damage, picking, hierarchy, materials and visibility remain separate. No merging of independently controlled surfaces. |
| Scheduling | Name and order controls, simulation, world, camera, presentation, interface and capture stages. Remove the missile-mechanism chain after adding disjoint query filters. | Successive authoritative steps and shared event ordering remain serial. Independent missile root/fin presentation may overlap; camera/origin state precedes presentation. |
| Simulation bookkeeping | Use fixed-capacity combat snapshots, reuse per-thread vegetation-contact storage and eliminate a redundant player clone for AI–AI contacts. | Original candidate traversal, shaped geometry, contact order, floating-point equations and random-number consumption. |
| BVR decisions | Build current teammate assignment counts once per pilot and calculate each candidate score once. | Counts reflect earlier pilots' decisions in the same tick; target order and tie handling remain unchanged. |
| Orbital map | Key prediction reuse on position, velocity, epoch, body, complete node data and scene generation. Reuse prediction vectors and paused orbit/Moon line samples. | Live forecast cadence and propagator remain unchanged; changes to node reference/epoch cannot leave a stale prediction. |
| Menu/audio/stable writes | Restrict menu prop queries, avoid hidden menu camera updates and unchanged activation/visibility writes. Cache stationary effect-voice pan gains when emitter updates arrive. | Engine pan smoothing and sound samples remain intact. No reduction in aircraft preview animation or audio quality. |

## Deliberately retained boundaries

The atmosphere source currently uses a view-dependent sky basis. Camera orientation remains in its validity key. A world-oriented environment source requires a coordinated shader/binding change and visual validation; deleting the orientation key would be incorrect. This renderer rewrite is **not implemented** here.

Rigid-part merging beyond identical mesh sharing is also deferred: moving, detachable and interactive attachment groups must retain independent behavior. This pass does not claim a general reduction in all cockpit draw calls. Cockpit formatted text and distinct radar pages still have some recurring CPU work. Terrain generation remains one bounded asynchronous job; this pass spreads resource publication, without adding speculative worker contention.

## Validation

Results and measured performance are recorded below after final validation. Runtime artifacts remain local and ignored by Git; reproducible focused orchestration is in `tests/optimization5/validate.py`.

- Rust regression suite: **380 passed**, including retained-row/full-redraw equivalence, pending upload union, GPU trail encoding against CPU reference geometry after motion, mesh identity, prediction cache invalidation/storage reuse, and complete terrain publication after simulated GPU readiness.
- Packaging/cache-policy tests: **3 passed**.
- No physics coefficients, integration methods, configured timestep, AI decision frequency, missile guidance cadence, texture resolutions, shadow settings or effect populations were reduced.

Nine authoritative JSON diagnostic reports match the preserved executable after
excluding the timing fields listed in `tests/optimization3/verify.py`. Both AI
telemetry CSVs match byte for byte. This includes flight, AI flight/combat, BVR,
air combat, missile guidance, audio, lunar flight and integrated workloads.

The native development-build GPU/CPU smoke captures differ at **169 of 1,296,000
pixels**, with a maximum difference of **one channel level**. Cockpit, forest and
lunar map captures were inspected. These are native renderer captures, not
generated illustrations. The geometry test also compares retained GPU input
against the CPU reference after motion without a history rebuild.

The experimental GPU release build completed in **4m 17s** and passed strict
bundle signature verification. Its executable
SHA-256 is `b0131c8647c3763a70ca886411064410000bafd1e98ab2c326c868f8f3fd239f`;
The preserved executable is
`ed6aba7b22ada56b8af52cd35d302f4eda2b506ced4e51811e428c6f3c77fbea`.
The accepted default-mode build is recorded separately below. The Cargo cache
remained **8.8 GiB**, under the existing 12 GiB policy.

## Focused performance results

Apple M1/Metal, 1440 × 900, existing default visual preset and shipping VSync.
Runs were sequential, without compilation, with 10-second warmups and 25-second
samples. The laptop was on battery; the host reported no thermal/performance
warning. These are short comparisons, not a statistically conclusive benchmark
campaign or a demonstrated hardware limit.

| Workload | Average FPS, baseline → GPU candidate | p95 frame ms, baseline → GPU candidate | p99 frame ms, baseline → GPU candidate |
| --- | ---: | ---: | ---: |
| Paused cockpit | 41.69 → 43.38 | 29.10 → 25.48 | 39.64 → 28.13 |
| Live BVR, first pair | 43.19 → 43.58 | 25.09 → 34.01 | 30.48 → 34.58 |
| Live BVR, targeted repeat | 45.38 → 42.81 | 33.89 → 34.06 | 34.22 → 34.42 |
| Stationary forest preset, live simulation | 45.97 → 46.25 | 35.38 → 35.12 | 36.39 → 36.10 |
| Deterministic terrain streaming, 50 m AGL / 220 m/s | 40.75 → 40.86 | 35.59 → 35.22 | 36.49 → 36.08 |

The streaming path moves about 7.7 km during warmup/sample and holds simulation
paused to isolate terrain presentation. It is separate from the stationary
forest preset, which can settle/crash during the sample and does not prove
streaming performance. Production simulation is unchanged.

The cockpit pair shows a small FPS improvement and better tail frame times.
Terrain averages are effectively unchanged. **BVR FPS is inconclusive:** the
first pair slightly favors the candidate, while the repeat is 5.7% slower. The
initial p95 regression did not repeat at that magnitude; both repeat runs have
approximately 34 ms p95. No broad BVR FPS improvement is claimed.

A final candidate run using `NORD_SMOKE_CPU=1` measured **44.85 FPS**,
**33.89 ms p95**, and **34.42 ms p99**. Asset preparation averaged 0.641 ms,
versus 0.502 ms in the first GPU run. This confirms the CPU/data-work tradeoff,
but does **not** establish a GPU-smoke FPS benefit: the CPU reference was faster
than the last GPU sample. The short sequential runs cannot separate that
difference conclusively from host/presentation variance. The acceptance choice
is to keep the CPU renderer as default and retain the GPU path as developer
opt-in. GPU material/vertex work and frame pacing need further focused profiling.

The first BVR pair nevertheless demonstrates reduced targeted work:

| Measurement | Baseline | Candidate |
| --- | ---: | ---: |
| Cockpit CPU mean per rendered frame | 0.075 ms | 0.036 ms |
| Asset preparation CPU mean per frame | 0.704 ms | 0.502 ms |
| Mesh modification events in sample | 6,639 | 2,181 |
| Material modification events in sample | 2,460 | 3,596 |
| Display-upload bytes in sample | 24,505,268 | 24,450,148 |
| Simulated seconds per wall second | 0.719 | 0.726 |

GPU smoke replaces recurring geometry updates with small per-trail material
uniform updates, explaining the material-event increase. These event counts
are not uploaded-byte measurements. Retained numeric pages reduce rasterization;
the continually changing HUD/radar still dominate display uploads.

In the matched paused cockpit, the renderer's mesh-allocation gauge fell from
**2,339 to 1,548** (791 fewer, 33.8%). Reserved GPU mesh-slab capacity remained
220,073,912 bytes in both runs; this is not a claim of equivalent VRAM savings.
CPU timing spans can be nested and must not be summed into a frame budget.
Per-pass GPU durations and draw-call counts were unavailable in these captures,
so the remaining milliseconds cannot be assigned conclusively to a particular
GPU pass. Frame intervals include presentation waits and scheduling.

## Stability and remaining work

The experimental GPU candidate's 120-second mixed-workload sample completed with no runtime errors; the whole
process ran for 127.2 seconds including preparation. It exercised BVR, close
combat, terrain, gear, Earth/Moon maps, ascent/descent, lunar landing, warp,
cockpit and effects. On returning to BVR, entity/mesh counts returned to
**8,274 / 1,558**, matching the start. Images stabilized at 75 after initial
lighting preparation. Peak sampled process RSS was 1,066 MiB. This short check
does not establish long-term memory stability.

**60 FPS was not achieved in the sampled atmospheric workloads.** The remaining
priorities are:

1. Resolve rendering/presentation frame pacing, particularly the BVR variance.
   Instrumented CPU savings are small compared with the remaining frame time;
   a later focused Metal capture is needed to attribute GPU execution versus
   synchronization. The current measurements do not justify an engine migration
   or a claim that the M1 has reached its absolute limit.
2. Reformulate the indirect-lighting source in a world-oriented basis, preserving
   complete-generation publication, cancellation and readiness. This remains
   the architectural atmosphere change described in the review.
3. Merge only proven-compatible rigid attachment groups. Identical mesh sharing
   is implemented; general merging across controls, damage and cockpit picking
   is not.
4. Further reduce cockpit string formatting and share projections across
   distinct radar/HUD pages. This pass shares identical page rasterization and
   retains numeric rows, rather than replacing all display rendering.
5. If future profiling warrants it, split dense individual terrain tiles more
   finely or parallelize bounded tile generation. Current publication is bounded
   by tile count rather than a strict millisecond budget; startup generation is
   still synchronous.

Local artifacts: `tests/optimization5/equivalence.json`,
`smoke-pixels.json`, `regressions.log`, `render-baseline/`, `render-candidate/`,
`visual-gpu/`, `visual-cpu/` and `stability/`. The reproducible harness supports
`--phase visual`, `final`, `terrain`, `bvr-repeat` and `shipping`. The `final`,
`terrain` and `bvr-repeat` phases explicitly select the experimental GPU path;
`shipping` clears developer overrides and tests the accepted default. Captures and executable
snapshots remain bounded to this pass and are ignored by Git.

The final smoke-reference ablation is recorded in
`tests/optimization5/render-cpu-reference/bvr.json` and
`tests/optimization5/smoke-ablation.log`. It changes rendering only; the same
candidate executable is used.

## Accepted shipping artifact

The installed `Nord Flight.app` was rebuilt with **CPU smoke as default** and
the GPU renderer behind `NORD_SMOKE_GPU=1`. That final rebuild completed in
**4m 32s** and passed strict bundle signature verification. Its executable
SHA-256 is **`9d5150c8b036dd5a58ca541aaea028fdfa94926ea4f928c263460c287ef3b7b9`**.
The authoritative simulation code is identical to the numerically compared
candidate; the final change selects the default smoke presentation path.
Final native checks are recorded under `tests/optimization5/shipping/`.

The final signed app launched successfully and its F-22 cockpit capture was
inspected. Its default-mode BVR run measured **44.99 FPS**, **33.95 ms p95**,
**34.44 ms p99**, and **0.750 simulated seconds per wall second**. The nearby
baseline repeat measured 45.38 FPS / 33.89 ms p95: overall BVR performance is
effectively unchanged in this bounded comparison, rather than a demonstrated
FPS improvement. Cockpit processing averaged **0.038 ms per frame**, compared
with the first baseline's 0.075 ms; asset preparation averaged **0.609 ms**,
compared with 0.704 ms. These remain short sequential battery-powered samples.

The final build retains the cockpit, asset-sharing, terrain, cache, scheduling,
audio and simulation-bookkeeping improvements. The more speculative GPU smoke
optimization is implemented but excluded from the default rendering path.
Restart the app to load the rebuilt executable; validation preserved the
previously running game process rather than discarding its flight state.
