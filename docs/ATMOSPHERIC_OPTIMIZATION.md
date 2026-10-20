# Cockpit and atmospheric optimization — October 17, 2026

## Scope and reference

This pass changes presentation only. The starting installed optimized executable
and source hashes are preserved in `tests/optimization4/baseline/`. It targets
cockpit/atmospheric FPS and transient scene-switch corruption, retaining physics,
AI cadence, instrument refresh, textures, mesh detail, shadows and effects.

The lighting scheduler follows the production technique of distributing sky
environment filtering across frames while keeping the visible sky current.
[Epic documents this approach](https://dev.epicgames.com/documentation/en-us/unreal-engine/sky-lights-in-unreal-engine#timeslicingoptimization).
Its platform timings are not predictions for this application. Bevy already uses
the lookup-table family of atmosphere methods described by
[Hillaire](https://onlinelibrary.wiley.com/doi/full/10.1111/cgf.14050); this pass
retains that model and its existing sample counts.

[Unity's SRP Batcher documentation](https://docs.unity3d.com/Manual/SRPBatcher.html)
also emphasizes persistent GPU material data and avoiding redundant render-state
changes. This informs the unchanged-assignment/storage audit; it does not imply
that Unity's batching implementation is being ported or that a renderer migration
is required. Existing cockpit visibility already hides internal displays outside
the cockpit, and many display/light faces already exclude shadow casting; those
behaviors are retained rather than removing visible geometry.

## Implementation

- Explicit graph edges order current sky LUT generation → atmosphere cubemap →
  source snapshot/downsampling → filtering/publication → main pass. Previously
  the cubemap-generation node had no dependency edges.
- Probe texture bindings follow actual GPU identities when LUTs are replaced,
  including quality changes and map/atmosphere removal and re-entry. Missing
  assets/pipelines are retried instead of unwrapped or treated as complete.
- An internal extracted `EnvironmentMapScene` component versions exact sky
  inputs and discontinuities. Camera rotation remains an input because the
  current atmosphere LUT basis depends on it. Scenario/camera/map/preset cuts,
  large camera teleports and abrupt sky changes cancel pending work.
- An immutable downsampled source feeds separate pending diffuse/specular maps.
  Six face stages use original samples/mips. Both complete maps are copied into
  the published textures together before main rendering. Initial scenes and cuts
  complete a full update; previous complete maps survive pipeline preparation.
- Stable inputs avoid source generation, redundant downsampling and filtering.
  Scratch resources persist until their front texture identities change.
- Ambient/fog and control-surface assignments avoid unchanged writes. Cockpit
  instruments retain existing display resolution and refresh behavior.

Developer-only switches: `NORD_ENV_STAGES=1|3|6` (default six),
`NORD_ENV_LEGACY_ORDER=1` (reproduction only), `NORD_PRESENT_MODE=no-vsync`
(pacing diagnosis only). Existing cache-disable switches remain available and
disable sliced filtering through the original bypass path. Normal presentation
remains AutoVsync. None of these switches alters simulation timing.

## Final validation and delivery

The final source is frozen with 237 source/asset hashes at
`tests/optimization4/candidate/source-hashes.json`. Release executable SHA256:
`9450721bf5aefc2636b8058b959256bc441cdac0dcbd97223eb40795a1a83a94`.
Cargo verified the release build successfully; its artifacts were already
current, so this verification took 0.90 seconds without a redundant rebuild.

The final application suite has **299 passing tests**; the vendored engine suite
has **10 passing tests**. All nine release diagnostic JSON reports match the
preserved previous optimized release after the existing explicit timing-field
exclusions. Both AI CSVs are byte-identical. See
`tests/optimization4/release-physics-equivalence.json`.

Both the corrected development pair and the final release candidate/full-update
reference pair cover 100 transitions and all 1,200 frames per configuration,
including both graphics presets. The release pair has matching scene metadata
in every frame, no missing frames and zero large-image review flags. Inspected
contact sheets show no recurrence of the terrain flash. These are scripted
transition checks, not a claim that every possible camera path is artifact-free.
Results: `tests/optimization4/release-transition-review/comparison.json`.

Native captures reproduced the reported full-screen corruption. Ground/tree
batches created by asynchronous terrain streaming used `Transform::default()`;
their real `WorldObject` transform could be applied only on the following frame
because terrain and world updates were scheduled without a dependency and used
deferred commands. Tile-local geometry therefore appeared around the camera for
one frame. The reproduced frame differs from the matched full-update reference
over 63.4% of pixels by more than 25 channel levels.

Terrain and tree meshes now receive their correct current camera-relative
translation and rotation when spawned. The streaming regression verifies this
before any `update_world` system runs. This fixes the demonstrated geometry
defect independently of the separate atmospheric graph/lifetime corrections.
Final captures include every frame of each twelve-frame transition window.
Before/after evidence is retained as `tests/optimization4/evidence/terrain-before.png`
and `terrain-after.png`. Intermediate development PNG sequences were retired
after final validation; final release sequences and metadata remain available.

![Before: one-frame streamed-tree corruption](../tests/optimization4/evidence/terrain-before.png)

![After: correctly positioned first-frame terrain](../tests/optimization4/evidence/terrain-after.png)

These are transition captures; their on-screen FPS includes capture overhead
and is not the timing source for the uninstrumented render comparison below.

## Matched release performance

M1 MacBook Air, 8 GiB RAM, AC power, 1440 × 900, original AutoVsync,
default preset (FXAA, no MSAA, 1024 shadow setting). Each workload used two
baseline and two candidate runs, alternating A/B/B/A, with 15-second warmups
and 30-second samples. Values below average the two run-level measurements;
percentiles are not pooled across runs. The higher preset was checked visually,
but this FPS table does not measure its performance.

| Workload | Before FPS | After FPS | p95 ms before → after | p99 ms before → after | After frames >25 / >50 ms per 30 s |
|---|---:|---:|---:|---:|---:|
| Paused cockpit, quiet repeat | 31.25 | 59.80 | 34.05 → 17.12 | 49.75 → 18.97 | 6.5 / 0.5 |
| Live cockpit | 32.08 | 59.67 | 35.41 → 17.17 | 49.61 → 18.54 | 6.5 / 1.5 |
| Airborne external | 36.58 | 59.72 | 33.83 → 17.20 | 37.55 → 18.31 | 6.0 / 2.0 |
| Largest BVR cockpit | 35.88 | 59.70 | 33.84 → 17.28 | 38.24 → 20.41 | 9.5 / 0.5 |
| Low terrain flight | 40.26 | 59.79 | 33.85 → 17.10 | 34.89 → 17.40 | 5.0 / 1.5 |
| Atmospheric spacecraft, 60 km | 37.31 | 59.82 | 33.75 → 17.08 | 37.15 → 18.08 | 5.0 / 0.5 |

Live simulation/wall-clock rates improved respectively from 0.534, 0.610,
0.598, 0.671 and 0.622 to 0.994, 0.995, 0.995, 0.996 and 0.997. The configured
step and authoritative calculations are unchanged. This reflects less render
backpressure in native flight, rather than a measured acceleration of the
physics integrator itself. Paused simulation remains stopped.

Raw p50/p95/p99, stutter counts, process/power records and individual samples:
`tests/optimization4/release-matrix/` and `release-paused-quiet/`.
No task compilation or other benchmark overlapped these runs. An unrelated
Apple ANECompilerService used about one CPU core during part of the initial
matrix. Its presence limits attribution of exact speedup magnitudes. The
paused case was repeated after that activity ended and is the row used above;
the original paused result (31.74 → 58.34 FPS) remains in the raw matrix.
`quiet-paused-host.json` records the quiet repeat's process observations.

No measured workload regressed in FPS, frame percentiles or simulation rate.
The average approaches 60 FPS, but **the strict 16.7 ms p95 target remains unmet**:
candidate p95 is about 17.08–17.28 ms, with occasional larger stalls. This does
not establish an M1 or Bevy performance ceiling, and triggers no quality cut.

## GPU evidence and remaining costs

Xcode Metal captures compare the same candidate binary's full-update reference
with six-stage filtering in live BVR. Median normalized radiance intervals fell
from 22.62 to 3.10 ms and irradiance intervals from 12.48 to 2.50 ms. These GPU
event intervals overlap; **do not sum them into a frame time**. Captures are
separate from the unprofiled FPS measurements. Data lives in
`tests/optimization4/metal-reference/` and `metal-six/`.

Resource-creation events increased from 65.60 to 106.74 per observed frame in
those evolving traces. They measure turnover, not resident memory or CPU
allocations; different amounts of simulated activity prevent causal isolation.
This pass does not claim that GPU resource churn was reduced. Total draw-call,
CPU allocation and complete GPU-frame duration counters remain unavailable in
this harness where the platform did not expose them.

Ranked remaining work, if another pass is requested:

1. Identify occasional submission/presentation stalls responsible for p95/p99
   misses. A short no-VSync control did not improve pacing (paused 49.88 FPS,
   BVR 55.50 FPS), so shipping AutoVsync is retained.
2. Isolate staging/material/effect resource creation with equal simulated
   activity. The current event-count increase deserves investigation before
   claiming further memory/submission improvements.
3. Profile residual opaque fragment and cockpit draw work. In the Metal control,
   opaque fragment intervals remained about 2.7–3.0 ms, while lighting-filter
   peak intervals fell substantially. Retain visible canopy, shadow and display
   behavior when considering further batching.

No engine migration is justified by these results. Broad optimization and
additional diagnostic campaigns are outside this completed focused pass.

## Stability, correctness and acceptance limits

The focused soak completed **600.016 measured seconds** (616.984 process seconds)
with BVR, cockpit, terrain, gear, atmospheric spacecraft, effects and return to
BVR. It exercised pause and both 1/60 s and 0.01 s steps. Image assets stabilized
at 44; meshes peaked at 910 and returned to 771, below the initial 780. Entities
returned from a peak of 4,326 to 4,187, below the initial 4,196. Sampled RSS peaked
at 1,041.4 MiB and ended at 159.6 MiB. Two-second RSS sampling may miss short
peaks and excludes total GPU memory. No accumulating resource growth was found
in this workload. AC remained connected; macOS reported no thermal/performance
warnings, but physical temperature/frequency telemetry was unavailable.

The 299 application tests and ten engine tests cover scene/source invalidation,
resource epochs, immutable pending sources, complete publication, cancelled
updates/readiness, presentation refresh and first-frame terrain transforms.
Numerical equivalence covers nine JSON diagnostics and two identical AI CSVs;
it does not imply validation of every possible flight history. Final native
captures check cockpit/navball/HUD presentation, camera/map/scenario switches,
moving views, weather/daylight and preset changes. Manual interaction with
every cockpit button was not repeated in this pass.

The accepted executable is packaged and ad-hoc signed in `Nord Flight.app`.
`tests/optimization4/installed-build.json` records its packaged hash, build UUID,
backup and strict signature verification. Source/executable snapshots are bounded
to the preserved baseline and accepted candidate; the 12 GiB Cargo-cache policy
is retained. See `tests/optimization4/capture-retirement.json` for removal of
intermediate captures only.
