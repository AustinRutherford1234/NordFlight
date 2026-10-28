# F-22 high-altitude FBW follow-up

This follows the [flight-model integration](F22_FLIGHT_MODEL_V2.md) and targets
the user's report of poor FBW behavior at the BVR scenario's **11 km** start
altitude. The control laws and protection thresholds remain estimated game
parameters, not reconstructed F-22 flight software.

## Problems found and changes

- Low dynamic pressure previously selected post-stall incidence control even
  when true airspeed was high. Low-speed mode now requires both low pressure
  and low TAS, with a continuous 140–240 m/s transition. Normal-speed thin-air
  flight retains signed normal-load control.
- The old cruise incidence limit allowed the controller to keep pulling deep
  into the separated polar while trying to obtain an unattainable G load.
  Cruise incidence protection now follows TAS and Mach independently of air
  density. Low-speed high-incidence permission depends on balanced, healthy
  thrust vectoring and available thrust; loss of an engine/nozzle reduces that
  permission. Negative incidence receives predictive protection as well.
- Rate-loop bandwidth now follows the moment Jacobian expressed as angular
  acceleration through the current inertia tensor. Weak effectors no longer
  receive the same aggressive rate-tracking gain as strong ones. The requested
  moment also accounts for rigid-body gyroscopic coupling.
- All airborne F-22 control derivatives are evaluated about the current
  actuator state, replacing pitch/yaw derivatives sampled only at neutral.
  One set of six panel evaluations supplies all three columns.
- A bounded, fixed-iteration allocator solves within normalized command stops.
  It preserves cross-axis cancellation when an effector saturates and prioritizes
  pitch/yaw stabilization over roll. The previous independent clipping of an
  unconstrained inverse could discard that cancellation.
- Roll demand falls as yaw authority decreases at high incidence. Excess
  sideslip at normal flight speed reduces roll and positive-incidence demands
  to give yaw recovery priority. Ground-roll trim retains its established law.

These changes command existing surfaces/nozzles. They add no orientation, rate,
speed or altitude clamp and no artificial torque. FBW remains distinct from
altitude hold: neutral stick does not level the wings or flatten a climbing
flight path. Insufficient lift/thrust at extreme altitude can still cause a
descent, and arbitrary deep departures outside available control authority are
not guaranteed to recover.

Other aircraft, crew blackout/control settings, simulation stepping and rendering
quality retain their existing paths.

## Verification

**494 tests pass**, including five new regressions:

1. Normal-load cruise at 18 km and 300 m/s, with less than 100 m altitude drift
   over 20 seconds and settled load within 0.12 G of 1 G.
2. Thirty-six BVR cases: neutral, full aft, full forward, left/right roll and
   combined pull/roll; 290 and 362.5 m/s; two, one or zero running engines.
   Each applies its input for four seconds, then releases for sixteen seconds.
   Checks include meaningful maneuver authority, bounded incidence/sideslip,
   release recovery and finite, uncrashed state.
3. Combined BVR maneuver at 60, 120 and 240 Hz, comparing attitude/rates and
   final trajectory as well as numerical finiteness.
4. A saturated coupled-allocation case that must retain yaw cancellation when
   pitch reaches its stop.
5. Nonzero stick inputs in vacuum with engines off: FBW cannot generate torque;
   integrated inertial angular momentum remains within 0.5% over two seconds.

The full suite also retains takeoff, assisted approach, signed-G holds,
low-speed high-incidence/vectoring, roll direction/authority, damage and missile
guidance checks. An additional 42-case altitude/speed/departure sweep reaches
32 km; those extreme stress cases are diagnostic, not a claim that the aircraft
can sustain flight or recover every departure there.

`tests/f22-high-altitude-fbw/before-bvr.json` captures the previous production
controller using the same diagnostic harness as the updated controller. The
BVR harness uses the scenario's actual position/orientation. It tests both
290 m/s and the 362.5 m/s produced by the existing aircraft-selection speed
scaling. Source snapshots were restored before building/testing the update.

With both engines operating, matched-input comparisons at 11 km give:

| Trial | Previous FBW | Updated FBW |
| --- | ---: | ---: |
| 290 m/s combined pull/roll: peak incidence | 26.06° | 16.76° |
| Same trial: peak sideslip | 17.16° | 2.65° |
| Same trial: peak body angular speed | 1.911 rad/s | 0.854 rad/s |
| 362.5 m/s full aft stick: peak incidence | 31.42° | 19.88° |

The saved response plot shows the previous post-release oscillation and the
updated recovery. These values compare identical starts and inputs, not equal
final trajectories or equal achieved load. The cruise incidence protection
intentionally gives up unattainable requested G before a departure.

The final assisted approach still lands on the paved runway at 83.55 m/s with
2.29 m/s sink. Gripen, Su-27, F-35B and Hornet flight-study rows and the separate
Su-27 cobra trace match the previous build exactly.

Sequential uncapped numerical-study timing (before/after/after/before, 187,200
physics steps per run) measured 7.83–9.09 seconds before and 4.65–4.69 seconds
after. This includes process startup, diagnostics and JSON serialization with
stdout discarded; it is not an isolated physics microbenchmark or a rendering
FPS measurement. The new allocator has fixed work, and reusing one set of panel
derivatives removes the previous duplicate probes. Raw timings are saved in
`timing.json`.

The final native executable was also captured and visually inspected in BVR
external and cockpit/HUD views at 11 km. These paused captures check startup and
presentation; the production-physics trials above check dynamic handling. The
local app bundle is updated, ad-hoc signed and passes strict signature verification.

```sh
cargo test --locked --offline
cargo build --locked --offline
target/debug/gripen-flight --study-f22-fbw > tests/f22-high-altitude-fbw/after.json
python3 tests/f22-high-altitude-fbw/plot_response.py
```

The plot additionally needs the saved local baseline JSON. Traces, native
captures, comparison measurements and final delivery metadata remain under
`tests/f22-high-altitude-fbw/`. Tests establish simulated handling and numerical
regressions; they do not validate the real aircraft's control laws.
