# Atmospheric rendering implementation results — 2026-10-20

The focused rendering changes are packaged in `Nord Flight.app`. The largest
confirmed defect was an inactive menu camera disabling environment-map reuse
and six-stage filtering during normal flight. Fixing the active-view gate
produced repeatable cockpit and BVR gains without changing simulation outputs.
The 60 FPS / 16.67 ms p95 target is **not met in every workload**.

## Delivered

- Exact resource/layout identity caches for atmosphere and probe bindings.
- Static scattering keys separated from camera-dependent settings; exact paused
  sky/aerial lookup reuse with jitter/custom-projection fallbacks.
- A dedicated canonical indirect-light probe and matching inverse sampling,
  retaining the current-camera visible sky path and existing generation guards.
- Active extracted-camera counting, so inactive menu cameras no longer force
  full environment filtering. Multiple active views retain the fallback.
- Explicit opaque rigid-frame batching within attachment/material/layer/shadow
  groups. Animated controls, clickable parts and damage pieces remain separate.
- Complete owned display snapshots, short queue locks, latest-revision/dirty-region
  merging and bounded buffer recycling; GPU writes occur outside CPU locks.
- Developer reference and frame-latency switches. Shipping VSync and default
  frame latency remain unchanged.

The requested source analysis and implementation documents are
`ATMOSPHERIC_RENDERING_PROBLEMS.md` and `ATMOSPHERIC_RENDERING_FIX_PLAN.md`.

## Matched release comparisons

M1/Metal, 1440 × 900, default preset, 10-second warmup and 25-second sample.
Pair 1 ran baseline then candidate; the targeted repeat reversed that order.
No compilation ran during measurements. AC was restored before this matrix.
Background Spotlight and macOS services were active and host conditions varied;
both pairs are reported rather than selecting the fastest candidate sample.

| Workload | Pair | Baseline FPS | Candidate FPS | Baseline p95 ms | Candidate p95 ms | >50 ms frames, before → after |
|---|---:|---:|---:|---:|---:|---:|
| Paused cockpit | 1 | 31.52 | 48.01 | 40.65 | 39.69 | 12 → 7 |
| Paused cockpit | 2 | 41.85 | 59.84 | 34.38 | 17.20 | 5 → 0 |
| Live BVR cockpit | 1 | 36.08 | 62.17 | 31.63 | 26.74 | 1 → 19 |
| Live BVR cockpit | 2 | 42.39 | 59.28 | 34.22 | 17.54 | 7 → 3 |
| Terrain streaming | 1 | 23.49 | 44.11 | 74.53 | 33.07 | 81 → 13 |
| Terrain streaming | 2 | 43.64 | 46.30 | 24.21 | 44.82 | 0 → 49 |

Cockpit/BVR average-FPS improvement repeats. Tail latency remains variable;
terrain's second-pair p95/p99 regression is unresolved. This build is not a
claim of universally smoother terrain streaming. No further broad diagnostic
campaign was run, following the request to wrap up promptly.

Full p50/p95/p99, >25/>50 ms counts and both pairs are retained in
`tests/atmospheric6/final/accepted-summary.json`, alongside raw run/process reports.

## Evidence for the root cause

The first candidate still recorded 993 paused-cockpit filter bypasses and no
source writes during sampling. The gate counted all `Camera3d` components,
including an inactive main-menu camera. Bevy removes `ExtractedCamera` from
inactive cameras, so that component identifies views that actually submit.

After the correction, the repeat paused cockpit recorded 1,496 reuse hits,
zero source writes and zero bypasses. Live BVR used complete six-face cycles
instead of full filtering every frame. The new ECS test covers inactive menus,
non-3D cameras, multiple active views and deactivation.

The initial mesh-allocation gauge fell from 1,548 to 1,420 with batching;
reserved slab capacity stayed unchanged. This is not a draw-call measurement.

## Correctness and visual checks

- **384 application tests**, **12 vendored renderer tests**, and **3 packaging /
  Cargo-cache tests** pass. `git diff --check` passes.
- **11 numerical comparison files match** the preserved executable after
  excluding timing fields: flight, AI, combat, missiles, audio and lunar outputs.
- Native F-22 cockpit, damage/fire and forest captures show no visible
  regression. Their p99 absolute color-channel difference is zero after masking
  only changing top-right telemetry. Small local differences remain around
  rasterized geometry; this is not a claim of pixel identity.
- **1,200 consecutive frames per build across 100 scripted transitions**:
  complete coverage, no scene-state mismatch and zero automated large-difference
  flags. All ten contact sheets were inspected with no visible corruption.
  Day/night, weather, graphics-preset, map and camera changes are covered.
- The two-minute mixed-scene check exits successfully. Images stabilize at 75;
  entities/meshes vary with effects and terrain, then stabilize after the scene
  change. Resident memory shows no rising trend during this short run; this is
  not a long-duration leak proof.
- One- and two-frame latency overrides were sampled. Bevy's existing default
  is already two frames. Variable results and unmeasured input latency do not
  justify changing it.

All temporary transition screenshots were compared before pruning. The report,
all metadata, contact sheets and 150 representative full-resolution images remain;
about 2.1 GiB was reclaimed. Cargo cache remains below the existing 12 GiB policy.

## Boundaries and remaining work

Exact shadow-depth reuse was not enabled: pooled cascade-depth ownership and all
caster inputs have not been proven stable. Cascades, resolution, distance and
filtering are unchanged. Numeric displays and label atlases were already opaque;
meaningful HUD, glass, radar and effect alpha remains intact. No visual-quality
reduction was used.

The next priorities are terrain tail-latency/submission behavior, then a valid
GPU execution / CPU submission / presentation-wait breakdown. Built-in Bevy
GPU pass timestamps are unavailable on this Metal path. Metal capture status is
recorded separately below; unavailable counters must not be inferred from CPU
spans. No measured single-pass GPU bottleneck or engine/hardware limit is claimed.

The accepted app executable SHA-256 is
`00cc4152198b946146489d715898aa1ce2082e290b5fa34ec139b58aa611a76d`.
Packaging and strict signature verification succeeded. The preserved original
signed executable and source/asset hashes remain in
`tests/atmospheric6/baseline`.

The packaged executable also completed a native cockpit launch/capture and
exited successfully. The pre-existing user flight process was restored after
the isolated checks; its currently running executable is the older image until
the app is restarted.

## Metal capture status

The corrected Metal System Trace attempt completed for both paused cockpit and
live BVR, with 18-second recordings and bounded 10-second trace windows. Traces
are retained under `tests/atmospheric6/final/metal` (about 102 MiB together).
Their table inventory includes GPU intervals, application encoders and drawable
waits. A detailed per-pass/CPU-submission/wait attribution is **not completed**;
the request to wrap up diagnostics takes precedence over extending analysis.
No per-pass duration or dominant GPU pass is asserted from these recordings.
