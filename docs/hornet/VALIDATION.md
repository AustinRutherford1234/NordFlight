# F/A-18F physics implementation and evidence

2026-10-28. Current status: automated physics/regression checks pass; the rebuilt application still requires the user's flight review. This document separates numerical checks, engineering estimates, broad specification checks and interactive acceptance. None of the automated results constitute real-aircraft flight-test validation.

## Architecture and conventions

`src/aircraft/hornet/` owns the Hornet plant. The existing `physics::simulate` loop dispatches to it, substepping to at most 1/240 s. Other aircraft retain their force paths. There is no second world simulator. World position, velocity, quaternion, local forces and full inertia arithmetic use double precision. SI body axes are +X right, +Y up, +Z aft; a positive X moment pitches the nose up. `Flight.position` remains the mesh/body origin; translational velocity is CG velocity, with the body-origin displacement corrected as attitude changes.

Mass properties sum crew, equipment/pylons, explicit fuel locations, installed stores and removable structural components. Parallel-axis contributions give the full symmetric tensor about the current CG. Releases and component detachments change mass and inertia. Uniform fuel depletion preserves the estimated tank fractions; this is not a detailed fuel-transfer system.

The aerodynamic evaluator includes eight wing strips, separate LEX, left/right tail and fin elements, and fuselage contributions. Each samples rotation-induced flow; wing/tail/fin modal velocities and incidence feed back into loads. Wing lift, compressibility and wave-drag tables interpolate smoothly. Separate flow memory gives lagged separation/reattachment. Gear, brakes and existing store drag enter once. The old aggregate Hornet polar and lift multiplier no longer enter the active integration path. Leading-edge droop modifies separation onset; trailing-edge surfaces contribute at wing-strip locations. This is a reduced-order model, not a panel method or independently resolved flap flowfield.

Physical actuators limit travel, rate, acceleration and authority with an estimated hinge-load proxy and hydraulic pressure. Jams and detachment use the existing damage state. `assistance.rs` adapts the prior rate-command targets/gain and existing envelope assistance to physical actuator requests using the new plant derivatives. It never sets angular velocity or attitude. No new production F/A-18 flight-control laws were developed. T disables assistance; all raw validation cases also leave autopilot disabled.

Each engine has independent spool, thermal and distortion histories, thrust, fuel flow and deterministic flameout/restart conditions. Thrust acts on the measured mesh nozzle lines. Shutdown/exhaustion removes powered thrust; windmilling retains flow-opposing drag. Aggregate telemetry feeds the existing presentation interfaces.

Wing, tail and fin bending/torsion use the shared exact held-load damped oscillator. Stiffness degrades with damage and estimated thermal exposure. Modal velocities/incidence feed aerodynamic loading; aerodynamics, inertia, gear contacts, hook and catapult loads have separate diagnostic channels. Positive/negative load exceedance, pressure, Mach and temperature have distinct damage contributions. These modes do not establish a validated flutter boundary or fatigue life. Visual deformation is outside this change.

## Carrier mechanics

Wheel positions/radii follow the articulated asset. Spring/damper and bump-stop reactions, braking, rolling friction, lateral tyre slip and steering use velocities relative to the actual supporting deck or ground. Steering rotates the tyre instead of adding a side force. The passive nose-caster response is explicitly an engineering estimate, not an assertion about the real NWS system. Wet-deck and land lateral-friction estimates differ.

The old `carrier::constrained` trajectory was removed. Catapult and cable forces pass through ordinary six-degree-of-freedom integration. The catapult follows an estimated 105 m force/energy schedule with mass and engine contribution, a down component at the launch attachment, finite force and release limits. It supplies no velocity boost or attitude snap. Initial spawn placement and stopped-aircraft Alt+C repositioning remain explicit conveniences.

The swept hook tip determines catches. Its lowering angle is shared with the visible hook and follows simulation time. The cable force acts at that tip, follows the anchor geometry, resists payout, accumulates dissipated work and releases beyond finite stroke/energy limits. The absorber is a reduced-order dissipative law, not a proprietary cable/valve model. It can creep slowly under residual idle thrust after stopping; the nominal test settles to about 0.04 m/s rather than using a prescribed zero velocity. The moving deck remains on the specified straight 25-knot course.

Approach speed comes from the new aircraft's lift/trim solution. Airspeed and ship-relative speed are distinguished when constructing approach velocity and pitch. Parked start keeps idle/zero initial throttle rather than applying the airborne trim's throttle.

## Automated evidence

Command: `cargo test --bin gripen-flight -- --nocapture`. Saved local log: `tests/hornet-physics/final.log`. On this Apple M1 Mac, **561 passed, 0 failed**, including the existing fleet, carrier, collision, resets, rendering contracts and combat regressions. Generated logs/screenshots are intentionally excluded from Git; test source and this report are versioned.

| Check | Result / scope |
| --- | --- |
| Straight trim | 0 m / 120 m/s, 3 km / 180 m/s, 8 km / 250 m/s, 12 km / 300 m/s pass <1% force and .001 normalized moment residual |
| Six-variable balanced trim | 1,000 and 5,000 kg fuel; level, ±.05 rad flight-path angle, ±.35 rad bank at 3 km / 180 m/s; largest force norm .00607, moment norm .0001432 |
| Raw 30 s flight | Starts at 3 km / 180 m/s; ends 3009.07 m / 179.93 m/s; maximum body-rate norm .0006163 rad/s; no assistance or autopilot |
| Advertised speed plausibility | Clean configuration at 11 km / Mach 1.8 trims at power .923, force residual components <.000006 of weight; broad capability check, not measured performance agreement |
| Advertised altitude plausibility | Clean configuration at 15.4 km / 295 m/s trims at power .576; force residual norm about .00402; does not locate service ceiling or climb-rate limit |
| Frame partitioning | Equivalent 60/120/240 Hz calls use the same bounded 240 Hz plant and agree; this checks frame independence |
| Integration refinement | Separate 240/480/960 Hz plant runs pass the 2% velocity/peak-load criteria in the nominal five-second case |
| Conservation / signs | Tensor symmetry/positive energy; asymmetric-store CG; vacuum rotational energy/angular momentum <1% drift; zero flow; table continuity; control-moment signs |
| Unsteady and failures | Gust, large incidence, command reversal, asymmetric store, engine-out, jam, hydraulic loss and detachment paths stay finite in tested short runs; no claim of measured recovery/departure fidelity |
| Modal decay | Wing/tail/fin bending and torsion test frequencies dissipate free-response energy |
| Arrest, raw and assisted | Both catch and stop without damage; effective payout 111.84 / 111.86 m; dissipated work 51.04 / 51.09 MJ; final ship-relative speed .0385 / .0366 m/s |
| Hook / misses / limits | Catch requires lowered hook and actual tip crossing; hook-up bolter, sampled deck misses, payout limit release and island checks covered |
| Catapult mass variation | 1,000 / 6,000 kg fuel; release after 2.85 / 2.87 s with 76.03 / 76.11 m/s ship-relative speed; no velocity reset |
| Pilot-commanded deck departure | After release, .5 normalized pitch for 1.25 s then neutral; five seconds of free-flight continuation: 1,000 / 6,000 kg fuel ends at 72.60 / 37.69 m altitude with +22.27 / +8.63 m/s vertical speed; no velocity or attitude reset |
| Catapult aborted launch | Lowering throttle releases the launch state; finite loads still integrate normally |
| Physics performance | 3,600 measured 60 Hz frames, raw then assisted: .0469 ms average, .1263 ms p95 in optimized test profile; rendering excluded; below 2 / 4 ms targets |
| Stabilator regression | Nose-up command produces greater nose-up moment and tail downforce; both imported trailing edges rise in the renderer's transform test |

The maximum-speed test originally used the default eight-missile loadout and failed. It was corrected to compare the advertised capability in a clean configuration. Drag was retained for the loaded aircraft. The earlier failed log remains locally available as `regression-before-clean-configuration-fix.log`.

The longer arrest test initially exposed an aggressive absorber/tyre combination and rollover. The final estimated force schedule, physical tyre steering and caster response were checked through the full stop with assistance both enabled and disabled. These are internal consistency/calibration results; they are not independent carrier arrest measurements.

## Remaining acceptance and limits

The public references do not supply a full installed thrust deck, derivative database, departure map, modal survey or failure envelopes. Coefficients, transient behavior and failure thresholds remain bounded engineering estimates listed in [PARAMETERS.md](PARAMETERS.md). Specification plausibility checks do not establish measured agreement across the envelope. The uncertainty ranges have not undergone a complete sensitivity campaign.

Automated tests sample representative cases; they do not exhaust every weight, altitude, damaged configuration, off-centre landing, gust spectrum or simultaneous failure. Full climb/acceleration curves and demonstrated stall recovery still require comparison with suitable public evidence. User flight review is a required acceptance step. New FBW, HUD, weapons procedures and deformation graphics remain deferred as requested.


## Mac application inspection

The optimized development bundle at `Nord Flight.app` was rebuilt with `build-mac.sh dev` and signed locally. Native Bevy screenshots were inspected for both stabilator directions, the carrier approach scene, the stopped arrest and the launch/relaunch sequences. The two stabilator captures visibly raise/lower the trailing edges with opposite physical commands. Carrier capture presets simulate normal forces before freezing the resulting scene for rendering; they are not a manual piloting session. Launch captures explicitly apply the same pilot rotation input exercised by the departure test. A native-app accessibility connection timed out, so interactive keyboard manipulation through that tool was not verified. The user's hands-on flight review remains open.

Local captures live under `tests/hornet-physics/`: `stabilator-up.png`, `stabilator-down.png`, `approach.png`, `launch-release.png`, `arrested.png`, `relaunch.png`. Run the app with `NORD_AIRCRAFT=fa18f NORD_FLEET_CAPTURE=stabilator-up` (or `stabilator-down`) and `--visual-check dry` for the stabilators; use `NORD_CARRIER_CAPTURE=approach|launch-release|arrested|relaunch` for carrier sequences. Each writes `tests/visual-dry.png`; preserve it before running the next case. `arrest-mid` and `launch-mid` show intermediate states.

Delivered optimized development binary SHA-256: `dc554f0275031ad0a6e8eb2cab066107a146b6f670d9b3c7e1dd322b1028a61e`. Strict deep bundle signature verification passed. The final source regression rerun (`final-after-capture-harness.log`) also passed all 561 tests after the native capture harness and departure test were added.
