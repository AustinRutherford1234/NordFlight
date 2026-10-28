# F/A-18F flight controls

2026-10-28. This replaces the temporary Hornet rate-command adapter. It is a representative, estimated control system for NordFlight's reduced-order aircraft, not a reproduction of proprietary Super Hornet control software. The physical aircraft, engines, contacts and six-degree-of-freedom integration remain authoritative.

## Selected scope and controls

- Keyboard first. **S / Down Arrow** commands nose-up; **W / Up Arrow** commands nose-down. Q/E commands roll; A/D commands yaw. The earlier carrier hint saying W to rotate was incorrect and is corrected to S.
- **T**, or the existing cockpit FBW switch, selects normal control or explicit direct control. Automatic faults can select degraded, direct or failed operation. A direct selection cannot restore failed computers, electrical power, hydraulics or detached surfaces.
- **F** retains the current manual flap selection: up / .55 / full. There is no automatic flap schedule.
- Automatic pitch trim and basic stabilization remain on approach. The pilot flies the flight path and throttle. There is no approach guidance controller, autothrottle, flight-path/altitude hold, automatic flare, MAGIC CARPET mode or automatic launch rotation.
- After catapult release the pilot rotates with S. No launch velocity/attitude reset is added. Catapult and wheel reactions continue through the physical plant.
- Hornet autopilot is removed: Alt+F6 cannot enable it, cockpit AP buttons are omitted, and the physics entry clears stale autopilot state. Other aircraft keep their existing systems.
- Existing flight/system text and the cockpit systems screen show mode, available computer count and sensor quorum. No separate configuration panel or new HUD guidance is introduced.

## Signal path

Four sensor lanes produce rate, air-data and normal-load samples. Each data type is independently range-checked and voted. A unique largest mutually consistent group of at least two sources is required. An isolated outlier is excluded immediately; a two-versus-two disagreement does not average two incompatible solutions. The voted data uses a 35 ms filter. A frozen sensor remains latent until its disagreement becomes observable; a common-mode error shared by all sources can remain undetectable.

Four flight-computer lanes maintain independent rate/load integrators. Their acceleration demands are compared before allocation. A divergent lane is excluded from the output immediately and isolated after 100 ms. With a remaining quorum, an enabled lane can rejoin after two seconds of consistent agreement. Existing avionics health affects computer and sensor availability through documented deterministic thresholds. Explicit channel, bias, freeze and validity faults are also available to headless tests. There are no unexplained random control failures.

| Mode | Availability and behavior |
| --- | --- |
| Normal | At least three computers, three-source quorums for all required data, and adequate actuator/hydraulic state; scheduled command laws and protections |
| Degraded | Two computers, reduced sensor quorum, or degraded effectors/hydraulics; conservative rate commands; usable protections retained when air/load data remains valid |
| Degraded with invalid air/load data | Gyro damping and conservative pilot commands only; no model inversion or unavailable air/load protection |
| Direct | Pilot override, one remaining computer, or insufficient rate-sensor agreement; manual commands plus retained trim bias through physical actuators |
| Failed | No usable computer/electrical path; commands hold the current surfaces and actuator power is removed; there is no invented mechanical backup |

A steady disagreement without a computer-command quorum falls back to direct control. Rejoining an isolated computer requires a remaining quorum; total loss does not reconstruct an authoritative controller from unknown data. Labels show standby before the first simulated FCS sample.

## Command laws and trim

In the clean configuration, pitch blends normal-load demand with pitch-rate damping. Pilot input commands approximately -3 to +7.5 g, with the positive limit reduced for mass above the 20,000 kg reference. An outer normal-load error term and limited integrator generate a pitch-rate demand; the inner loop commands angular acceleration. Neutral input targets the configured trimmed response, not a stored attitude, altitude or heading.

Manual flap extension smoothly blends toward a damped pitch-rate response. There is no stored on-speed angle of attack or descent path. Pitch input filtering removes keyboard discontinuities without changing the existing key directions. Roll demand is scheduled with pressure, incidence and manual flap configuration. Stability-axis roll/yaw mixing, sideslip feedback and turn coordination reduce adverse yaw; they do not command a navigation heading. Positive and negative roll responses are tested separately.

Normal protection reduces and, where needed, reverses the requested pitch rate as predicted incidence or measured load approaches an estimated limit. Incidence prediction includes pitch rate. High-incidence roll authority is reduced. These are soft closed-loop protections with finite actuator authority and sensor/plant delay; external disturbances, failures and unachievable loads can exceed an intended bound. Direct mode removes these control-law protections.

Settled tail bias transfers into the existing Hornet trim value only with near-neutral pitch input, modest pitch rate, usable hydraulic pressure and airborne operation. This makes direct-mode entry retain a useful bias. It does not create an attitude or flight-path reference. Integrators are bounded and frozen/leaked under saturation or significant allocation residual. Mode transitions reset integrators and ramp normal engagement over 300 ms; surfaces always move through the physical actuator model.

Weight-on-wheels uses pilot commands with gyro damping and preserves takeoff trim. It does not invert free-flight moments against suspension/deck reactions. Ground/air transitions use contact hysteresis, preventing touchdown chatter from repeatedly changing the law. Launch rotation remains pilot-commanded.

## Surface allocation and physical boundaries

The allocator commands the two stabilators, two ailerons and two rudders separately. Manual trailing-edge flaps and leading-edge droop retain their selected configuration. A local aerodynamic Jacobian is evaluated using the voted flight measurements and current known aircraft configuration. The allocator uses normalized axis weighting, regularization and bounded coordinate re-solving. It excludes jammed, detached and severely damaged effectors, and redistributes available demand among remaining surfaces.

Travel and an estimated 220 ms rate-reachability window bound surface requests. The real plant still applies actuator travel, rate, acceleration, hinge-load proxy and hydraulic limits. A bounded secant around trim provides a direction out of a flat/clipped local polar near a stop; it does not impose the predicted force on the aircraft. The actual nonlinear aerodynamics determines achieved force and moment. Unachieved angular acceleration, surface saturation and predicted authority are retained as diagnostics.

No controller output imposes aircraft position, velocity, attitude or angular velocity, or changes throttle or flap selection. The predictor temporarily supplies voted rates/airflow to zero-time force probes and restores the original flight inputs before returning; the controller-boundary test checks this explicitly. The only aircraft bias updated by FBW is pitch trim. The old `assistance.rs` adapter is removed. State advances only on accepted physical timesteps; zero-time coefficient probes, pause and rendering cannot advance the controller.

## Parameter register

All numerical controller/sensor values below are simulation engineering estimates with low confidence as real-aircraft values. They apply to this F/A-18F plant. Bounds are proposed sensitivity ranges, not confidence intervals; a complete sensitivity campaign has not been run.

| Parameter | Nominal / units | Sensitivity range |
| --- | --- | --- |
| Keyboard command filter | .10 s | .05–.2 s |
| Voted measurement filter | .035 s | .015–.07 s |
| Rate / air / load disagreement | .15 rad/s; .08 rad α/β and 10 m/s + 12% airspeed; .6 g | .5–2 times nominal |
| FCC command disagreement / isolation / recovery | .8 rad/s²; .10 s; 2 s | .5–2 times nominal |
| FCC availability from avionics health | thresholds .10, .25, .50, .75 | ±.1 normalized health |
| Rate sensor health thresholds | .03, .18, .38, .68 | ±.1 |
| Air sensor health thresholds | .07, .22, .42, .72 | ±.1 |
| Load sensor health thresholds | .05, .20, .40, .70 | ±.1 |
| Normal pitch range | -3 to +7.5 g; 20,000 kg reference, positive limit floor 5.5 g | positive 6–7.5 g, negative -2 to -3 g |
| Pitch-rate limits | .65 rad/s clean to .45 rad/s fully flapped | ±25% |
| Normal-load feedback / integral | .12 rad/s/g; .025 rad/s/g/s, capped ±.10 rad/s | ±30% |
| Inner pitch / yaw / roll gain | 2.8 / 2.5 / 3.0 per s | ±25% |
| Inner integral | gain .4; state bound ±.18 rad; .35 s saturated leak | ±30% |
| Roll schedule | .5, 1, 2.8, 3.2, 2 rad/s at 0, 1.5, 6, 30, 80 kPa | ±25% |
| Incidence upper schedule | .60 rad below Mach .7; .40 at 1.2; .28 at 2.2 | ±.1 rad; constrained by plant evidence |
| Incidence prediction / unloading gain | .20 s; 2 per s | ±30% |
| Negative incidence / load feedback | -.28 rad; -3.2 g feedback margin; .18 rate/g | ±20% |
| Yaw pilot / sideslip feedback | .45 rad/s; .85 per s | ±30% |
| Auto-trim rate / range | .025 rad/s; ±.4 rad | .01–.04 rad/s; range ±.1 rad |
| Ground/air contact timing | >25 ms contact, below -100 ms release timer, bounded ±200 ms | ±50% |
| Normal engagement | .30 s | .15–.6 s |
| Allocation reachability / probe / secant | .22 s; ±.002 rad local; ±.4 rad around trim | ±30% |
| Allocation solve | 18 bounded sweeps, regularization .002 tails / .003 other effectors | 12–24 sweeps; regularization .5–2 times nominal |

This model intentionally keeps a bounded simulation incidence envelope. It does not interpret public marketing descriptions of high-AoA capability as a validated proprietary limiter schedule.

## Public evidence

- [Boeing, reconfigurable Super Hornet flight controls, 1998](https://boeing.mediaroom.com/1998-08-20-Boeing-Super-Hornet-Demonstrates-Reconfigurable-Flight-Controls): supports the inclusion of degraded operation and redistribution following a horizontal-tail actuator failure. It does not disclose the control gains used here.
- [NAVAIR, Super Hornet Collier Trophy overview](https://www.navair.navy.mil/node/3201): qualitative support for FBW and automatic compensation for damage/failure. It is not a numerical control-law database.
- [NASA NTRS 20010029666](https://ntrs.nasa.gov/citations/20010029666): public F/A-18 power-approach research combining dynamic inversion and control allocation. This is methodological context from a different research configuration, not F/A-18F calibration data.

The flight-control system has no validated production flight-software gain set, certified sensor model, or hardware redundancy/timing equivalence. The controller's aircraft predictor assumes known configuration and damage state and uses the simulator's structural estimate; real observer uncertainty is not fully modeled. Continued flight and fault validation must remain separate from claims about the real aircraft.

Numerical, fault, timing and native-render evidence is recorded in [FBW_VALIDATION.md](FBW_VALIDATION.md).
