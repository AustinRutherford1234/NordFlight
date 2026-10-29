# Cockpit investigation and real-time flight correction — 2026-10-29

This follow-up fixes the confirmed simulation slowdown and removes hidden exhaust material updates. It does **not** establish a solution to the variable fullscreen rendering rate. It follows the [initial investigation](COCKPIT_PERFORMANCE_2026-10-29.md), using the same M1 MacBook Air with 8 GB RAM, 2880 × 1800 fullscreen, normal quality, and VSync.

## Approved investigation

The pilot approved three 30-second live-flight checks, preparation/testing of a bounded real-time timing fix, and one five-second Metal System Trace after warm-up. Runs were sequential without simultaneous compilation. The current Nord Flight process was suspended only during diagnostics and resumed afterwards. No extra soak was run. The later 30-second live run validates the separately approved timing correction.

### Three live-flight comparisons before the timing change

All three used the same executable, a standard carrier-approach start with neutral pilot commands, ten seconds of warm-up, and thirty sampled seconds. The frozen case retains the last cockpit texture pages after warm-up while physics, other visuals, and aircraft motion continue normally. All sampled flight states were airborne, unpaused, and uncrashed.

| Case | Mean update FPS | p95 / p99, ms | Simulation / wall time | Cockpit CPU, ms/frame | Upload bytes in sample |
|---|---:|---:|---:|---:|---:|
| Normal cockpit | 58.74 | 27.13 / 28.93 | 0.9786 | 2.783 | 1,065,847,788 |
| External camera | 65.41 | 26.44 / 26.82 | 1.0897 | 0 | 24,499,948 |
| Cockpit, frozen instruments | 39.90 | 34.11 / 34.80 | 0.6652 | 0.000151 | 14,244,044 |

Freezing cockpit pages removed virtually all measured cockpit redraw CPU time and reduced total upload bytes by 98.7%, yet the measured update rate worsened. This single ordered comparison cannot prove that uploads have no cost; it does show that eliminating them did not resolve the slow run. It does not justify lowering display resolution or cadence as a production fix.

The unmodified normal cockpit also ran substantially faster than the earlier 39.91-FPS live profile. These runs demonstrate variability, not an optimization gain. Under the old per-frame clock they also fly different distances during equal wall-time samples, so this is not an identical-trajectory camera A/B test. Update FPS is measured at the application loop and is not a display presentation-rate trace.

### Approved Metal capture before the changes

A disposable copy of the same executable was ad-hoc signed with the profiling entitlement. The delivered app was not given that entitlement. After fifteen seconds of warm-up, Metal System Trace recorded five seconds of live cockpit flight; approximately 282 frame labels were observed. The copy was deleted after capture, and the raw trace remains local (the GPU evidence directory is approximately 172 MB).

| Pass/channel label | Median active interval per observed frame |
|---|---:|
| Opaque fragment | 4.583 ms |
| Transparent fragment | 3.864 ms |
| Radiance convolution compute | 1.373 ms |
| Sky fragment | 1.328 ms |
| Irradiance compute | 1.226 ms |
| Opaque vertex | 1.227 ms |
| Near shadow cascade vertex | 1.091 ms |
| FXAA fragment | 0.983 ms |

**These intervals overlap. They must not be added into a frame budget.** Labels can aggregate work from multiple views, including the crew portrait. The profiled capture is not an uninstrumented FPS benchmark, and it did not capture every slow state seen in the separate runs.

There were 276 observed drawable waits for this process, with a median of 8.445 ms, p95 25.453 ms, and maximum 26.409 ms. Such waits can include VSync, scheduling, and rendering backpressure; they do not independently identify GPU saturation or a particular shader as the cause.

The resource export recorded approximately 29 `material uniform data 0` buffer creations and 79 staging-buffer creations per observed opaque frame. Allocated byte totals measure resource creation turnover, not resident RAM/VRAM or a leak. The vendored renderer's non-bindless material preparation creates a uniform buffer when preparing changed material data (`vendor/bevy_pbr/src/material_bind_groups.rs`). This corroborates the opportunity to avoid material changes for hidden exhaust layers.

## Implemented corrections

### Wall-clock flight scheduler

`src/sim_clock.rs` replaces one-configured-step-per-rendered-frame advancement with a bounded fixed-step accumulator. Flight, environment, combat, and combat effects consume the same ticks. Rendering rate no longer determines flight speed at ordinary frame intervals.

- Allowed warp remains the simulation speed control, including the existing fractional slow-motion rates and near-ground/high-warp restrictions.
- The saved timestep setting becomes a **maximum physics interval**, bounded internally by the existing 1/120-second limit. Hornet structural/aircraft integration still uses its existing smaller substeps. Existing saved values are retained; the UI and README explain the changed meaning.
- Fractional ticks carry between frames. Pause/resume, reset to an earlier simulation time, and scenario-generation changes clear fractional debt; the first resume/reset frame does not replay the previous interval.
- At most 0.1 seconds of wall time is accepted per frame. At most 256 fixed ticks execute per frame; exceptionally long host stalls or the smallest interval at high warp can discard excess time instead of causing an unbounded catch-up loop. Whole-run discard counts are included in benchmark diagnostics.
- Quiet high-warp coasts use wall time times allowed warp and retain their burn/entry/encounter guards. No aerodynamic, engine, FBW, actuator-speed, or carrier-force parameters were retuned.

### Hidden exhaust materials

At dry thrust, the Hornet shows its two recessed nozzle glow layers while 26 outer burner layers are hidden. Previously the visual loop still recalculated and marked all 28 materials modified every frame. It now skips hidden layers after updating their visibility. Each layer receives its complete current transform and color on the same frame it becomes visible. Visible plume animation, material formulas, geometry, lights, and heat haze are preserved.

The transition regression checks dry-to-wet visibility/material updates, retention while hidden, and refreshing to the current simulation time on reactivation. The earlier dry-glow test now checks the visible internal glow rather than the intentionally hidden outer material.

## Validation and limits

The full automated suite passes **637 tests**, zero failures, in 15.95 seconds. New tests cover:

- Scheduler rates 0.1× through 4× at 24, 30, 40, 60, 120, 144, and 240 update Hz, with multiple configured intervals.
- Matching actual Hornet position, velocity, attitude, rates, and fuel across those update schedules at normal and half speed.
- Shared flight/environment/combat/missile clock advancement, pause/resume, reset, bounded stall work, and planned-burn stopping in high warp.
- Hidden exhaust preservation and reactivation.

Initial test runs caught a catch-up limit that was too small for tiny physics intervals at lower rendering rates, plus legacy tests asserting the superseded per-frame timing or hidden-material behavior. The final limit and tests above replace those cases; failed-run logs are retained locally.

The updated, signed Mac development bundle was then checked in one thirty-second fullscreen normal-quality live carrier-approach run:

| Measurement | Earlier live profile | Updated live validation |
|---|---:|---:|
| Mean update FPS | 39.91 | 39.91 |
| Sample wall duration | 30.0003 s | 30.0006 s |
| Simulation advanced | 19.95 s | **30.0000 s** |
| Simulation / wall time | 0.6650 | **0.999981** |
| p95 / p99 interval | 34.40 / 35.17 ms | 34.14 / 34.45 ms |
| Full simulation CPU | 0.181 ms/frame | 0.275 ms/frame |
| Material modifications per update | 29.22 | **3.68** |

The corrected run advances more physics per rendered frame, as required to maintain real time. It preserves the fixed physical tick size rather than enlarging a physics step to fill the frame. All sampled states remained airborne, unpaused, and uncrashed. The aircraft passed above the carrier; this is not a manual landing, deck-contact, or catapult flight review.

The whole-run diagnostic records 1.787 seconds of discarded host time, including startup. The measured thirty-second window advanced thirty simulated seconds, demonstrating that startup stalls were not replayed into flight. No simulated-step-budget time was discarded.

Material modifications fell by about 87%, but **overall FPS did not improve in this comparison**. These samples have different physical trajectories after the timing change and are separated in time; they establish the corrected simulation rate and lower material-update count, not statistical confidence in an FPS gain. No post-change Metal trace or new long soak was run.

The Mac app was rebuilt with `build-mac.sh dev` and passed strict deep code-sign verification. The live diagnostic uses the same built executable as the package (stripped and signed for delivery). This automated evidence is distinct from the pilot's interactive flight review.

## Remaining investigation

The rendering issue remains variable. Current evidence supports these next targets without claiming measured gains:

1. **Presentation and host-load variation:** record window focus/occlusion alongside a matched live camera comparison and correlate drawable waits with frame scheduling. The current aggregate waits do not distinguish scheduling from GPU backpressure.
2. **Remaining material preparation:** visible glow and HUD/material changes still recreate non-bindless uniform buffers. Consider persistent material-buffer updates only after confirming resource identity/lifetime safety in the renderer; do not patch allocation ownership from counts alone.
3. **Fragment/render-pass work:** transparent and opaque fragment channels remain significant in the current trace. Identify the specific visible materials/views before changing shaders. Environment filtering and full-resolution postprocessing also remain measurable work, with no proven current gain from disabling them.

Existing crew-camera opt-out, portrait sizing, dense mirror-mip filtering, HUD coverage cropping, stable terrain anchors, and dirty-region display uploads are already present. They are not missing fixes. The frozen-display result argues against sacrificing instrument detail as the next step. Further runtime diagnostics require the pilot's approval.

## Reproduction and evidence

Tracked harnesses in `tests/hornet-cockpit-performance/`:

- `matrix.py`: the three approved thirty-second comparisons, using `carrier-live` and the opt-in `NORD_BENCH_CAMERA=external` / `NORD_BENCH_FREEZE_INSTRUMENTS=1` controls.
- `metal.py`: the separately approved five-second live Metal trace; its disposable profiling copy is deleted afterwards.
- `live.py timing-validation`: the thirty-second timing validation using a fresh output folder.

Local generated evidence includes `matrix/*.json`, `gpu/{profile.trace,aggregate.json,resource-summary.json,drawable-summary.json}`, `timing-validation/*.json`, and the final `timing-verified-regressions.log` / `timing-package.log`. Raw traces, logs, binary copies, and process/environment exports stay ignored by Git. A curated machine-readable summary is [COCKPIT_TIMING_EVIDENCE_2026-10-29.json](COCKPIT_TIMING_EVIDENCE_2026-10-29.json).
