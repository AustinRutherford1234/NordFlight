# Nord Flight optimization report — October 17, 2026

**Later cockpit/atmosphere pass completed:** See
[ATMOSPHERIC_OPTIMIZATION.md](ATMOSPHERIC_OPTIMIZATION.md) for the current installed
build, near-60 FPS focused results and reproduced/fixed scene-transition flash.
The measurements and pending-status notes below remain the historical general
optimization report.

**Installation update, October 17:** Following the user's request to implement
the new build, the preserved optimized candidate was installed into
`Nord Flight.app`, stripped and ad-hoc signed. Strict/deep signature verification
passed, and the native app launched with its `NORD / Gripen Flight` window.
All 236 recorded source/asset hashes matched the compiled candidate before
installation. The prior executable is preserved at
`tests/optimization3/baseline/gripen-flight-starting-app`. Installation hashes
are recorded in `tests/optimization3/installed-build.json`. No new benchmarks
were run. The delivery-state section below records the earlier report-time
status; installation and launch are now complete, while remaining validation
and artifact pruning are still unfinished.

Diagnostics stopped at the user's request. No further benchmark or build is
running. This report covers completed implementation and measurements; it does
not mark the original exhaustive validation plan as complete.

## Results

The largest gains are in orbital flight and planetary map views. Cockpit FPS
improved only slightly, and the overall 60 FPS goal is not met. Graphics settings,
mesh detail, texture resolution, effects populations and simulation fidelity were
preserved; no automatic quality reduction was introduced.

These are matched optimized baseline/candidate runs on the M1 MacBook Air,
1440 × 900, using the existing default graphics preset: FXAA, no MSAA, 1024-pixel
shadow map. They are not measurements of the optional F9 quality preset. Each
completed comparison used A/B/B/A order, a 20-second warmup and 40-second sample
per run, with two runs of each executable and no concurrent compilation.
The completed comparisons used AC power.

| Workload | Baseline FPS | Candidate FPS | FPS change | p95 frame time before → after |
| --- | ---: | ---: | ---: | ---: |
| Runway, paused external | 37.39 | 39.34 | +5.2% | 33.80 → 33.79 ms |
| Cockpit, paused | 32.07 | 32.62 | +1.7% | 33.95 → 34.09 ms |
| Airborne external | 32.23 | 35.42 | +9.9% | 40.01 → 34.05 ms |
| Close combat | 34.52 | 36.61 | +6.1% | 33.92 → 33.99 ms |
| Largest BVR scenario | 32.51 | 36.71 | +12.9% | 35.56 → 33.94 ms |
| Terrain | 36.40 | 39.97 | +9.8% | 33.79 → 33.89 ms |
| Spacecraft at 60 km | 30.43 | 37.38 | +22.8% | 48.39 → 33.95 ms |
| Orbital flight | 31.94 | 59.71 | +87.0% | 40.56 → 17.16 ms |
| Earth map, paused | 37.25 | 59.69 | +60.2% | 33.89 → 17.15 ms |
| Moon map, paused | 40.78 | 59.89 | +46.9% | 33.58 → 17.05 ms |

Source: `tests/optimization3/accepted-v2/release-matrix/summary.json`.
Only completed two-pair comparisons appear above. Ascent had one completed run
per executable when stopped and is excluded. Descent, landing, maximum-warp
render comparisons and additional paused controls remain incomplete.

Near-60 average FPS does not satisfy the strict 16.7 ms p95 target: even orbital
flight remains slightly above that threshold. Its fraction of frames over 25 ms
fell from 83.87% to 0.38%. BVR p99 fell from 54.10 to 37.32 ms. The smaller
cockpit improvement means the principal cockpit bottleneck remains unresolved.

## Implemented optimizations

- Moved Earth surface, cloud and haze vertex-color calculations to the GPU,
  retaining the original formulas, vertices, interpolation and textures. At the
  60 km workload, the CPU Earth span fell from 6.03 ms to 0.0021 ms per frame.
  A CPU reference path remains available.
- Reused cockpit upload buffers and transferred exact changed rectangles while
  preserving resolution, refresh behavior and pending updates during GPU
  readiness/resize changes. Live BVR upload traffic fell from 8.945 to 0.842 MB/s,
  approximately 91% less.
- Avoided unchanged material, light, layout and selected transform writes;
  reused aircraft/decoy/trail storage and explicit conservative trail bounds.
- Added conservative lake-edge bins while preserving crossing arithmetic and
  edge order. Retained full aircraft integration, AI cadence, collision accuracy,
  seeded damage and double-precision orbital propagation.
- Added narrow caches to the pinned Bevy PBR dependency for camera-independent
  atmosphere LUTs and environment-generation bindings. Reused completed
  environment filtering only for unchanged atmosphere-owned sources with valid
  resource/pipeline dependencies. Copied-source revision tracking preserves
  producer/copy/filter ordering. Active lighting still receives full filtering
  at the original resolution and sample counts.
- Added bounded profiling/capture tools, stale-output rejection and interrupted
  process recovery. The existing 12 GiB Cargo-cache policy remains in place.

The worker study found essentially equal BVR diagnostic times with four, six
and eight workers, around 16.65 seconds. The default remains six. Additional
threads did not address the measured atmospheric rendering workload.

## Simulation correctness and CPU throughput

- 297 application tests passed.
- Eight vendored engine cache tests passed, including invalidation, readiness,
  texture identity and copied-source ordering.
- Three existing harness tests passed. Two subsequently added retention tests
  have not been executed and are not counted as passing.
- All nine optimized diagnostic JSON reports matched after explicit timing-field
  exclusions. Both AI telemetry CSVs were byte-identical. This covers the tested
  flight, AI, combat, BVR, missile, audio and lunar diagnostic paths.
- Alternating AC CPU controls measured jet diagnostic internal time at
  8.6405 → 8.6637 seconds, effectively unchanged. BVR process time decreased
  from 17.5291 to 16.6224 seconds, approximately 5.2% shorter. These CPU results
  are separate from render FPS.

Raw evidence is in `tests/optimization3/accepted-v2/`, including
`release-physics-equivalence.json`, `release-cpu-pairs/summary.json` and
`release-workers/summary.json`. Final test logs are
`tests/optimization3/copied-app-regressions.log` and
`tests/optimization3/copied-engine-regressions.log`.

## Stability and visual evidence

The corrected candidate completed a real **1,800.010-second mixed-workload
sample**, exiting normally after all twelve workload types and a return to BVR.
The run exercised resets, pause, camera/map changes, landing gear, Earth/Moon,
warp and timestep changes. Finite-state and bounded-effect assertions passed.

- 363 resource samples; maximum particles 110 against a cap of 256.
- Sampled peak RSS 601.8 MiB; final RSS 153.2 MiB. Two-second polling can miss
  short peaks and does not measure all driver/GPU memory.
- Image assets stabilized at 44 after the first five seconds and remained there.
- Mesh/entity counts recovered to 771/4,187, below their initial 780/4,196.
- AC remained connected; macOS reported no thermal/performance warnings.
  Physical temperature and clock telemetry were not available.

Native static captures and ascent/descent sequences were collected. The ascent
sequences have 24 common frames with exactly equal physical-state metadata;
descent has 21. The GPU ascent crossed 100 km, with the inspected spacecraft
mesh and lighting intact. The slower CPU reference stopped before 100 km, so
the full matched post-transition comparison is unfinished.

Existing static comparisons show cockpit, terrain and maps matching outside
the original telemetry mask, with small Earth pixel differences also present
between repeated captures of the same executable. Those comparison JSONs used
an overly broad top-header mask; the script has been narrowed, but the final
recomputed scores and complete motion-image review were not finished. Do not
treat these as comprehensive visual sign-off. Manual native control checks
and additional impact/collision effect captures are also unfinished.

Soak evidence: `accepted-v2/release-soak/` and `accepted-v2/soak-review.json`.
Captures: `accepted-v2/release-baseline/`, `accepted-v2/release-candidate/`, and
the motion sequence folders under `accepted-v2/`.

## Remaining bottlenecks, ranked

1. **Active atmospheric environment filtering.** Metal captures still identify
   radiance/irradiance filtering as substantial recurring GPU work in atmospheric
   cockpit views. Orbital captures omit these events when the source is unchanged,
   supporting the cache's intended behavior. Further work must preserve changing
   lighting and convolution quality.
2. **Cockpit/atmospheric rendering and synchronization.** The cockpit remains
   around 33 FPS despite lower asset churn. CPU sampling includes GPU submission
   and synchronization; more CPU threads alone are not supported as a solution.
3. **Terrain/visibility queries and audio mixing.** These appear in CPU samples,
   but current evidence does not establish that optimizing them would solve the
   cockpit frame budget.

The corrected cockpit Metal capture measured 52.4 resource-creation events per
observed opaque frame, compared with about 79.4 before binding reuse. This is
resource turnover, not a memory or FPS percentage. The isolated same-executable
cache-disabled Metal comparison was not completed. Profiled GPU event intervals
overlap and must not be summed into a frame time. Native draw-call and allocation
counts remain unavailable rather than being reported as zero.

The evidence does not establish that an engine migration is necessary or that
it would deliver 60 FPS at identical quality.

## Delivery state and unfinished work

The optimized candidate is built and preserved at:

```text
tests/optimization3/candidate/gripen-flight-release
SHA256 039700a75065de7893b101906babd7adece0608db7cebca007982023a1dbb982
```

The preserved optimized baseline is:

```text
tests/optimization3/baseline/gripen-flight-release
SHA256 fa6bddc31cac3f0a439a163326e5b87a49e29b2af0ba512019d73d38f2b6d24c
```

Sources/assets and hashes are archived alongside the binaries. Earlier
`accepted/` measurements used a superseded cache implementation; this report
uses the corrected `accepted-v2/` results.

**The installed `Nord Flight.app` has not been replaced with this candidate.**
Final app packaging/signing/launch verification and retirement of intermediate
artifacts were not completed. No further tests are queued to run automatically.
The remaining full-matrix, quality-preset, isolated cache, visual/control and
packaging checks are intentionally left unfinished following the stop request.

For detailed source contracts and historical measurements, see
[OPTIMIZATION3.md](OPTIMIZATION3.md),
[OPTIMIZATION3_SOURCE_REVIEW.md](OPTIMIZATION3_SOURCE_REVIEW.md), and
`vendor/bevy_pbr/NORD_PATCH.md`.
