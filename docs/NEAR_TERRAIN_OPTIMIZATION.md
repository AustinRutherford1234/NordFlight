# Near-terrain rendering and streaming — 2026-10-21

This pass targets low-altitude land/forest flight, particularly larger windows
and fullscreen. It changes terrain presentation only. Aircraft physics,
collision terrain, tree population, terrain tessellation, lighting, shadow
settings and graphics presets are unchanged.

## Source findings and implementation

The previous renderer could submit one forest mesh spanning a 2 km tile. Its
bounds kept all its trees eligible for a camera or shadow view even when only
a corner contributed. The experimental spatial partition is now enabled with
512 m groups. It copies existing triangle attributes without modifying vertex
positions, normals, colors or UVs; anchors and camera-relative transforms stay
the same. Existing Bevy camera and light-space culling can reject smaller groups.
This is culling granularity, not tree removal or lower-detail vegetation.

The previous publication limit was one whole terrain tile per frame. A tile
could contain a substantial forest mesh, so a tile count did not bound mesh
preparation work. Streaming now stages individual meshes, capped at four meshes
and 2 MiB of source attribute/index data per update. An oversized developer
reference mesh can pass alone to guarantee progress. This byte count estimates
source mesh data; it is not a measurement of actual GPU traffic or execution.

All replacement pieces remain hidden until every required GPU mesh is ready.
The complete layout is then published with current camera-relative transforms.
Old coverage remains visible during preparation. Retired geometry is hidden at
publication and reclaimed with the same per-frame limits, rather than freeing
all mesh assets together. Cleanup and staging do not share a frame. Another
replacement does not start until cleanup finishes, bounding ownership to the
active layout and one replacement/retiring layout. Missing and retiring tile
orders are stable.

Final review tightened the cleanup guard when a completed asynchronous cell is
already stale: even if reclaiming emptied the queue, another layout cannot be
retired with a fresh budget in the same update. The staged-publication test
covers both single-mesh and multi-frame retirement with a stale completed cell.
This small guard correction follows the timing comparisons below; those FPS
samples are not a new benchmark of the guard correction.

The internal developer overrides remain available:
`NORD_TERRAIN_LARGE_BATCHES=1` retains unpartitioned reference forest meshes;
`NORD_TERRAIN_BATCH_SIZE` selects a partition size. Shipping graphics controls
and visual-quality settings do not change.

A second controlled experiment adds `DepthPrepass` to the flight camera only
when the internal `NORD_TERRAIN_DEPTH_PREPASS` environment switch is present.
It is **off by default**. Same-executable AC comparisons did not show a benefit:
external 40.01 → 40.01 FPS; cockpit 34.44 → 33.67 FPS. It is not enabled in the
shipping path. The experiment preserves the existing graphics preset, geometry
and material shaders, but adds a geometry pass; no benefit is inferred from
the technique alone.

## Validation scope

The installed optimized executable and source hashes were preserved in
`tests/optimization7/baseline` before edits. The focused runner is
`tests/optimization7/validate.py`; it pauses and restores an existing game
process instead of discarding the user's flight.

- All **398 application regressions pass**. The new checks cover staging byte
  and mesh limits, oversized-reference progress, dense forest partition bounds,
  complete publication and deferred retirement. Existing tests compare every
  original forest triangle attribute and verify first-visible-frame transforms.
- The seeded flight-study output is exactly equal to the preserved executable
  after the existing timing-field exclusions (`flight/equivalence.json`).
- Matched native F-22 external and cockpit forest captures are pixel-identical
  outside the masked top-right telemetry area (`visual/comparison.json`).
  Both images were inspected. These are posed native renders, not a proof of
  frame-by-frame equivalence throughout streaming or manual flight testing.
- A 120-second streaming sample, after 10 seconds of warmup, completes without
  renderer errors. The route moves 28 km, through forest and then coastal water.
  Mesh counts range from 1,405 to 1,611 and entity counts from 8,338 to 8,544;
  neither grows continuously. Peak sampled RSS is 789 MiB and falls as the route
  leaves forest. This is a short stability check, not a long-duration leak proof.
- Observed streaming maxima are **2,094,708 source mesh bytes and four meshes
  per update**, within the configured 2 MiB/four-mesh limits.
- Release packaging and strict ad hoc signature verification pass. The native
  runs use the executable inside the signed app; the Cargo cache remains under
  the existing 12 GiB policy.
- After the final cleanup-guard correction, all 398 regressions pass again.
  The accepted release is rebuilt/signed, its flight-study output still matches
  the baseline, and its native cockpit capture is again pixel-identical outside
  telemetry. Exact accepted source/executable identities and launch records are
  saved under `accepted/`. No second timing matrix or long soak follows that
  guard correction.

## Measured results

M1/Metal, default preset, VSync unchanged, **2880 × 1800 native fullscreen**,
50 m above terrain and a reproducible 220 m/s moving camera path. The flight is
paused while the diagnostic path moves it, isolating rendering/streaming from
physics. The scene is Gripen; static visual comparisons additionally use F-22.
There is no concurrent compilation. Warmup is 10 seconds and each sample is
25 seconds. Two pairs alternate baseline/candidate order; one targeted cockpit
repeat follows because tail latency varied. All these initial runs used battery
power. Both versions' raw power and process records are retained.

| View / pair | Baseline FPS | Candidate FPS | Baseline p95 ms | Candidate p95 ms | Frames >50 ms, before → after |
|---|---:|---:|---:|---:|---:|
| External 1 | 39.96 | 39.99 | 34.20 | 34.00 | 1 → 0 |
| External 2 | 39.96 | 39.99 | 34.19 | 34.22 | 0 → 0 |
| Cockpit 1 | 32.80 | 34.53 | 34.70 | 34.57 | 23 → 16 |
| Cockpit 2 | 34.32 | 34.48 | 34.58 | 34.68 | 3 → 6 |
| Cockpit repeat | 32.42 | 34.21 | 35.54 | 34.82 | 23 → 11 |
| Cockpit, final AC pair | 34.50 | 33.08 | 34.53 | 34.61 | 13 → 10 |

Across the three battery cockpit pairs, average FPS rises **33.18 → 34.41
(3.7%)**. Frames above 50 ms total **49 → 33** across equal 75-second sample
durations. Mean of the per-run p95s is 34.94 → 34.69 ms; mean of the p99s is
46.87 → 48.93 ms. Tail improvement is mixed: one pair has a worse p99, while
the targeted repeat improves it. The external view has no meaningful FPS gain.
After AC power was connected, a separate final pair did **not** reproduce the
average-FPS gain: 34.50 → 33.08 FPS, with almost unchanged p95. Thus a consistent
FPS improvement is **not established**, and the low-altitude slowdown remains
unresolved. The retained structural benefits are exact geometry culling and
enforceable streaming/retirement limits. They are not a 60 FPS claim. No quality
reduction was introduced to produce a more favorable result.

Forest vertices in camera-visible external batches fall approximately **66%**;
the corresponding shadow-cascade count falls **28%**. Cockpit reductions are
only about **2% and 11%**, respectively, because its forward view includes more
distant forest. These are source-vertex counts in CPU-visible batches, not draw
calls, GPU timings or a count of pixels shaded. More groups also add entity and
submission bookkeeping; fewer submitted vertices do not guarantee faster frames.

The existing GPU diagnostics expose no valid positive per-pass GPU durations
in these runs. They cannot establish the remaining dominant GPU pass. CPU
preparation spans likewise do not account for the complete frame, presentation
or GPU waits. A full-resolution pixel/shadow/submission timeline is still needed
before choosing another major rendering change. No hardware or engine ceiling
is inferred from these results.

Initial terrain setup remains synchronous; these streaming limits apply to
replacements during flight. Complete-layout visibility publication is still an
atomic operation, rather than a fixed-budget visibility change. Those are
remaining potential spikes. Native coastal captures also show an existing
abrupt coastal profile; matched baseline/candidate captures are identical, so
this pass does not claim to repair that independent terrain-shape issue.

Raw reports, all samples, source/executable identities, native images and the
bounded validation runner are in `tests/optimization7`. Only the baseline
executable snapshot is retained alongside the installed accepted candidate.
