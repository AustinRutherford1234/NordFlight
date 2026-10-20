# October 16 optimization: evidence and validation

**October 17 status: diagnostics stopped at the user's request.** The current
delivery report is [OPTIMIZATION_REPORT.md](OPTIMIZATION_REPORT.md), with ten
completed comparisons, the passed 30-minute soak and explicit unfinished work.
No validation jobs remain running or queued. Historical plans below do not
indicate that further tests are active. The installed app has not been replaced.

## Baseline and measurement boundaries

Starting sources and executable are preserved in `tests/optimization3/baseline`.
The source archive includes embedded assets and the vendored dependency patch.
The instrumented baseline only adds opt-in measurement and scenario selection.
All ordinary simulation arithmetic and rendering are unchanged in that baseline.

Native window: 1440 × 900 pixels, scale override 1, AutoVsync, M1 / 8 GiB.
The main matrix preserves the existing default graphics preset (`quality:false`
in the raw reports): FXAA, no MSAA and a 1024 shadow map. The optional F9 preset
remains available, with its original samples, four-sample MSAA and 2048 shadow
map. Additional high-scattering/four-sample-MSAA benchmark controls are reported
separately. The existing command-line quality mode retains the startup 1024
shadow map; it is not a complete reproduction of pressing F9 in-game. Default
FPS must not be described as high-quality-preset FPS.
Frame samples include compositor scheduling; nested CPU spans must not be added
as if they were disjoint. Bevy 0.18 Metal does not expose useful GPU timestamps
through RenderDiagnosticsPlugin. Missing GPU timing, draw-call and allocation
counts are null rather than invented or reported as zero. Xcode traces supplement
the internal counters.

An existing game instance was discovered during the first cockpit/orbit timing
samples. Those initial samples are diagnostic-only and excluded from final speed
claims. Subsequent measurements suspend that instance temporarily and restore it;
its simulation state is retained. The runner never terminates the user's app.

## Initial bottleneck ranking

1. **Earth CPU shading and mesh preparation.** At the 60 km live spacecraft pose,
   Earth update averaged 5.975 ms per frame; the 40-second sample contained 3,879
   mesh modification events. The isolated BVR sample averaged 1.513 ms in Earth
   updates and 7,785 total mesh modifications. This justifies moving the same
   vertex-color calculations to a vertex shader, keeping geometry/interpolation.
2. **Rendering and synchronization.** BVR averaged 34.06 FPS (p95 33.92 ms), and
   the 60 km scene 32.19 FPS (p95 33.74 ms). Physics alone does not explain those
   frame intervals. CPU render-command timings are small; GPU traces and final
   matched comparisons are required before assigning the remainder to the GPU.
3. **Display and material churn.** BVR uploaded 356.3 MB of cockpit pixels in
   40 seconds, with 63,875 material modification events. Display rasterization
   averaged 0.324 ms/frame and upload flushing 0.177 ms/frame. Preserve display
   cadence, reuse buffers and transfer only changed regions.
4. **Lake geometry and bounds.** A CPU sampling trace identifies lake polygon
   tests and mesh-bound calculation among the application costs. Index lake
   edges conservatively and retain exact crossing arithmetic; keep explicitly
   maintained missile-trail bounds instead of recomputing them automatically.
5. **Full-fidelity simulation.** BVR's simulation span averaged 1.603 ms/frame.
   Existing independent-aircraft parallelism is retained. More workers require
   measured benefit; timestep, aircraft count and AI cadence remain fixed.

## GPU investigation

A five-second native Metal System Trace of the aligned development baseline
identified environment-map convolution as a major remaining GPU workload.
The exported active intervals are summarized in
`tests/optimization3/metal-short-baseline/aggregate.json`:

| GPU event / channel | Median interval per observed frame |
| --- | ---: |
| Environment radiance filtering / compute | 20.480 ms |
| Environment irradiance filtering / compute | 12.581 ms |
| Opaque pass / vertex | 11.336 ms |
| Transparent pass / fragment | 3.783 ms |
| Atmosphere LUTs / compute | 0.655 ms |

These are profiled event intervals, **not disjoint frame costs**. GPU channels
and nested/parallel work can overlap; adding this table does not yield frame
time. The capture is separate from uninstrumented FPS runs. There are 175
observed frames with each environment-filtering event, which establishes that
this work repeats rather than being only startup preparation.

The pinned engine's `light_probe/environment_filter.wgsl` filters the environment
with the original GGX radiance samples and 1,024 irradiance samples per texel.
Those samples and output resolutions remain unchanged. Increasing CPU worker
counts cannot eliminate this GPU workload. The narrow engine change in this
pass first reused the camera-independent transmittance/multiscattering LUTs
and environment-generation bind groups. A further exact source-write cache now
reuses completed environment convolution for an unchanged atmosphere-owned
source. All active atmosphere updates still run the original full convolution.
Generic maps and multi-view scenes retain their original dispatches. Actual
bound output IDs, complete binding dependencies and all prepared pipeline IDs
invalidate reuse; the registry is pruned and owns no textures.

## Correctness verified during implementation

- Application regressions: **297 passed**, in
  `tests/optimization3/static-cache-tests.log`.
- Engine cache invalidation/texture-aliasing regressions: **2 passed**, in
  `tests/optimization3/vendor-cache-tests.log`.
- Harness freshness, process-identity recovery and narrow comparison-exclusion
  regressions: **3 passed**.
- Development baseline/candidate comparison: all **nine** complete diagnostic
  JSON reports match after explicit timing exclusions. Both AI telemetry CSVs
  are byte-identical. See `tests/optimization3/dev-physics-equivalence.json`.
- A short same-process native stress run completed all twelve scripted workload
  transitions with finite flight states and bounded effect pools. This smoke
  test is separate from the required 30-minute soak.

Native Earth shader versus same-executable CPU-fallback captures are recorded in
`tests/optimization3/visual-initial-comparison.json`. Runtime shader compilation
has been checked; unit-test compilation alone was not used as rendering proof.
The final development captures are in `tests/optimization3/candidate-dev-final`.

The subsystem review and exact preservation boundaries are recorded in
[OPTIMIZATION3_SOURCE_REVIEW.md](OPTIMIZATION3_SOURCE_REVIEW.md).

## Additional unchanged-source optimization

`content-dev-image-equivalence.json`, `content-dev-repeat-equivalence.json`, and
`content-dev-repeat-vs-off.json` record same-executable checks. Cockpit and orbit
captures match exactly outside the dynamic FPS corner. The first atmospheric
and map captures had small differences, reproduced in enabled-versus-enabled
captures; repeated enabled captures match disabled captures exactly. Both map
images and the orbital spacecraft/Earth image were inspected. These development
checks establish runtime readiness; they are not the final optimized benchmark.

The short paused orbital probe used a five-second warmup and ten-second sample:
59.90 FPS, p95 17.12 ms with reuse, versus 38.83 FPS, p95 34.14 ms disabled. The
enabled sample recorded 599 result hits and zero source writes. It preserves the
original copied/downsampled intermediates and omits only identical convolution.
The final matrix uses the prescribed longer warmup/sample and alternating pairs.

Latest regressions: 297 application tests and eight engine tests pass, including
source-revision identity, resource/pipeline invalidation and missing-result cases.
The final cache snapshots the source version when downsampling copies it. The
producer may run between downsampling and filtering; reading the live source
version at filter time could otherwise mislabel an older intermediate. Its
ordering regression verifies the necessary final refresh after production stops.
The initial optimized validation was deliberately stopped to include this fix;
its `accepted/` results are superseded by the final run's fresh labels.
Final optimized comparisons and soak results will be appended after testing.

## Corrected final candidate: CPU acceptance

The corrected optimized candidate is SHA-256
`039700a75065de7893b101906babd7adece0608db7cebca007982023a1dbb982`.
Its sources match the 236-file snapshot in `tests/optimization3/candidate/`.
Final results use `tests/optimization3/accepted-v2/`; earlier `accepted/` was
stopped during profiling to include the copy/filter ordering correction.

All nine authoritative diagnostic JSONs match after the documented timing-only
exclusions; both AI telemetry CSVs are byte-identical. The initial flight timing
was sensitive to the power/thermal transition. Alternating ABBA controls on AC
power resolve that comparison:

| Optimized CPU workload | Baseline median | Candidate median |
| --- | ---: | ---: |
| Jet steps, diagnostic internal timer | 8.6405 s | 8.6637 s |
| Complete flight diagnostic process | 8.9383 s | 8.9458 s |
| Full BVR diagnostic process | 17.5291 s | 16.6224 s |

Jet throughput is effectively unchanged; the sampled BVR workload is about
5.2% shorter. These are CPU diagnostics, separate from visible FPS. All control
run physical outputs match. Raw results and per-run power/process metadata are
in `accepted-v2/release-cpu-pairs/`; the complete comparison is
`accepted-v2/release-physics-equivalence.json`. Render and soak acceptance remain
in progress until the final table below is populated.

## Final profiler evidence

`accepted-v2/metal-accepted/` records the corrected candidate in the cockpit.
Metal resource creation fell from about 79.4 to **52.413 events per observed
opaque frame**, approximately 34% fewer. This measures GPU resource turnover;
it is not resident memory, CPU malloc count or an FPS percentage.

The candidate's atmospheric capture still contains median active event intervals
of 21.791 ms for radiance filtering and 13.506 ms for irradiance filtering.
These overlapping profiler intervals are not additive frame costs. The orbital
capture in `accepted-v2/metal-orbit/` contains neither filtering event, while
retaining copy/downsampling, geometry, transparency and shadows. This confirms
the intended unchanged-source reuse independently of FPS measurements.

The five-second Time Profiler capture succeeded using a disposable copy of the
candidate with `get-task-allow`; that copy was deleted afterward. The canonical
candidate and delivered app receive no debug entitlement. The exported sample
summary is `accepted-v2/cpu-profile/summary.json`: 3,449 statistical samples,
with 32 samples lacking a backtrace. The capture includes synchronization and
Metal submission as well as terrain/visibility calculations and audio mixing.
For example, terrain-height and terrain-visibility inclusive weights are 165
and 158 ms, and the audio mixer has 69 ms of leaf weight. Inclusive stacks
overlap; these values must not be added or presented as frame wall time.

The nine-run worker study leaves four, six and eight compute workers near
16.65 seconds for the same BVR diagnostic, with identical physical outputs.
The default remains six. More workers do not remove the atmospheric GPU work.

In the final live-BVR pairs, cockpit texture-upload traffic is 8.945 MB/s before
and 0.842 MB/s after, approximately 91% less, while preserving display dimensions
and refresh behavior. Do not generalize this into a claim that every asset event
fell in every live scene: evolving combat advances at different wall-clock rates
and can have different active effects. Raw per-frame mesh/material/image event
counts are retained in `release-matrix/summary.json`; paused controls separate
presentation work from those changing simulation states.

## Native motion evidence

The CPU-reference and GPU ascent sequences have 24 overlapping captures with
exactly equal physical-state metadata; re-entry has 21. Their frame readbacks
are excluded from timing acceptance. The GPU ascent sequence additionally
crosses 100 km and continues into space. The inspected transition frame retains
the spacecraft mesh and lighting. The reference sequence is slower and needs a
longer wall-clock capture for its complete post-transition comparison.

The complete default-preset static comparison is
`accepted-v2/release-visual-equivalence.json`; same-executable cache-disabled
references are in `accepted-v2/cache-image-equivalence.json`. Cockpit, terrain
and maps match exactly outside the dynamic telemetry mask; combat differs by
at most one channel level. Small Earth pixel differences also occur across
same-executable captures. Inspect native motion and the images alongside the
numeric differences; neither test compilation nor a small average difference
alone establishes visual equivalence.

## Thirty-minute stability acceptance

The corrected candidate completed **1,800.010 measured seconds** (1,822.147 s
process wall time including warmup/startup), exiting normally. All twelve
workload types executed, including a return to BVR after the full sequence.
The normal finite-state and particle-bound assertions passed. There were 363
resource samples; the largest effect population was 110 against its 256 cap.

RSS was polled every two seconds: sampled peak **601.8 MiB**, final **153.2 MiB**.
This sampling can miss shorter startup peaks and does not measure all driver/GPU
memory. Image assets increased from 29 to 44 during the first five seconds of
startup, then stayed at 44 throughout every subsequent phase. Mesh/entity counts
peaked at 904/4,320 and recovered to 771/4,187 on the final BVR return, below the
initial 780/4,196. Thus the observed workload does not show accumulating image,
mesh or entity resources. This is a measured 30-minute result, not a proof that
every possible future workload has bounded memory.

Raw events, power/thermal snapshots and RSS samples are in
`accepted-v2/release-soak/`; `accepted-v2/soak-review.json` summarizes the check.
The run exercised simulation-time effects, resets, pause, map/camera changes,
landing gear, both planetary environments, warp and timestep changes. AC power
remained connected; macOS reported no thermal/performance warnings. Physical
temperature and clock-frequency telemetry was not available from this monitor.
