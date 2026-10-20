# Performance and build notes

The later focused cockpit/atmosphere pass is complete and installed. See
[ATMOSPHERIC_OPTIMIZATION.md](ATMOSPHERIC_OPTIMIZATION.md): the six measured
workloads now average 59.7–59.8 FPS at 1440 × 900, with unchanged simulation and
visual settings. Strict 16.7 ms p95 remains unmet. The historical report below
predates this pass.

October 17 measured results and delivery status are in
[OPTIMIZATION_REPORT.md](OPTIMIZATION_REPORT.md). Diagnostics are stopped at the
user's request. Orbital/map views approach 60 FPS; cockpit remains about 33 FPS.
The optimized candidate was subsequently installed into `Nord Flight.app` on
user request; signature and native launch were verified. No benchmarks resumed.

This pass targets smoother gameplay and faster simulation at the existing visual quality. Physics coefficients, configured timesteps, solver iteration counts, collision-skin detail, terrain meshes, particle limits, shadow settings and antialiasing are retained.

## Runtime changes

- Reserve one Bevy worker for I/O and one for asynchronous background work, leaving the remainder of Bevy's detected thread budget for its compute pool. On the eight-core M1 MacBook Air used here, the compute pool has six workers instead of the default four. This is a worker budget, not CPU affinity or a guarantee that every worker is busy.
- Update independent scenery transforms with Bevy's parallel query iterator. Remove unnecessary ordering between presentation systems; Bevy still serializes conflicting resource/component access. Keep input, simulation, camera updates and final UI visibility in their required order.
- Redraw the damage texture and failure labels only when their displayed state changes. A burning aircraft retains the original per-frame flame-symbol animation. Rasterize each polygon inside its clipped bounds rather than scanning the entire texture.
- Avoid issuing a navball image modification when its existing attitude/velocity cache says no redraw is needed. Keep the cockpit's 3D attitude-material refresh, which is needed to bind the prepared texture correctly.
- Transform convex aircraft pieces once per aircraft-contact query, reject nonoverlapping swept bounding boxes before detailed SAT, and stream separating axes without allocating an axis list per pair. Cache immutable skin-projection half-spaces for burst-distance queries.
- Reuse sampled terrain-collision trajectories across the aircraft's convex pieces. Preserve the sample count, orientations and individual vertex terrain checks.
- Calculate the six control-derivative panel evaluations only when fly-by-wire allocation uses them. Raw-control flight retains the same direct requests and surface physics.

Successive timesteps of the same aircraft remain serial because each depends on the preceding state. Raising worker counts alone cannot accelerate that dependency, and GPU-limited scenes will not scale with CPU core count. The renderer, physics and diagnostic batch throughput are measured separately.

## October 15 source-wide review

The later [40-minute follow-up](PERFORMANCE_40_MINUTE_PASS.md) adds prepared
panel-flow reuse, cached engine intake terms, parallel integration for large
BVR formations, reusable tactical buffers, exact solar/render-frame caches,
persistent instrument textures, full-resolution label atlases, streamed/cullable
missile trails and more precise UI/render change detection. The numerical and
visual settings are retained. Its final development comparisons and release
delivery are recorded separately in [VALIDATION.md](VALIDATION.md).

The follow-up [optimization audit](OPTIMIZATION_AUDIT.md) covers every Rust
module. It adds conservative hull-bound rejection for flat-airfield ground
checks, exact bounded terrain-visibility caching, background terrain/forest
mesh construction, detailed gear-state caching, and fewer unchanged material,
transform and text updates. Hidden tactical maps skip their layout work, HUD
drawing buffers are reused, and missile tracking borrows its target rather than
cloning it. Physics and AI frequencies and visual quality settings are retained.

`python3 tests/compare_optimization2.py BASELINE CANDIDATE` runs three paired
whole-simulation and focused-workload comparisons. Add `render` to run the
separate four-run renderer comparison. Both executables must use the same
profile. The October 15 pass uses matched development builds; it does not claim
release-build speedups from development timings. Full samples, numerical
equivalence checks and executable hashes are retained under
`tests/optimization2-*-comparison.json`, with results in
[VALIDATION.md](VALIDATION.md).

## Reproducing checks

Build with `./build-mac.sh release`. Run `cargo test --locked` and `target/release/gripen-flight --verify-flight` for functional checks. The release build uses optimization level 3 and thin LTO. Development remains incremental with `./build-mac.sh`; changing release source can require another LTO/link pass even when dependencies are cached.

`python3 tests/compare_performance.py BASELINE CANDIDATE` runs three paired, sequential CPU studies with alternating executable order. It records executable hashes, wall times and complete numerical outputs, and fails if the polar/maneuver results or deterministic combat reports differ. Do not run a compiler or a renderer benchmark concurrently. The flight study includes 294 polar samples, six maneuvers, 100 repeated landings and an orbital coast. Its CPU timing is not renderer FPS.

`target/release/gripen-flight --benchmark-render performance-after` runs the unchanged paused runway scene with VSync, a 20-second warmup and a 40-second sample. It records mean FPS, median/p95/p99 intervals and actual Bevy worker counts. Native captures use `--visual-check MODE`; they establish appearance rather than dynamic performance. Host, display, compositor, thermals and background activity affect measurements. Results from this pass are recorded in [VALIDATION.md](VALIDATION.md).

`python3 tests/compare_render.py BASELINE CANDIDATE` runs that same scene four times in before/after/after/before order and records each run and the two-run mean FPS. It must run separately from CPU studies and compilation.

## Rust compatibility warning

The reported `block v0.1.6` warning originates in the older Objective-C block API used by the Metal graphics dependency. It is unrelated to flight dynamics, and the builds shown in the terminal completed successfully. A narrow MIT-licensed source patch under `vendor/block` replaces an uninhabited foreign-symbol type with an opaque inhabited type, takes its raw address, and explicitly names the existing C ABI. It preserves the dependency's API and block layout. The patch's provenance and maintenance/removal notes are in `vendor/block/PATCH.md`; diagnostics are not suppressed.

To inspect the old stored report, run `cargo report future-incompatibilities --id 1`. To request a report for a new build, run `cargo build --release --locked --future-incompat-report`. `--future-incompat-report` is an argument to Cargo rather than a shell command. Stored reports can still describe older build artifacts after the source is fixed.

`Nord Flight.app: replacing existing signature` is the normal output from replacing the development bundle's ad-hoc code signature.
