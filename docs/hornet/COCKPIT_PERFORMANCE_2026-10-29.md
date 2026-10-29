# F/A-18F cockpit performance investigation — 2026-10-29

## Outcome

Implemented the two approved updates: identical rear display repeaters share their front display's live texture/material, and the ejection visual system skips inactive work, hiding actors once when the sequence resets. Aircraft geometry, rendering quality, and flight fidelity are unchanged.

The paired check measured a **6.8% reduction in the cockpit CPU span**, but **no meaningful overall frame-rate improvement**. The severe cockpit-only slowdown reported by the pilot is **not resolved or fully reproduced**. The worst frame-time percentile did not improve. These are measured housekeeping improvements, not a claim that the cockpit lag is fixed.

## Scope, permissions, and conditions

The pilot reported delayed controls/low FPS in fullscreen normal quality, particularly low over terrain and on carrier approach, with a smoother external view. Approved work comprised a cockpit/external comparison, a five-minute soak, then the two changes above and two 30-second before/after checks. The pilot subsequently approved one additional 30-second live carrier-flight profile; its results are recorded below.

- Hardware: MacBookAir10,1, Apple M1, 8 GiB unified memory; Metal adapter Apple M1.
- Fullscreen: 2880 × 1800 physical pixels, normal quality (`quality: false`), VSync. Initial windowed work used 1440 × 900; fullscreen renders four times as many pixels.
- Power: AC power. No thermal/performance warning was returned by the recorded system checks; this does not exclude throttling.
- Baseline source: `0fbd8c60042a109b52e8e51e0dab9195936981d3`, with a diagnostic-only carrier approach script added for the fullscreen comparisons.
- Runs were sequential, without simultaneous compilation. The harness isolates Nord Flight, uses caffeinate, and records process RSS/CPU and the binary hash. Other user applications were not suspended.

The carrier rendering script repeatedly moves the aircraft along a representative final approach, with the carrier advancing normally, but **pauses aircraft physics and collision simulation**. It exercises the view, streaming, displays, and visual updates. Its assigned `f.elapsed` follows wall time; its reported simulation rate is therefore not evidence of live physics speed. The terrain-stream script also pauses physics.

Frame intervals are measured between application updates, including scheduling and VSync, rather than a display-presentation trace. Mean FPS can be skewed by long intervals; percentiles are reported alongside it. CPU spans can overlap and must not be added. This profiler did **not** return usable per-pass GPU elapsed times, so no exact GPU bottleneck is assigned.

## Fullscreen comparisons before the changes

Each short case used eight seconds of warm-up and twenty seconds of sampling. These individual cases were exploratory comparisons, not repeated statistical trials.

| Case | Mean update FPS | p50 interval, ms | p95, ms | p99, ms |
|---|---:|---:|---:|---:|
| Terrain, external | 26.60 | 18.87 | 34.14 | 54.05 |
| Terrain, cockpit | 43.18 | 19.98 | 34.11 | 34.77 |
| Carrier approach, external | 59.79 | 17.20 | 26.41 | 26.85 |
| Carrier approach, cockpit | 57.39 | 22.16 | 27.53 | 29.10 |
| Carrier approach, cockpit, diagnostic glass removal | 58.23 | 13.19 | 27.35 | 28.06 |

The terrain cases reproduced a substantial fullscreen rendering cost in both views. The carrier view showed only a modest cockpit penalty in this workload, rather than the severe cockpit-only behavior reported by the pilot. Removing diagnostic glass did not establish a useful improvement. That control removes code-created blended/additive meshes at startup; it is not a comprehensive removal of every imported transparent material. No glass or canopy change was shipped.

A separate windowed live cockpit run with actual physics active measured 59.63 mean update FPS, p95 17.30 ms, p99 18.53 ms, and 0.9936 simulated seconds per wall second. This does not establish fullscreen live-flight performance. The initial windowed soak was interrupted after the fullscreen clarification and is excluded from soak acceptance. An initial windowed profiling attempt set `NORD_BENCH_FPS_ONLY=0`; because the switch is presence-based, those zero counters are invalid instrumentation and were not used as evidence of zero work.

## Approved five-minute soak before the changes

Fullscreen normal-quality carrier cockpit rendering ran for **300.016 sampled seconds**, with 16,596 frames and no crash or hang (process exit 0).

| Measurement | Result |
|---|---:|
| Mean update FPS | 55.32 |
| p50 / p95 / p99 interval | 17.47 / 29.64 / 34.27 ms |
| Frames over 25 / 50 ms | 7,243 / 39 |
| Peak process RSS, including startup | 1,688.78 MiB |
| Median RSS, 30–90 / 90–180 / 180–300 s | 227.3 / 259.2 / 250.1 MiB |

Entity and mesh counts varied with terrain staging and did not show unchecked accumulation in this run. The final entity sample was 12,176 entities and 2,037 meshes. Process RSS is **not total unified GPU allocation**. These observations do not prove the absence of a GPU-memory leak. Aggregate interval percentiles do not establish a frame-time trend over the soak. This was a **pre-change rendering soak**, not a post-change full-flight soak.

## Source findings and implementation

1. **Duplicate rear display work.** Three rear screens repeat front instrument pages. Previously each owned an `Instrument` and independent image/material, so the same page was rasterized and uploaded separately. A post-startup binding now shares the original material, removes the repeater's update component, and preserves the existing display geometry and shared brightness/power behavior. Five source instruments remain; three rear repeaters reference them. A regression constructs the actual Hornet cockpit and checks the sharing.
2. **Inactive ejection updates.** The visual system previously wrote empty labels and hidden actor transforms every frame, including computing orientations from default zero positions. It now returns while inactive, and hides actors/clears the label once on the active-to-inactive transition. A regression verifies finite transforms and visibility reset after an active sequence.
3. **Frame-dependent simulation timing remains separate.** The current main loop advances the configured simulation interval per rendered frame. Falling rendering rate can therefore slow simulation relative to wall time and amplify perceived control lag. This is a source-derived explanation to investigate, not a measured cause of this pilot's fullscreen cockpit-only issue. No timing-contract change was made in this update.

The enabled 20-second pre-change carrier profile measured 3.057 ms per application frame in the cockpit span and 0.617 ms in asset preparation. It recorded approximately 15.6 MB/s of upload bytes. These quantities include the profiler's scope and idle/update cadence; they are not GPU execution costs or an additive frame budget.

## Approved paired checks

Both runs used the same profiled fullscreen carrier rendering path, normal quality, ten-second warm-up, and thirty-second sample. The first used the saved pre-change executable; the second used the updated executable. Instrumentation was enabled in both runs (`NORD_BENCH_FPS_ONLY` unset).

| Measurement | Before | After |
|---|---:|---:|
| Sample duration | 30.007 s | 30.001 s |
| Frames | 1,466 | 1,479 |
| Mean update FPS | 48.81 | 49.28 |
| p50 interval | 17.79 ms | 17.26 ms |
| p95 interval | 33.48 ms | 33.34 ms |
| p99 interval | 62.97 ms | 68.97 ms |
| Frames over 50 ms | 20 | 21 |
| Cockpit CPU span, mean per frame | 3.613 ms | 3.367 ms |
| Asset preparation, mean per frame | 0.747 ms | 0.743 ms |
| Total upload bytes | 425,470,600 | 415,090,564 |
| Upload calls | 715 | 700 |

The measured cockpit CPU span fell 6.8%; mean update FPS increased only 1.0%, while p99 worsened. A single pair does not establish confidence intervals or a repeatable overall FPS gain. The span also includes frames when the display redraw cadence skips work. The five-minute soak and these instrumented checks have different sampling/profiling conditions and should not be treated as a single before/after series.

## Regression and remaining validation

All **631 automated tests passed**, including the new display-sharing and ejection-reset checks. The Mac development bundle is rebuilt and ad-hoc signed through `build-mac.sh dev`. Building and signing do not establish manual flight quality or solve the remaining slowdown.

The subsequently approved live profile below confirms rendering-rate-dependent simulation slowdown. It does not identify the responsible GPU pass or establish the cockpit/external difference during identical live flight. A GPU/presentation trace and a paired live external-camera control remain useful possible follow-ups, requiring separate diagnostic approval. No further runtime tests were performed.

## Subsequently approved live-flight profile

One additional run used the same 2880 × 1800 fullscreen resolution, normal quality, VSync, enabled counters, ten-second warm-up, and a **30.000-second sample**. The `carrier-live` diagnostic initializes the standard carrier approach once, with neutral pilot commands, normal Hornet FBW, and wind disabled; thereafter normal physics advances position, attitude, and velocity. It does not reposition, recycle, pause, or automatically fly the aircraft to the deck. Physics and collision counters confirm that their normal paths executed.

| Measurement | Live profile |
|---|---:|
| Application update frames / mean FPS | 1,198 / 39.91 |
| Mean / p50 interval | 25.06 / 26.26 ms |
| p95 / p99 interval | 34.40 / 35.17 ms |
| Frames over 25 / 50 ms | 599 / 1 |
| Simulation advanced during sample | 19.95 s |
| Simulation time / wall time | 0.6650 |
| Full simulation CPU span, mean per application frame | 0.1812 ms |
| Aircraft physics, total / calls | 211.689 ms / 4,788 |
| Collision, total / calls | 2.314 ms / 4,788 |
| Cockpit CPU span, mean per application frame | 3.3206 ms |
| Asset preparation, mean per application frame | 0.7551 ms |
| Display flush, mean per application frame | 0.3638 ms |
| Upload bytes / calls | 792,641,056 / 1,695 |

All recorded flight-state samples were unpaused, airborne, and uncrashed, with cockpit camera index 3. Near the sample start (wall time 10.134 s), the aircraft was 142.9 m above sea level and at carrier-relative z = 1,942 m. The final one-second state sample (wall time 39.585 s) was 93.2 m above sea level and z = 482 m. The aircraft approached continuously but did not reach touchdown during this test. The track and altitude were allowed to evolve naturally; the run is not evidence of correct manual landing handling or final deck-contact performance.

This live run reproduces sustained rendering slowdown with physically advancing flight. The average simulation CPU cost is less than 1% of the mean frame interval; measured collision CPU work is approximately 0.00193 ms per application frame and is nested inside physics/simulation. These averages make physics and collision CPU cost an unlikely explanation for the **sustained** approximately 40-FPS result in this workload. They do not locate individual frame spikes or exclude different costs during actual deck contact.

The simulation rate agrees with the source timing contract: 39.91 updates/s × 1/60 simulated second/update ≈ 0.665 simulated seconds per wall second. Unlike the prescribed rendering path, this is a real simulation-rate measurement. Slow rendering therefore also slows aircraft motion and control evolution by about one-third. Correcting that contract would require a separate bounded fixed-step/wall-clock accumulation change and appropriate flight/input regressions; no such behavior change is included here.

Compared with the earlier post-fix rendering-only pair, live mean update FPS was lower (39.91 versus 49.28), while cockpit CPU cost was similar (3.321 versus 3.367 ms/frame). Upload volume was higher (26.4 versus 13.8 MB/s), and display-flush CPU cost rose from 0.187 to 0.364 ms/frame. Live instruments change with physically evolving state. These are different trajectories and workloads measured at different times, not an isolated causal A/B test of physics or uploads. The result supports investigating rendering, display updates, and presentation scheduling; it does not prove upload bandwidth saturation or identify a specific shader. Usable per-pass GPU elapsed times remain unavailable.

The process exited normally after 46.04 seconds including startup/warm-up/cleanup. Peak process RSS including startup was 1,287.1 MiB; AC power was present and no system thermal/performance warning was reported. The existing Nord Flight process was resumed by the isolation harness. This follow-up changed only opt-in diagnostic code and this report; the installed app retains the previously packaged production fixes. The live diagnostic compiled successfully and ran once; the full 631-test regression result above belongs to the preceding production update and was not rerun for this diagnostic-only addition.

## Evidence and reproduction

Tracked harnesses:

- `tests/hornet-cockpit-performance/diagnose.py`: initial windowed comparison; its original soak was interrupted.
- `tests/hornet-cockpit-performance/fullscreen.py`: fullscreen matrix and approved pre-change five-minute rendering soak.
- `tests/hornet-cockpit-performance/paired.py before|after`: the approved pair, requiring a saved `paired/before` executable for the baseline.
- `tests/hornet-cockpit-performance/live.py`: the separately approved single live profile; full local results, state samples, binary identity, and process records are under `live/`.

Local generated evidence (ignored by Git) remains under `tests/hornet-cockpit-performance/`: `fullscreen/*.json`, process samples/logs, `paired/{before,after}.json`, binary identity/environment JSON, build logs, and `paired/regressions.log`. These scripts actively reset the diagnostic flight and should only be run within the authorized test scope. They are not part of normal gameplay.
