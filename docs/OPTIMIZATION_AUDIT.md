# Source optimization audit — October 15, 2026

This review covers all Rust modules in the simulator, including embedded tests
and diagnostic entry points. `tests/optimization2-source-inventory.json` records
the reviewed file hashes and sizes. Static allocation-site counts are search
results, not a runtime allocation profile. The changes below preserve configured
physics timesteps, AI update frequency, aircraft coefficients, collision detail,
terrain tessellation, sound synthesis and rendering quality settings.

## Implemented changes

| Area | Change | Correctness boundary |
| --- | --- | --- |
| Shaped airframe/ground contacts | Cache immutable hull bounds; reject a sampled hull above the flat airfield before testing every skin vertex. | The entire transformed box must clear the airfield and lie inside its flat region. All other cases retain the original vertex tests and trajectory samples. |
| Radar, seekers and AI awareness | Cache exact repeated terrain-visibility queries in a bounded, thread-local 256-entry table. | Both endpoint bit patterns must match. No coordinate quantization, stale moving-target reuse, locks or approximate terrain masking. Lake initialization invalidates the table. |
| Terrain streaming | Build replacement terrain and forest meshes on the existing asynchronous worker. | Keep complete old coverage until all replacements are ready; commit together. One job at a time bounds outstanding work. Geometry generation and collision terrain remain unchanged. |
| Detailed landing gear | Skip local assembly updates when extension, door position, compression, damage and actual steering are unchanged. | The aircraft root still moves normally; any visual input change invalidates the signature. |
| Cockpit and damage materials | Compare optical properties before requesting mutable asset access. | Actual color/emission changes still upload; unchanged values no longer issue material modification events. |
| Cockpit controls and lighting | Avoid unchanged transforms, visibility, light intensity and label assignments. | Animation, blinking and control changes still update. |
| Tactical map and HUD | Skip hidden map layout/readout work, reuse the HUD drawing buffer, and avoid unchanged text assignments. | Flight HUD status still updates while the map is hidden. Map opening computes its current contacts and missiles. |
| Missile tracking | Borrow target state instead of cloning an entire target aircraft per missile step. | Hit resolution and seeded update order are retained. |
| Physics status | Reuse unchanged flight status strings. | Status contents and transitions remain the same. |

## Review coverage and retained constraints

- **Aerodynamics, control allocation, propulsion, orbit and time warp:** reviewed
  update loops and temporary work. Numerical force evaluation, integration order,
  solver behavior and configured simulation cadence are retained. Successive
  steps depend on earlier results and cannot simply run concurrently.
- **AI, radar, missile guidance, damage and aircraft contact:** reviewed repeated
  visibility queries, target copying, pair checks and effect lifetimes. Retained
  shared six-degree-of-freedom aircraft physics and decision frequency. Parallel
  mutation of combat state was avoided because target damage, radar support and
  seeded outcomes depend on update order.
- **World, terrain, airframe, gear, detail and damage meshes:** reviewed mesh
  generation, asset updates, collision broad phases and lifetime bounds. The
  background task uses another worker without creating an unbounded queue or
  reducing terrain quality. Initial scene construction remains synchronous.
- **Cockpit, controls, navball, instruments, HUD, maps and scenario setup:**
  reviewed hidden work, raster caches, materials and label updates. Instrument
  refresh cadence and interaction paths are retained.
- **Audio:** reviewed the callback, bounded voices and shared-state access. The
  existing nonblocking callback handoff and voice limits remain; this pass does
  not change the gun/engine sound character or spatial sound model.
- **Build and dependencies:** retained the incremental development profile,
  optimized dependencies, release thin LTO, existing worker budget and bounded
  build-cache policy. Additional profiles and host-specific compiler flags would
  create more artifacts and require separate measurements.

## Measurement and validation

`tests/compare_optimization2.py BASELINE CANDIDATE` runs three paired repetitions
of the flight, air-combat, BVR and focused workloads, alternating executable
order. The saved executables use the same development profile. The baseline
includes the diagnostic helper but precedes the production optimizations.

The comparison checks the entire deterministic combat and BVR reports and all
reported flight polars, maneuvers and step counts. Elapsed-time fields are
excluded from numerical comparisons. The focused workloads check the same
visibility and collision result counts; separate regressions compare the exact
cached/uncached visibility results and validate conservative contact rejection.

The renderer comparison is a separate before/after/after/before pass through
the existing paused runway benchmark, with five seconds of warmup and forty
seconds of measurement per run. No compiler, test suite or CPU study runs during
these measurements. Host scheduling, thermals, compositor and VSync still affect
results; CPU diagnostic throughput is not renderer FPS.

Measured results and native capture evidence are recorded in
[VALIDATION.md](VALIDATION.md). Complete samples and executable hashes are in
`tests/optimization2-cpu-comparison.json` and
`tests/optimization2-render-comparison.json`.

This is a measured optimization pass, not a claim that all remaining runtime
cost has been eliminated. GPU draw calls, dynamic effects and the full aircraft
physics workload remain material costs. Terrain work can still finish late on a
busy host; old tiles remain visible while it catches up.
