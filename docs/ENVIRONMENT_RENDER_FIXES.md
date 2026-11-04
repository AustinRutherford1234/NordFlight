# Shoreline stability, terrain streaming and image cleanup

This follow-up addresses visible land/water breaks, vegetation update cost and
whole-scene grain in the real-terrain release.

## Changes

- Sea geometry remains at the configured sea level. Elevated lakes use fixed
  world-space, indexed near/far meshes rather than a camera-following ring whose
  vertices could blend unrelated water elevations. Dry shore vertices inherit a
  wet elevation; neighboring triangles share vertices. A separate render-world
  readiness gate keeps old coverage until replacement mesh, masks and material
  bindings exist on the GPU. Coverage updates every 4 km and is built in the
  background.
- Water masks query the lifted Cartesian terrain coordinate, matching terrain
  rendering and contacts. Previously, querying at sea level displaced mountain
  shorelines relative to elevated terrain. Mixed land/water tiles receive an
  additional refinement floor so flat water does not hide coastline detail from
  the sampled terrain-error estimator.
- Vegetation is combined into spatial-cell meshes on background workers. At most
  two jobs are outstanding and one cell is published per update. Empty cells
  create no render entities or GPU meshes. Near/far canopy changes use hysteresis,
  and only nearby canopy casts shadows. Authoritative tree/prop placement and
  collisions are unchanged.
- The 3D camera's default debanding noise is disabled. Standard graphics uses
  2× MSAA plus FXAA. Stochastic terrain texture contrast and normal strength are
  reduced, and detail fades with its pixel footprint rather than sparkling at
  long range. The interface stays at framebuffer resolution.
- An optional frame-start limiter runs before Bevy updates simulation time.
  It is disabled by default because native testing found that timer-based
  limiting alongside Vsync reduced frame rate.
  `GraphicsSettings.frame_rate_limit` supports 0 (unlimited) or 30–240 Hz.
  This is a ceiling rather than a guarantee of meeting the chosen rate.

## Validation scope

The full Rust suite passed with 883 tests and one ignored test after the core
changes. An additional actual-Norway-pack regression for lifted shoreline queries
then passed alongside the five ocean tests (six focused tests total). It locates
coordinates where the former sea-level query disagrees with the terrain surface.

Native captures and timing artifacts are retained locally under
`tests/environment-fixes/`. `visual-before/` is the previous release;
`visual-after/` is an intermediate candidate before the final coordinate/pacing
corrections. Capture dimensions can differ if the live desktop changes; inspect
the actual images and report dimensions rather than assuming a matched panel.

The initial timing candidate remained uneven, so it is retained as diagnostic
evidence rather than accepted performance. Final measurements follow below.


## Timing investigation and selected defaults

The initial timer-limited candidate was rejected: the coast route fell to
50.16 app FPS (p95 29.06 ms) with Vsync, and disabling Vsync still yielded only
52.44 FPS. The timer limiter is therefore disabled by default; Vsync remains.
With limiting explicitly disabled on the otherwise final candidate, the same
short routes measured:

| Route | Mean app FPS | p95 ms | p99 ms |
|---|---:|---:|---:|
| coast | 60.00 | 17.40 | 18.78 |
| mountain | 59.34 | 17.34 | 32.18 |
| forest-cockpit | 59.14 | 17.06 | 33.40 |

These short runs used 15 seconds warm-up plus 15 seconds sampling, an application
framebuffer of 2880 × 1800, and world scale 0.5. They ran on AC power. The earlier
baseline sequence ran on battery and must not establish a controlled percentage
speed-up. Baseline cockpit p95 was 26.18 ms versus 17.06 ms here, but the power
change and short samples constrain that comparison. Mountain/cockpit p99 still
exceeded 25 ms in this sequence; occasional hitches remain.

Full final-source unit results: **884 passed, 0 failed, 1 ignored**. The final package evidence follows below.


## Final packaged build

The final arm64 app passed `codesign --verify --deep --strict`. Its executable
SHA-256 is `e07efb9cb9852f89bc261737c4dad64fa53ea325b182cbba0ceb4214f94c2dd1`.
Source/executable provenance is saved in
`tests/environment-fixes/delivery-provenance.json`.

The production-default build completed a moving mountain/shoreline run from
Cartesian x/z (292700, 301150) to approximately (292700, 317317), covering about
16.2 km at scripted 180 m/s. There were 20 seconds warm-up and 70 seconds sampling,
with 2880 × 1800 application framebuffer and scale 0.5 on AC power. No compiler
or second diagnostic renderer ran during timing.

- Mean app rate: **59.92 FPS**.
- p95 interval: **17.16 ms**; p99: **17.93 ms**.
- Six frames exceeded 25 ms, including one exceeding 50 ms. Isolated stalls remain.
- No runtime panic or shader validation error; zero discarded simulated or wall
  seconds in this paused, scripted rendering/streaming study.
- The route crossed multiple 4 km water-publication boundaries. End-of-run entity
  count was 15,460 and mesh count 2,618. These are retained asset/entity counters,
  not a complete GPU-memory measurement.

This is evidence of the final renderer's moving-stream performance, not a full
interactive-flight or every-coastline acceptance matrix. Final lake, mountain
and cockpit captures are under `tests/environment-fixes/visual-delivery/`.

Final lake, mountain and cockpit images were inspected. The lake surface is
continuous in the captured shore views, with smoother aircraft/terrain edges and
no enlarged camera-dither grain. Coastline shape still reflects the finite
source/mask resolution; this update does not add survey-grade shoreline data.
