# Environment upgrade validation — 2026-11-03

This report distinguishes implemented behavior, measured performance, and remaining
acceptance work. The implementation is substantial, but the entire original plan
is not yet complete. In particular, a short route achieving 60 FPS does not
establish all-route acceptance.

## Delivery and provenance

The macOS app is `Nord Flight.app` in the repository root. It bundles all required
terrain/material files and works offline. The release uses Bevy 0.18.1 and Metal
on the current 8 GB M1 MacBook Air. The measured fullscreen application framebuffer is 2880 × 1800;
Balanced tests render the world at 1440 × 900 (scale 0.5), with UI at framebuffer resolution. macOS hardware inventory reports the built-in panel as 2560 × 1600; the reported test dimensions refer to the application framebuffer, not a panel specification.

The terrain assets total 246,499,281 bytes: two regional packs plus coarse global
elevation. They use approximately 93 m north/south height spacing and 46 m
land-cover spacing, resampled from the cited source products. They are not the
full-resolution GLO-30/WorldCover datasets, bare-earth surveys, or precise airport
survey data. See [architecture and configuration](ENVIRONMENT_TERRAIN.md) and
[source/license manifest](../assets/environment/SOURCES.md).

## Correctness checks

The full latest pre-material-blend Rust test run finished with **869 passed,
2 failed, 1 ignored**. All environment, terrain, publication readiness, vegetation
retirement and native UI routing tests passed. Two failures remain in concurrent
SR-71 work and have not been altered to force a green result:

- `aircraft::sr71::flight::engine::tests::ground_start_and_shutdown`
- `aircraft::sr71::flight::tests::engine_shutdown_and_asymmetry_have_no_free_power`

The native `--verify-environment` diagnostic initializes actual installed packs,
then simulates braked engine-off ground starts on the runway and apron and a
powered takeoff. Earth and sci-fi Earth both passed, with intact gear and no
crash. Earth lifted through 15 m AGL after 11.425 simulated seconds; sci-fi Earth
after 11.3375 seconds. These are basic contact/takeoff checks, not certification
of every landing or aircraft model.

Python checks cover adjacent raster sample coordinates and pack checksum
verification. Rust checks cover geographic poles/wrapping, interpolation, actual
pack borders, mountain render/contact agreement, airport grading, atmosphere
profiles, and quality-independent authority. The immutable process-lifetime
snapshot prevents in-flight planet switching; a restart is required.

## Earlier short-route measurements

`tests/environment-upgrade/accepted-ac/` contains the earlier AC-powered candidate
measurements: 20 seconds warm-up, 25 seconds sampling, sequential release runs
without concurrent compilation. These precede the final apron grade and material
blend corrections and are historical candidate results, not final-build claims.

| Workload | Mean app FPS | p95 interval ms | p99 interval ms |
|---|---:|---:|---:|
| Forest external | 60.00 | 16.99 | 17.39 |
| Forest cockpit | 59.96 | 17.03 | 17.44 |
| Coast | 64.85 | 25.71 | 26.22 |
| Mountain | 59.84 | 17.07 | 18.21 |
| Airfield | 59.96 | 17.05 | 17.69 |
| Live carrier | 59.60 | 17.14 | 21.26 |
| BVR combat | 59.68 | 16.97 | 17.22 |

The coast sample **missed** the p95 ≤18 ms and p99 ≤25 ms targets. Its mean app
update rate is not a measured presentation rate; irregular frame pacing can
produce a high mean alongside poor tails. The live carrier sample advanced
simulation at 0.9994× wall time and discarded no simulated work. The other terrain
routes primarily exercise paused rendering with scripted movement. BVR is the
first combat phase of the existing atmospheric exercise.

The original native-resolution battery baseline measured forest external
51.70 FPS and cockpit 31.81 FPS, with respective p95 22.55/35.50 ms and p99
36.07/55.01 ms. Different power state and render resolution prevent attributing
these differences entirely to the terrain changes.

A subsequent `final-ac/` directory was named before reading power state; macOS
actually reported battery power at 36%, discharging. That partial run was stopped
for a material-blending correction. Do not classify it as AC acceptance evidence.
Every run's process JSONL records actual battery/power state over time.

## Visual review

`tests/environment-upgrade/visual-frozen/` contains representative Earth and
sci-fi captures at ground, low flight, mountain, high altitude and orbital views,
including cockpit. These revealed hard material selector boundaries and overly
bright vertex colors. A subsequent correction cross-fades interpolated material
selectors and explicitly converts sRGB palette values into linear mesh colors.
Final `visual-final/` forest and mountain captures exercise the corrected shader. Representative screenshots are not a complete matched-altitude, grazing-light,
overcast and tile-transition visual acceptance matrix.

## Delivery status against the requested stages

| Stage | Delivered | Still required for full original scope |
|---|---|---|
| 1. Baseline and interfaces | Saved baseline; immutable surface service; active-body parameters; legacy fixtures | Broader matched-power baseline matrix and full GPU attribution |
| 2. Real terrain packs | Offline global/Sweden/Norway payloads, preparation CLI, masks, shared contact queries | Full-resolution pyramids, robust bare-earth/void processing and paging for larger packs |
| 3. Ground graphics and LOD | Six PBR materials, stochastic detail, projected sampled error, resource-aware publication | Conservative precomputed error bounds, neighbor geomorphing, complete visual acceptance |
| 4. Vegetation/assets | Stable clustered placement, opaque shared silhouettes, mesh LOD, sparse rocks/buildings | Production tree assets/variants, impostors, grass geometry and complete budget accounting |
| 5. Scenarios/planets | Geographic anchors, sea/lake sampling, configurable physics/biomes, sci-fi preset | Surveyed airport, full prelaunch pack-requirement flow, full-globe detail and extended mission validation |
| 6. Tune/package | Signed local Mac app, settings, scripts, documented measurements | Resolve missed frame-time targets and validate/package on actual Windows hardware |

## Remaining plan and acceptance gaps

- Quadtree errors are sampled estimates. Skirts hide cracks; continuous geomorphing
  and precomputed conservative tile error pyramids remain future work.
- Trees use shared opaque procedural silhouettes with near/far mesh LOD. Blender
  source assets, baked impostor atlases and nearby grass geometry remain future
  visual work. Representative settlement blocks are not detailed real buildings.
- The height data remains a resampled surface model. Full canopy/building removal,
  better local DTM and surveyed runway geometry/elevations are unfinished.
- The local renderer remains tangent-region based. The globe palette follows the
  preset, but continuous detailed rendering on every hemisphere is unfinished.
- All installed authority is pinned within a strict CPU load budget; larger future
  packs need paging. The configured GPU residency/staging budgets are not yet a
  strict accounting cap across every texture, vegetation and overlapping resource.
- Scenario anchors use real geography; pack requirements currently warn rather
  than enforce a complete prelaunch selection/download flow. Save revision
  migration and a graphical pack manager are not complete.
- Material arrays are currently fixed at 512²; `texture_detail` is reserved configuration and does not yet select larger arrays. Vegetation distance is additionally capped near 2.4 km plus an altitude allowance. High currently changes render resolution, geometric error, and shadows, rather than delivering the full planned visual tier.
- Separate GPU-pass bottleneck attribution and actual Windows shader/compression/
  performance verification remain unperformed. No Windows FPS claim is made.
- Existing Earth–Moon deterministic tests ran within the suite. Full interactive
  end-to-end Earth–Moon mission acceptance was not repeated.

## Final packaged-build 20-minute soak

The exact packaged arm64 executable SHA-256 is
`cb451c955428c6e849ea0746830a7fdaba3ea463e827768c902f736f61b3db33`.
`codesign --verify --deep --strict` passed. The local app occupies approximately
582 MiB. Source fingerprints and executable hashes are retained in
`tests/environment-upgrade/source-final.json`.

`tests/environment-upgrade/soak-final/` contains a completed 1,200.013-second
measurement after 20 seconds warm-up (about 1,225 seconds process telemetry).
macOS reported battery power throughout, decreasing from 29% to 11%; Low Power
Mode was disabled. There was no concurrent compiler or second diagnostic renderer.

There was no process crash, nonfinite-flight assertion, effect-pool overflow or
shader validation error. No sampled aircraft state was crashed. Camera, map,
pause, gear, combat and scenario-reset paths ran across two cycles of the six
atmospheric exercise phases.

Frame intervals **failed acceptance**: p95 **25.64 ms**, p99 **38.13 ms**. Mean app
update rate was 82.44 Hz, which is not a display-presentation FPS measurement.
The run recorded zero explicitly discarded simulated seconds but **22.43 seconds
of discarded/clamped wall time**. Consequently this does not satisfy a claim that
normal-speed simulation always kept real time. Scripted pauses and resets also
prevent interpreting its aggregate simulation ratio as ordinary live flight.

Resource evidence across repeated phases:

| Workload | First-cycle mesh range | Second-cycle mesh range |
|---|---:|---:|
| Cockpit | 2,363–2,363 | 2,363–2,363 |
| Moving terrain | 2,324–2,345 | 2,324–2,345 |
| Gear | 2,327–2,333 | 2,327–2,333 |
| Atmosphere | 2,327–2,327 | 2,327–2,327 |
| Effects | 2,333–2,339 | 2,327–2,339 |

Image asset count remained 167 after initial loading. Terrain entity maxima were
25,258 and 25,902 in the two moving cycles, varying with asynchronous staging and
vegetation cells rather than doubling on reset. End-of-run retained main-world
images occupied 396,231,752 bytes and meshes 218,221,166 bytes. Process RSS peaked
at about 1,350 MiB and ended near 320 MiB; compression and GPU allocations make
RSS an incomplete footprint measure. These observations support stable repeated
asset counts, **not a proof of bounded total GPU residency**.

The long run establishes runtime stability, not full performance or visual acceptance.

## Final-build short routes and coastal profile

The final-build short routes used 15 seconds warm-up and 20 seconds sampling.
Power changed during this sequence; each process JSONL contains the full timeline.
All listed routes missed the frame-time target. These results supersede any
suggestion that the earlier candidate's passing routes establish final acceptance.

| Workload | Mean app Hz | p95 ms | p99 ms | Power |
|---|---:|---:|---:|---|


| forest-external | 82.70 | 26.08 | 26.78 | Battery |
| forest-cockpit | 72.92 | 26.04 | 26.65 | Battery |
| coast | 56.84 | 25.79 | 57.10 | Battery → AC |
| mountain | 74.39 | 26.13 | 44.46 | AC |
| airfield | 73.28 | 26.02 | 46.44 | AC |
| carrier | 71.83 | 25.83 | 37.54 | AC |

The combat process exited before producing its report; that short result is
unavailable and must not be inferred from the other six routes. The completed
20-minute soak did include repeated BVR/effects phases. Final live-carrier
simulation rate was 0.9995× during the sample; its whole-run timing also includes
3.81 seconds of clamped wall time around initialization.

A separate AC-powered coastal profile (`profile-coast-ac/`) tested the existing
`NORD_FRAME_LATENCY=1` option. It still missed: p95 25.17 ms, p99 26.06 ms.
The production default was left unchanged. Measured CPU spans included:

- Background terrain build: 4.40 ms mean, 5.82 ms maximum (20 builds).
- Main-world terrain system: 0.0034 ms mean, 0.71 ms maximum.
- Asset preparation: 0.39 ms mean, 12.01 ms maximum.
- World update: 0.14 ms mean, 21.03 ms maximum.
- Terrain staging: at most four meshes / 1,421,536 source bytes in one frame.

The adapter identified itself as Apple M1 / Metal with BC compression support.
Mesh allocator slabs occupied 288,391,896 bytes in this snapshot. The profile
reported CPU encoding durations but no usable positive GPU-pass durations, so it
cannot isolate GPU saturation from presentation pacing. Asset/world spikes are
observed contributors; a Metal System Trace is still needed for reliable full
bottleneck attribution. Overlapping pass times were not summed.


The AC dense-forest route at approximately 12.4321° E, 59.2125° N used 20 seconds
warm-up and 20 seconds sampling (`dense-forest-ac/`). External p95/p99 were
26.26/32.27 ms; cockpit 26.44/42.13 ms. Both missed acceptance. Dense vegetation
therefore remains a required optimization workload, even though the long test
showed no runaway mesh/image count across repeated scenes.

The short BVR combat rerun on AC completed successfully (`combat-final-ac/`): mean app rate 89.94 Hz, p95 25.32 ms, p99 42.60 ms. The earlier missing-report run remains excluded.

## In-game world selector follow-up

Added Earth/Sci-fi Earth buttons to flight setup and the Escape pause menu.
World changes restart into fresh setup with explicit preset arguments. Nine
menu regression tests pass. Native startup with those arguments displayed
sci-fi Earth selected in setup. Automated pointer input returned a computer-use
`noWindowsAvailable` error, so the actual click-to-restart interaction was not
verified end to end. The updated packaged executable SHA-256 is
`d8f5e359fb197512fdae200dd596c8109d7ba35a12d0885f4a024da4b89af6a2`.
Its signature verifies; timing results above belong to the earlier executable.
