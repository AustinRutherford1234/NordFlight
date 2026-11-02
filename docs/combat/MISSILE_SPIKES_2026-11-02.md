# Periodic missile-salvo hitch investigation

## Reproduction and diagnosis

The user reported severe spikes about every .2 seconds with roughly 30 airborne missiles. The culprit reproduced in a short live fullscreen, normal-quality Hornet cockpit test with a seeded 30-cruise-missile salvo and the normal fleet defense logic. Additional ship-launched rounds increased the live count slightly. No soak was used.

The fleet engagement scheduler runs every .25 simulation seconds. It checked radar/terrain visibility and swept ship obstruction for candidates before checking whether the launcher was cooling down, had ammunition, or the target already had two assigned defensive rounds. Long obstruction rays used their complete enclosing axis-aligned box to search the ship BVH. A long diagonal ray's box overlaps many triangles that are far off the ray, causing expensive sphere/triangle tests.

During the 25-second measured live baseline (after five seconds of warm-up), 97 engagement decisions averaged **56.595 ms**, peaked at **74.556 ms**, and used 5,489.7 ms of CPU wall time. Overall frame intervals measured median **14.630 ms**, p95 **67.964 ms**, p99 **78.272 ms**, average **48.23 FPS**. The mean engagement cost tracks the reported periodic stutter. The fleet radar scan averaged .0385 ms and countermeasure check .0016 ms; those periodic tasks did not explain this reproduced hitch. Timings include host scheduling, and nested CPU spans must not be summed.

## Correction

- Reject launchers on cooldown or without ammunition before examining candidate geometry.
- Reject saturated target commitments and unreachable shots before terrain/ship visibility tests.
- Traverse sphere-sweep BVH queries using conservative segment-versus-expanded-box slab tests at nodes and leaves, rather than only the enclosing box of the entire path.
- Retain the existing swept sphere/triangle narrow phase, moving/damaged ship coordinate transforms, hit fractions, physical interception, and raised catapult deflector tests.
- Add opt-in CPU spans and maximum-call times for the naval scheduler, countermeasures, weapons, CIWS, visuals and missile smoke. Normal play does not enable timing collection.

No missile-count cap, guidance frequency, motor burn, physics timestep, rendering quality or collision mesh was reduced.

## Headless measurements and correctness

A 48-sample cloned 30-round fixture measures the scheduled work independently from rendering. The matching before/after scenario holds all launchers in cooldown, where launches cannot occur:

| Task | Before mean / p95, ms | After mean / p95, ms |
|---|---:|---:|
| Fleet engagement decision | 67.493 / 80.602 | .00129 / .00133 |
| Whole weapon update, including scheduled work | 95.698 / 192.546 | .0369 / .0285 |

A separate after-only, launchers-ready fixture measured .395 ms mean, .441 ms p95 and .847 ms maximum per engagement decision. It is not a matched comparison against the cooling-launcher baseline. The headless timing harness also runs scan/countermeasure phases in a fixed order, so visibility-cache state can affect individual measurements. The live baseline independently confirms the same expensive engagement path.

Across 144 sphere/ray sweeps over both installed ship collision models, with radii 0, .1 and 2 m, the optimized query matched the original query's hit existence and earliest fraction within 1e-10. There were 68 matching hits. Narrow-phase triangle candidates fell from **176,255 to 3,580** in this fixture. Additional slab tests cover parallel, tangential, stationary and away-moving segments; existing raised-deflector and moving/damaged ship tests remain enabled. This is sampled equivalence with regression coverage, not a mathematical proof for all possible input geometry.

Full regression: **785 passed, zero failed, one opt-in diagnostic ignored**, 48.70 s. The ignored diagnostic was then run explicitly and passed, as did the isolated BVH equivalence test.

Reproduce CPU measurements:

```sh
NORD_MISSILE_PROFILE_REPORT=tests/missile-spikes/after.json cargo test --locked thirty_missile_periodic_work_profile -- --ignored --nocapture
cargo test --locked segment_bvh_matches_original_triangle_sweeps_on_both_ship_models -- --nocapture
```

Reproduce a short native profile only after obtaining permission to pause the user's flight:

```sh
python3 scripts/fa18f/profile_missile_spikes.py --label after --seconds 15
```

The native harness adds five seconds of warm-up, uses the existing fullscreen benchmark, and restores prior running game sessions in a `finally` handler. Reports and logs stay local under `tests/missile-spikes/`. The fixture is an opt-in review mode, not a new game mode.

## Live follow-up limitation and delivery

The first updated-build comparison exited early without a report. The approved retry failed in Metal diagnostic collection with `Cannot allocate sample buffer`, followed by a lost graphics device. This happened with GPU timing enabled. The ordinary game does not enable these profiling plugins.

A shorter FPS-only follow-up completed using five seconds of warm-up and five seconds of measurement, within the remaining short-comparison budget. It measured **21.49 FPS**, median **20.328 ms**, p95 **156.363 ms**, and p99 **440.719 ms**. Only 3.20 simulation seconds advanced during that five-second sample. It therefore **does not establish an end-to-end FPS improvement**. It also differs in instrumentation and measurement duration from the baseline. Several large stalls remained near startup and within the measurement window; no unmeasured cause is assigned to those stalls. The periodic CPU path is corrected in the isolated fixture, while the remaining live frame stalls are unresolved.

The sample log marks environment capture/publication activity, but that alone does not prove it caused the large stalls. A quick post-run memory-pressure query reported 66% system-wide memory free; this snapshot does not establish memory pressure during the failed traces. No further live profiling or soak was run.

For a future approved frame-only check, add `--fps-only` to the native harness. That excludes GPU timing allocation and all CPU span counters, while retaining frame intervals, simulation-clock and entity/weapon-count reporting. A longer warmed comparison and CPU-only spans without GPU counters would be needed to distinguish remaining rendering/presentation and startup work. Those are follow-up investigations, not claims of this delivery.

The existing Mac app was rebuilt using `./build-mac.sh dev`, and its ad-hoc signature passed `codesign --verify --deep --strict`. Executable SHA-256: `92030c2b87f2c5302e2b8a1864802296c3bfd761ff1f59a08202cc645cae24e5`. Restart the app to load the corrected source. Every native harness ran its session-restoration handler.
