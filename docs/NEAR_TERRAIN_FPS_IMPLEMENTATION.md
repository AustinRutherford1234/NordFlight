# Near-terrain optimization implementation

October 21, 2026. Implementation and focused release measurements.

Implements the small, compatible changes in
[NEAR_TERRAIN_FPS_FIX_PLAN.md](NEAR_TERRAIN_FPS_FIX_PLAN.md). The user requires
battery operation. All native runs in this pass use battery power; no AC result
is substituted or requested. The original installed signed executable, source
hashes, asset hashes and starting dirty-worktree patch are preserved under
`tests/optimization8/baseline/`.

## What changed

### Atmosphere producer follows the filter's immutable capture

The canonical indirect-light source previously regenerated its sky lookup and
six cubemap faces while the environment filter was still consuming an earlier
immutable capture. The producer now asks the filter whether that owned snapshot
is still valid before dispatching that unused work. The visible camera sky and
aerial perspective remain current every frame. Capture boundaries, filter
sample counts, formats, resolution, mip coverage and complete publication are
unchanged.

Reuse requires the same scene generation, resource epoch, actual bound source,
front/back targets, immutable intermediate, and all five ready filter pipelines.
Source signatures/revisions advance only for actual writes. Resource changes,
cuts, missing readiness, unsupported ownership, multiple views and the developer
full-update path fall back conservatively. Replacing filtering bindings resets
pending progress rather than continuing into newly allocated textures.

Files: `vendor/bevy_pbr/src/atmosphere/environment.rs` and
`vendor/bevy_pbr/src/light_probe/generate.rs`.

### HUD coverage and opaque shader specialization

The aircraft cockpit HUD now draws only the conservative rectangle containing
nonzero-alpha texels, padded by two texels for filtering. Texture resolution,
plane-to-UV mapping, material lighting, emissive response and refresh cadence
remain unchanged. A genuinely empty HUD hides until content returns; it does
not use a zero-scale transform. Glass, canopy, navball and radar materials retain
their existing rendering behavior.

Source inspection also found that the existing zero-alpha HUD discard had been
compiled into opaque PBR shaders through a runtime material-mode branch. It is
now compiled only into alpha-blended pipelines. This removes an unnecessary
discard instruction from opaque terrain/forest-compatible shaders without
changing their intended output. Whether this improves early visibility on this
GPU must be distinguished from the source-level correction; it is not by itself
proof of the dominant frame bottleneck.

Files: `src/cockpit.rs`, `src/cockpit/coverage.rs`,
`vendor/bevy_pbr/src/render/mesh.rs` and `render/pbr.wgsl`.

### Shared terrain transforms and bounded streaming coalescence

Each tile's forest groups inherit the tile's camera-relative transform instead
of each carrying another equivalent `WorldObject` transform. Their individual
meshes, geometry, materials, bounds, shadow participation and visibility remain.
The first visible frame is tested against the root's global transform.

When a worker completes after the requested cell changes, it can retarget once
before GPU staging: retain useful completed tiles, dispose of obsolete CPU
meshes on the worker, and generate only missing tiles in stable order. One
retarget is allowed per completion so continuous motion cannot starve a valid
coverage publication. Once GPU staging begins, that complete coverage finishes;
old coverage remains until the replacement is GPU-ready. Existing byte/count
upload and retirement budgets remain intact.

File: `src/terrain.rs`.

## Scope retained

Authoritative physics, collision, AI, guidance, integration, double precision,
damage randomness, tree placement and assets were not edited. No reduction in
render resolution, tree population, mesh detail, textures, shadow quality,
lighting formulas, effects, instrument cadence or shipping VSync was made.

Instanced-tree reconstruction, static shadow reuse, custom mesh allocators and
core render-pass/backend rewrites remain conditional follow-ups from the plan.
They are not presented as implemented. The unsuccessful generic depth-prepass
experiment stays disabled. Existing display handoff and scheduling safeguards
are retained; no busy wait or GPU readback was added.

Developer-only reference switches are `NORD_ENV_SOURCE_REFERENCE`,
`NORD_HUD_COVERAGE_REFERENCE`, `NORD_TERRAIN_TRANSFORM_REFERENCE` and
`NORD_OPAQUE_DISCARD_REFERENCE`. They select the previous corresponding paths;
none changes physics or public controls.

## Correctness and visual evidence

- **401 application tests passed**, including HUD filtering/mapping/empty recovery,
  retained streaming results and first-visible-frame forest transforms.
- **15 vendored renderer tests passed**, including capture-boundary/publication
  equivalence across one/three/six stages and cuts, resource/pipeline invalidation,
  and opaque-versus-alpha shader specialization.
- Native F-22 external and cockpit views launched successfully. The external
  forest screenshot was pixel-identical to the baseline after masking the
  frame-time readout. The cockpit image's mean absolute channel difference was
  **0.00381 / 255**, p99 zero; **0.108%** of pixels differed by more than one level,
  confined to HUD sampling. Images and an amplified difference are in
  `tests/optimization8/dev-visual/`. These development captures validate the
  rendering path; they are not optimized FPS measurements.
- Release build/package/sign completed in **4m 14s**. Development rebuilding
  required clearing only the interrupted `bevy_pbr` development artifacts after
  a cached-object linker failure; the release build succeeded normally.
- The release baseline/candidate seeded flight studies are **exactly equal
  excluding timing fields** (`tests/optimization8/flight/equivalence.json`).
- The two-minute moving terrain/cockpit run exited successfully. Across 128
  route samples, mesh counts remained **1,434–1,585** and entities **8,367–8,518**;
  peak process RSS was **1,103 MiB**. The route eventually leaves forest for
  coast, so changing resource counts are expected. RSS declined later, but
  resident pages are affected by macOS pressure and do not prove GPU allocation
  retirement by themselves. No unbounded entity/mesh accumulation was observed.
- **1,200 native frames across 100 scripted transitions** were captured, covering
  cockpit/external/follow cameras, tactical map return, scenario resets, weather,
  daylight and both quality presets. Every frame received blank/saturation
  checks; none was nearly blank. The 100-view contact sheet and consecutive
  first-cycle strips were inspected without visible screen corruption or
  incomplete-lighting flashes. This is representative visual inspection plus
  per-frame checks, not a pixel-perfect baseline comparison of all 1,200 frames.
  All state/statistic records, 243 representative/consecutive PNGs and contact
  sheets are retained; 957 redundant PNGs were retired to bound storage.

## Focused native measurements

Acceptance data is recorded under `tests/optimization8/`: 10-second
warmup, 25-second samples, sequential baseline/candidate runs, matching camera,
path, resolution, presets and battery operation. FPS runs use
`NORD_BENCH_FPS_ONLY=1`; mechanism counters and Metal capture are separate.
Creative Cloud background CPU load and battery variability are recorded rather
than silently treating this laptop as an isolated powered benchmark host.

### Uninstrumented frame results

All runs retained the default graphics preset and shipping VSync. Fullscreen
was **2880 × 1800 actual pixels**, not 1440 × 900. The scripted paused route moves
at 100 m/s, 50 m AGL and stays over forest for the full 25-second sample; it
crosses streaming cells with authoritative flight paused. Live flight starts
350 m AGL with the ordinary flight model advancing. The reverse pair runs the
candidate before the baseline. Each table row is one complete sample.

| View / order | Baseline FPS | Candidate FPS | Baseline p95 / p99 ms | Candidate p95 / p99 ms | Frames >50 ms, baseline / candidate |
|---|---:|---:|---:|---:|---:|
| Fullscreen forest external, B→C | 35.36 | 46.74 | 36.64 / 49.69 | 34.59 / 52.90 | 8 / 13 |
| Fullscreen forest external, C→B | 35.92 | 46.18 | 33.84 / 37.81 | 34.55 / 35.03 | 3 / 1 |
| Fullscreen forest cockpit, B→C | 35.16 | 36.42 | 34.31 / 34.86 | 34.69 / 36.46 | 2 / 5 |
| Fullscreen forest cockpit, C→B | 18.04 | 28.46 | 71.06 / 148.96 | 50.72 / 51.55 | 259 / 68 |
| Fullscreen live low-altitude cockpit, B→C | 28.27 | 26.37 | 52.89 / 54.14 | 52.90 / 53.98 | 104 / 136 |
| Fullscreen live low-altitude cockpit, C→B | 26.73 | 27.12 | 51.21 / 52.31 | 52.17 / 53.81 | 139 / 128 |
| 1440 × 900 forest cockpit, B→C | 43.31 | 51.49 | 34.12 / 35.46 | 33.70 / 34.42 | 2 / 2 |

The external gain repeats in both orders: **28.6–32.2% more FPS**, roughly
**30%** across pair means, with mean frame time improving from about **28.1 to
21.5 ms**. Its p95 remains around 34.6 ms, so neither 60 FPS nor uniformly smooth
16.67 ms frames has been achieved. The first pair's worse p99 triggered the
reverse repeat; the tail regression did not repeat.

The fullscreen cockpit repeat is conspicuously slower for both versions, with
large baseline outliers. It is retained in the record, not averaged into an
inflated cockpit improvement claim. Background processes, battery/thermal
variability and host activity prevent a reliable fullscreen cockpit percentage.
The 1440 × 900 pair improved **18.9%**, but is one pair. Live cockpit results show
no repeatable gain; its initial 6.7% average-FPS regression reversed to a 1.5%
gain. Do not describe this pass as a proven live-cockpit performance fix.

Battery state was recorded for every run (approximately 73% down to 66% through
the frame comparisons). Creative Cloud, browser/application processes and macOS
services were active at varying CPU loads. `pmset` reported no thermal warning;
that is not proof of constant GPU clock or zero throttling. No apps were killed
to manufacture an isolated result. FPS-only reports intentionally have disabled
simulation counters; their zero `simulation_rate` is **unavailable telemetry**,
not evidence that live simulation stopped.

### Producer/consumer mechanism

In the separate instrumented same-release comparison, the reference performed
**239 source writes / 40 captures / 40 publications**, versus **40 / 40 / 40**
with gating. Both reported maximum update age **6 frames**, with no cancelled
cycles in the steady route. That is **83.3% fewer upstream writes** for that
sample, preserving the filter's capture cadence. The instrumented runs were
about 24 FPS in both cases; instrumentation changes the workload, and those
numbers are not accepted FPS results or proof of an isolated P1 speedup.

### Current Metal evidence and remaining costs

One five-second native fullscreen cockpit Metal System Trace captured 135 frame
labels. These are **median active channel intervals per observed frame**, not
exclusive serial pass durations; stages and GPU channels can overlap:

| Label / channel | Median active interval ms |
|---|---:|
| Main transparent / fragment | 17.51 |
| Main opaque / fragment | 13.64 |
| Environment radiance / compute | 4.46 |
| Main opaque / vertex | 3.91 |
| FXAA / fragment | 3.34 |
| Outer directional shadow / vertex | 2.59 |
| Sky / fragment | 2.53 |
| Environment irradiance / compute | 1.40 |

Do not add these intervals into a fabricated frame total or compare them as
equal-clock measurements against historical captures. The current capture
supports prioritizing contributing cockpit transparency and opaque pixel work,
then remaining environment filtering, shadow geometry and submission. Cropping
empty HUD borders does not eliminate the physically contributing canopy/glass.
This capture does not isolate each glass layer's cost or prove one exclusive
critical path. Hardware draw counts remain unavailable.

The separate drawable-acquisition table contains **127 app wait intervals**,
median **37.97 ms**, p95 **46.56 ms**, while profiling. These waits expose drawable
backpressure; they are not independent extra milliseconds to add to GPU work,
and do not establish an application mutex fault. Metal's application table
contains nested encoding/lifetime spans; their sum is not exclusive CPU
submission time. A trustworthy exclusive submission duration remains
**unresolved**, rather than relabeling a GPU/lifetime interval as CPU execution.
Detailed capture and aggregate data are retained under
`tests/optimization8/metal/`.

Next justified experiments are a feature-specialized glass shader preserving
the existing BRDF/IBL/fog and exact output, and exact shared tree-buffer submission
if geometry evidence warrants it. Static shadows still require complete caster
invalidation; fixed-rate updates and reduced shadow quality are not substitutes.
The measurements do **not** establish an unavoidable Bevy or hardware limit.

## Delivery and acceptance limits

The final app is `Nord Flight.app` in the repository root. Release packaging
completed and `codesign --verify --deep --strict` passed. All source assets,
authoritative source files and Cargo settings match the preserved starting
hashes; the changed-source inventory contains only the presentation/rendering
files described above. The original dirty work was preserved.

The 12 GiB Cargo-cache policy is unchanged; the final cache is below its limit.
Only the starting baseline executable and installed candidate are retained for
this pass. Tests, native process/power records, full percentile/stutter JSON,
mechanism counters and bounded visual/Metal evidence are under
`tests/optimization8/`.

Accepted: repeated external forest improvement, fewer unused atmosphere writes,
unchanged seeded flight outputs, passing regressions and native checks. Unmet:
60 FPS / 16.67 ms p95, a reliable fullscreen/live-cockpit improvement, exclusive
CPU-submission attribution, and full baseline motion-image equivalence for every
transition. These limits remain explicit; no quality or simulation concession
was used to conceal them.
