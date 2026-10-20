# Nord Flight optimization: implementation and validation

Updated October 17, 2026. Workspace: `/Users/Austin/Downloads/GripenFlight`.

The subsequent user-approved cockpit/atmosphere plan is now complete and
installed. Its final report is [ATMOSPHERIC_OPTIMIZATION.md](ATMOSPHERIC_OPTIMIZATION.md).
The historical stop/pending notes below belong to the earlier general pass;
they do not represent the current delivery state. Further optimization options
are ranked in the focused report, with no additional jobs queued.

Diagnostics stopped at the user's request on October 17. Implementation is
built; ten complete render comparisons and the real 30-minute soak are recorded
in [OPTIMIZATION_REPORT.md](OPTIMIZATION_REPORT.md). Remaining validation and
app packaging are unfinished. No diagnostic jobs remain running or queued.
The original handoff is preserved as `tests/optimization3/next-steps-start.md`.

## Implemented work

- Earth haze, cloud and surface colors now use the existing formulas at the
  existing mesh vertices on the GPU. Geometry, textures, interpolation and
  precision safeguards remain. A CPU-reference path supports equivalence checks.
- Cockpit uploads retain producer/staging buffers, detect exact changed bounds,
  preserve pending updates across resize/readiness changes, and perform GPU writes
  outside the producer lock. Resolution and refresh behavior are retained.
- Unchanged material, light, layout and selected transform assignments avoid
  unnecessary extraction/preparation work. Decoy and prior-aircraft-pose buffers
  are reused; maintained smoke bounds avoid duplicate automatic computation.
- Conservative lake-edge bins preserve the original crossing arithmetic and
  edge order. Aircraft physics, AI frequency, collision accuracy, seeded damage,
  integration and orbital propagation are unchanged.
- The pinned Bevy PBR crate caches exact camera-independent atmosphere LUTs.
  Environment-generation bindings, views and constant buffers are reused when
  every dependency matches. Completed convolution results are reused only when
  the atmosphere-owned source has not been written and all resources/pipelines
  match, with one 3D view. Active lighting updates retain their original
  resolution, sample counts and cadence. See `vendor/bevy_pbr/NORD_PATCH.md`.
- The harness records frame percentiles, simulation rate, CPU spans, asset
  mutations, cockpit upload traffic, RSS, worker counts, power and thermal-warning
  status. It rejects stale artifacts and recovers interrupted runs by checking
  process identities. Native capture sequences are opt-in and excluded from FPS
  measurements.

## Validation and delivery record

| Check | Current evidence |
| --- | --- |
| Application regressions | 297 passed; `tests/optimization3/final-capture-regressions.log` |
| Engine regressions | 8 passed, including producer/copy/filter ordering, source contents and cache invalidation/readiness; `tests/optimization3/copied-engine-regressions.log` |
| Harness regressions | 3 passed |
| Development numerical equivalence | All nine JSON reports and both AI CSVs match; `dev-physics-equivalence.json` |
| Optimized executable pair | Preserved under `tests/optimization3/baseline` and `candidate`; same compiler, target, release optimization and thin LTO |
| Optimized physical equivalence | All nine JSONs and both AI CSVs match; `accepted-v2/release-physics-equivalence.json` |
| Native visual/motion equivalence | Captures collected; common ascent/descent state metadata match; full image review unfinished |
| Alternating render matrix | Ten complete scenes; 20-second warmup, 40-second samples, two pairs per scene; remainder stopped |
| Mixed 30-minute soak | Passed 1,800.010 measured seconds; all twelve workloads, finite states and bounded effects; images stable after startup, mesh/entity resources recover |
| Packaging/signature/launch | Completed October 17 on user request: optimized candidate installed, strict/deep ad-hoc signature verified, native window confirmed; `tests/optimization3/installed-build.json` |

See [OPTIMIZATION3_SOURCE_REVIEW.md](OPTIMIZATION3_SOURCE_REVIEW.md) for subsystem
boundaries. See [OPTIMIZATION3.md](OPTIMIZATION3.md) for measured bottlenecks and
the final before/after table. Source archives and hashes preserve the starting
code and accepted candidate. The Cargo-cache policy remains 12 GiB.

## Reproducing validation

Run one workload at a time, without compilation or another benchmark. Keep the
power state and visual/timestep configuration consistent. The final suite uses:

```sh
python3 tests/optimization3/run.py tests/optimization3/baseline/gripen-flight-release release-baseline cpu
python3 tests/optimization3/run.py tests/optimization3/candidate/gripen-flight-release release-candidate cpu
python3 tests/optimization3/verify.py tests/optimization3/release-baseline tests/optimization3/release-candidate tests/optimization3/release-physics-equivalence.json
python3 tests/optimization3/matrix.py tests/optimization3/baseline/gripen-flight-release tests/optimization3/candidate/gripen-flight-release release-matrix
python3 tests/optimization3/run.py tests/optimization3/candidate/gripen-flight-release release-soak soak
```

Use fresh output labels when repeating completed runs. `matrix.py` can resume
completed scenes only when executable hashes match. `final_validate.py` sequences
the complete final suite; `final-progress.json` identifies its current stage.

Diagnostic switches, read at process startup:

- `NORD_EARTH_CPU=1`: original CPU Earth color path.
- `BEVY_DISABLE_STATIC_ATMOSPHERE_CACHE=1`: original LUT dispatch behavior.
- `BEVY_DISABLE_ENV_BIND_GROUP_CACHE=1`: original environment binding preparation.
- `BEVY_DISABLE_ENV_CONTENT_CACHE=1`: original repeated environment filtering.
- `NORD_COMPUTE_THREADS=4`, `6`, or `8`: controlled worker experiments.
- `NORD_CAPTURE_SEQUENCE=/absolute/output/path`: at most 32 lossless native
  captures at two-simulated-second intervals, with physical-state metadata.
  Use this only for visual checks; readback makes those timings unsuitable for FPS.

## Remaining optimization decisions

The native Metal trace identifies repeated environment radiance/irradiance
convolution as a substantial GPU workload. CPU improvements and more worker
threads cannot remove that cost. Future work should measure exact shader or
resource-management improvements first. Reducing lighting samples, map resolution,
refresh frequency, mesh detail or physics cadence is outside this pass.

The 60 FPS goal is assessed against a 16.7 ms frame budget, including p95 and
stutter frequency. It is not an assumed outcome. Missing GPU timestamp, draw-call
or CPU allocation metrics must be labeled unavailable rather than zero. An
engine migration requires a matched-quality prototype and concrete evidence of
an engine limitation; it is not justified by CPU timings alone.
