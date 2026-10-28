# Hornet FBW validation evidence

2026-10-28. Parent physics milestone: `68ca41e`. Architecture, user-selected scope, source distinctions and estimated parameter values are in [FBW.md](FBW.md). These results validate the implemented simulation in the tested cases; they do not validate production Super Hornet handling qualities.

## Automated checks

The complete binary test suite passes **579 tests**, including the other aircraft, collision, carrier, crew, combat and scenario-reset regressions. Commands and generated local evidence:

```sh
cargo test --bin gripen-flight -- --nocapture
cargo test aircraft::hornet::fbw -- --nocapture
NORD_FBW_PERF_BUDGET=1 cargo test aircraft::hornet::fbw::tests::frame_partition_and_cost -- --nocapture --test-threads=1
```

The 16 dedicated FBW tests cover independent sensor-type voting, outlier exclusion, ambiguous split rejection, frozen and nonfinite data, computer-command isolation and rejoining, channel and power-loss transitions, direct selection, bounded engagement, jammed-effectors reallocation, preserved manual flap selection, paused state, reset, neutral response, control release, engine-out/gust recovery, load/incidence response and frame partitioning. Additional Bevy input and cockpit-action tests check S/W signs, T selection and the disabled Hornet autopilot routes. The numerical/physics tests from the preceding milestone remain active.

A controller-boundary test verifies that FBW evaluation leaves aircraft position, attitude, velocity, angular rate, throttle, flap selection and fuel unchanged. It may command actuator targets and update its own state and pitch-trim bias. This guards against hidden trajectory or throttle control.

| Case | Measured result |
| --- | --- |
| Clean 180 m/s, pitch input .15 | About 2.05 g after four seconds; about .96 g six seconds after release; body-rate norm about .0024 rad/s after release |
| Manual flap .55, 180 m/s, pitch input .15 | About 2.17 g after four seconds; about .89 g six seconds after release, with rates near zero; the separate 120 m/s response-matrix case settles near 1 g by ten seconds |
| Half roll commands, both signs | Body Z rates approximately +1.45 / -1.43 rad/s after 1.5 s; below .4 rad/s after release; both directions exercised independently |
| 230 m/s, sustained full pitch, intact | Peak about 6.20 g and .439 rad (25.2 degrees) incidence in the three-second case; remains finite and intact |
| Same high-pitch case, one stabilator jammed | Peak about 5.12 g and .383 rad; bounded but substantially larger residual yaw/roll motion; this is not equivalent intact handling |
| Ten-second gust recovery | Final normal load about .991 g; sideslip .000377 rad; body rates near zero |
| Ten-second engine-out case | Final normal load about 1.000 g; sideslip -.000569 rad; body-rate norm below .001 rad/s |
| Frozen sensor and separate NaN air-data lane | Disagreeing sources excluded; commanded surfaces remain finite |
| Computer command biased by 10 rad/s² | Output excluded before actuation, lane isolated after the time threshold; rejoined after the bias cleared and agreement persisted |
| Two-versus-two gyro split | No authoritative sensor quorum; direct control selected |
| Two / one / zero computers | Degraded / direct / failed transitions respectively |
| Electrical power removed | Failed mode, zero usable sensor quorums; no invented mechanical control path |
| Ground, full thrust for four seconds | Neutral takeoff trim remains away from the stops, with no automatic rotation |
| Frame partitioning | Equivalent 60/120/240 Hz calls agree within .1 m position and .001 rad/s rate criteria using the bounded 240 Hz plant |
| Carrier arrest, FBW enabled | About 111.87 m effective payout and 51.06 MJ dissipated work; final ship-relative speed about .038 m/s |
| Pilot-commanded launch departure | At 1,000 / 6,000 kg fuel, five seconds after release: altitude 99.15 / 56.35 m, upward speed 29.02 / 15.49 m/s; both intact and airborne |

The dedicated high-input cases do not establish an all-conditions invariant envelope. Soft protection includes sensor filtering, finite actuator response and finite aerodynamic authority. A severe failure can leave significant coupled motion even when the simulation remains bounded. Headless trials apply normalized pilot commands directly; the existing crew/G-LOC input effects remain separate and were not disabled or retuned.

The first full regression run exposed carrier failures caused by inappropriate use of the airborne allocation path while weight remained on the wheels. The controller now has a ground path and hysteretic transition. Allocation also has a secant fallback for backing out of locally clipped polars near a surface stop. The revised system passes the carrier regression set, including pilot rotation at both fuel weights. Earlier failure evidence remains locally in `tests/hornet-fbw/regression.log`.

## Cost on this Mac

Hardware: Apple M1. The explicit isolated 60 Hz benchmark samples 480 rendered-frame-sized physics calls across a maneuver/release trial. Latest optimized-test-profile result: **.121 ms mean, .152 ms p95**, below the 2 / 4 ms physics targets. An earlier isolated sample measured .233 / .309 ms; concurrent full-suite geometry workloads measured .861 / 3.355 ms. These variations include host scheduling; rendering is excluded.

Timing assertions are enabled explicitly with `NORD_FBW_PERF_BUDGET=1` for an isolated target-machine run. Normal CI tests retain trajectory/frame checks and timing output without making shared-runner contention a hardware performance verdict.

## Response data and native app inspection

`response_matrix_writes_diagnostic_evidence` generates `tests/hornet-fbw/response.json`. It samples load, incidence, sideslip, achieved/requested rates, trim, mode, saturation and allocation residual during three pitch and two roll cases. Plot with a Python installation containing matplotlib:

```sh
python scripts/fa18f/plot_fbw.py
```

The generated `tests/hornet-fbw/response.png` was visually inspected. It shows load response returning toward neutral, scheduled roll response in both directions, and pitch-rate tracking/release transients. It is simulation telemetry, not an overlay against manufacturer flight-test data. Logs, plots and screenshots stay local under the repository's generated-test-artifact policy.

The optimized development Mac app was rebuilt using `build-mac.sh dev` and passed strict deep bundle signature verification. Binary SHA-256:

`e89d75a6da5dec5307699ee4d4acbe0511467a3c1def62ff060133b7bf473cb5`

Native app captures were produced and inspected for normal, degraded, direct and failed modes; the cockpit systems display; arrest; pilot-commanded launch departure; and relaunch. The mode/count indications match the physical state, and the launch/relaunch scenes show the aircraft airborne and climbing. Mode capture commands:

```sh
NORD_AIRCRAFT=fa18f NORD_FLEET_CAPTURE=fbw-normal 'Nord Flight.app/Contents/MacOS/gripen-flight' --visual-check dry
```

Substitute `fbw-degraded`, `fbw-direct`, `fbw-failed` or `fbw-normal-cockpit`. Each writes `tests/visual-dry.png`; preserve it before the next run. Carrier presets remain `NORD_CARRIER_CAPTURE=launch-release`, `arrested` and `relaunch`. These presets simulate the ordinary physics/controller and then freeze the result for native rendering; the launch presets supply an explicit pilot pitch input.

A native accessibility connection timed out. Keyboard bindings were tested through Bevy input events, but an OS-level manual piloting session was not completed through that tool. Hands-on user flight review remains required. Gain/scheduling calibration, full fault combinations and the proposed uncertainty sensitivity ranges require further evaluation; no claim of certified or proprietary flight-software equivalence is made.
