# F-22 flight model and missile release dynamics — 26 October 2026

This revision integrates the flight-model review in `src/aircraft/raptor/flight_model.rs`
and changes shared missile attitude dynamics in `src/combat/guidance/dynamics.rs`.
Coefficients, engine maps, fuel consumption, surface rates, structural limits and
fuel-tank locations remain transparent **simulation estimates**. They are not a
production F-22/F119 database, certified limits or reconstructed flight software.

The subsequent [high-altitude FBW follow-up](F22_HIGH_ALTITUDE_FBW.md) supersedes
the initial control scheduling/allocation described below and records its
separate BVR regression checks. Results below refer to the initial integration.

## F-22 changes

- **Engine envelope:** separate military/maximum thrust tables versus Mach and
  equivalent ISA density altitude. Continuous interpolation replaces the former
  4.4× high-Mach afterburner-increment multiplier and 90% recovery drop between
  Mach 2.5 and 2.7. Angle of attack and sideslip now reduce installed inlet
  recovery. Zero density produces zero air-breathing thrust.
- **Fuel:** estimated TSFC varies with afterburner and Mach, and is applied to
  the actual running-engine fraction. One healthy running engine consumes half
  the symmetric two-engine flow at the same operating point. Stopped engines
  consume no fuel; leaks remain additive.
- **Aerodynamics:** F-22-specific lift, separated drag, pitching moment and
  Mach corrections replace the shared generic polar. Each wing has a convective
  separation/reattachment history. Sideslip and local roll-induced incidence can
  produce asymmetric separation and roll moment. Tail stability/damping fade
  with high incidence; vortex lift retains a bounded transient response.
- **Controls:** dynamic-pressure blending between low-speed incidence commands
  and higher-speed signed-G commands. Full stick requests +9/−3 G, with rate and
  predictive incidence protection. The old input-multiplication limiter is not
  cascaded onto this G command. Feedback reads the completed accelerometer
  sample, including moving-panel forces. Roll includes a pressure-limited wind-axis yaw
  component so low-speed yaw authority is respected.
- **Physical actuators:** moving surfaces have independent positions, response
  and rate limits. Estimated travel is ±25° for stabilators/ailerons, −30°/+20°
  for flaperons, ±30° for rudders and −5°/+25° for leading flaps. Estimated rates
  are 65, 90, 80 and 35°/s by surface class, with pressure loading and hydraulic
  health. Jams override commanded travel, and detachments remove loads.
- **Vectoring:** retain separate pitch-axis nozzles, ±20° travel and 1 rad/s
  actuation. A separate demand filter prevents aerodynamic hinge loading from
  slowing the engine-nozzle command. Differential roll remains an explicit game
  allocation choice. The previous blanket 1.30× stabilator force multiplier is
  removed; roll/control regressions exercise the new physical limits instead.
- **Mass properties:** balanced feed from four estimated tank locations plus
  the actual stored-weapon/pylon positions. A symmetric full inertia tensor is
  assembled about the current CG using the parallel-axis theorem. Asymmetric
  stores produce products of inertia; axes no longer all scale by one mass ratio.
- **Open bays:** main- and side-bay door/launcher positions contribute drag.
  Offset application of bay drag also supplies small pitching/yawing moments.
- **Structural envelope:** separate positive and negative load damage, dynamic
  pressure, pressure-gated Mach and a lagged skin-temperature estimate. Current estimated
  thresholds are +9.5/−3.5 G for accumulating overload damage, 95 kPa dynamic
  pressure, Mach 2.35, and 465 K skin temperature. They are damage-model tuning
  values, not claimed F-22 flight-manual limits. No velocity or attitude clamp
  is used to manufacture a maximum speed. Aerodynamic heating and Mach damage
  vanish at zero dynamic pressure.
- **Approach:** target speed follows weight, flap configuration and density,
  replacing the fixed 106 m/s TAS target. The existing glide/flare assistant
  consumes that target. The diagnostic level-flight pilot uses the inverse of
  the new G-command mapping; historical acceleration trials used a different
  pilot law and should not be treated as identical control-input comparisons.

Other aircraft retain their existing aerodynamic/engine/control paths. Crew
blackout settings, simulation timestep semantics and rendering quality are
unchanged. The runtime model uses small tables, fixed-size state and matrix
operations; it adds no CFD solver or per-substep heap allocation.

## Missile release

The previous controller demanded a body direction derived from the airflow even
when the missile was nominally in its separation phase. Neutral fins therefore
did not mean passive attitude dynamics: a hidden 0.08-second rate servo actively
aligned the body to the air-relative velocity.

The replacement integrates angular motion from estimated inertia, static
restoring moment, aerodynamic damping and bounded control torque. Initial
aircraft/rail attitude and angular velocity are retained. Restoring torque
depends on aerodynamic force and a small estimated static margin (1.2% of length
for IR, 1.8% for radar), rather than a fixed alignment-rate command. Roll damping
also depends on pressure and vanishes in vacuum.

Fins and active attitude control remain neutral through 0.22 seconds of clearance.
Active control authority then blends in over 0.16 seconds, subject to available
aerodynamic/motor-vector torque. Motor timing, seeker/navigation logic,
countermeasures and guidance acceleration limits retain their existing roles.
All live player, enemy and BVR missile paths now supply local atmospheric wind;
air-relative velocity drives aerodynamic incidence/drag, while world velocity
continues to propagate navigation and position. The renderer uses integrated
body attitude, including roll, rather than facing the velocity vector.

## Public anchors and limits

- [USAF F-22 fact sheet](https://www.af.mil/About-Us/Fact-Sheets/Display/Article/104506/f-22-raptor/About-Us/f-22-raptor/): dimensions, mass/fuel class, Mach-two-class speed and supercruise beyond Mach 1.5.
- [Pratt & Whitney F119](https://www.rtx.com/en/prattwhitney/products/military-engines/f119): 35,000-pound thrust class, two-dimensional pitch-axis vectoring up/down 20°, and integrated engine/flight controls.
- [AFFTC initial F-22 high-AoA test results](https://sfte-ec.org/sfteecold/data/Abstract/A2000-II-02.pdf): controlled testing beyond 60° incidence, with explicit discussion of differences between predictions and flight tests.
- [NASA HARV research](https://www.nasa.gov/reference/f-18-harv/): general high-incidence flow/control/inlet research context; HARV measurements are not substituted for F-22 derivatives.

Tests establish numerical behavior, physical consistency and game handling.
Matching the real aircraft across its envelope would require suitable reference
data. Inlet losses are bounded estimates; engine surges, detailed compressor
dynamics, individual fuel-transfer plumbing, fuel slosh, structural finite-element
loads and production control-law modes are outside this reduced model.

## Reproduce

```sh
cargo test --locked --offline
cargo build --locked --offline
target/debug/gripen-flight --study-f22 > tests/f22-flight-integration/after-f22.json
target/debug/gripen-flight --study-fleet > tests/f22-flight-integration/after-fleet.json
target/debug/gripen-flight --study-missiles > tests/f22-flight-integration/after-missiles.json
target/debug/gripen-flight --study-missile-release > tests/f22-flight-integration/release.json
python3 tests/f22-flight-integration/plot_response.py
```

Raw before/after studies, final checks, native screenshots and delivery metadata
are stored locally under `tests/f22-flight-integration/`.

## Verification and observed results

The final optimized development build passes **489 tests**, including the existing
guidance/interception, timestep, control-authority and damage regressions. At
330 m/s and 6 km, full signed-G commands settle to final-half-second means of
**+8.9715 G and −3.0673 G**. The assisted approach trial lands on the paved runway
at **83.53 m/s (162.4 kt)** and **2.23 m/s** sink without crashing. These are
simulated handling checks, not real-aircraft flight validation.

The two incidence trials peak at 49.49° and 50.28° and recover to 3.39° and 2.64°
by 20 seconds after releasing the stick at 8 seconds. Gripen, Su-27, F-35B and
Hornet fleet-study rows match the saved baseline exactly; the separate Su-27
cobra traces also match exactly.

The release study covers three missile types, two altitudes and three initial
incidences. For a 20° AIM-9X release at 250 m/s, incidence is still **16.78° after
100 ms at 3 km**, or **18.97° at 12 km**. Fins and active control torque remain
zero during clearance. Lower air density produces a slower passive response.
The saved `missile-release-response.png` plots that finite response and subsequent
control handover. Native cockpit, missile and combat-warning screenshots were
captured with the final executable and visually inspected.

Full-screen native spot checks used 2880×1800, standard graphics, diagnostic
uncapped presentation, 12 seconds of warmup and 12 seconds of sampling:

| View | Baseline mean FPS | Final mean FPS | Baseline p95 frame time | Final p95 frame time |
| --- | ---: | ---: | ---: | ---: |
| External | 54.40 | 55.17 | 20.77 ms | 21.95 ms |
| Cockpit | 59.30 | 56.76 | 24.83 ms | 26.40 ms |

These short sequential live trials have different evolving trajectories and
background scheduling, so they do not isolate the cost of the physics change.
The final cockpit sample has more long frames; neither view establishes steady
60 FPS. Separate process-inclusive numerical-study timings were about 38.5–42.0
µs per physics step before and 37.0–38.6 µs after. Those timings include startup
and JSON output and used the candidate before the final small G-feedback/inertia
corrections. They indicate similar CPU cost, not a measured rendering speedup.
