# Naval SAM guidance and energy model

## Problem and change

The prior Standard/ESSM path aimed at target position plus `range / missile speed` seconds of target travel, capped at twelve seconds. That is not a constant-velocity intercept solution. It also allowed full 30/40-g turning at any speed and provided no drag cost for lift. A missed shot could repeatedly turn around toward a bomber while remaining an active commitment until the old 50/100-second maximum age.

`src/combat/naval/sam.rs` now supplies a dedicated point-mass plant for Standard-class and ESSM-class rounds. It solves the positive constant-speed intercept root, uses a heading-capture term for large launch/steering errors, and transitions to proportional-navigation correction based on relative line-of-sight rate and closing speed. Guidance receives only the existing radar/seeker/navigation-memory estimate. True target position remains reserved for existing sensor checks and the swept proximity fuse. Existing datalink, seeker gimbal, chaff and memory logic are retained.

The plant integrates at <=1/240-second steps, including mass loss, boost/sustain thrust, atmospheric density and sound speed, a transonic drag rise, induced drag, gravity, limited lift and 70-ms control response. Velocity direction is rotated by normal acceleration rather than applying a lateral Euler velocity increment that creates artificial kinetic energy. The physical speed follows thrust, drag and gravity; cruise speed is only a launch guidance estimate, not a velocity clamp.

This is an engineering-estimate point-mass model in still air, not a proprietary Standard Missile/ESSM aerodynamic database or six-degree-of-freedom fin model. The numerical values below are estimates. Existing representative seeker behavior is not a claim of an exact operational SM-2 block implementation.

## Parameter register

All quantities use SI units. Confidence is low for class-specific calibration; equations and numerical bounds have deterministic regression coverage.

| Parameter | Standard class | ESSM class |
|---|---:|---:|
| Wet / dry mass, kg | 700 / 450 | 280 / 180 |
| Boost thrust / duration, N / s | 100,000 / 4 | 35,000 / 2 |
| Sustain thrust, N | 28,000 | 20,000 |
| Total burn duration, s | 14 | 9 |
| Frontal / effective lift area, m² | 0.092 / 0.80 | 0.051 / 0.42 |
| Maximum commanded load, g | 30 | 40 |
| Effective maximum lift coefficient | 2.5 | 2.5 |
| Launch guidance speed estimate, m/s | 1,050 | 950 |
| Maximum flight age, s | 300 | 300 |

Mass decreases linearly through the burn interval. Drag coefficient is `0.32 + 0.28 exp(-((Mach - 1.1)/0.32)^2)`. The lift-induced drag term uses a factor of 0.06 and the effective lift area. PN gain is 4; large heading-error capture uses a 1.5/s gain. The latter is reduced smoothly as heading error drops between 0.25 and 0.05 radians. Positive intercept time is bounded to 75 s for steering robustness.

## Finite end-of-flight behavior (updated November 2, 2026)

A close intercept pass followed by 2.5 s of opening, eight seconds of unreachable pursuit after burnout, or insufficient speed/lift now ends guidance and transitions the round to ballistic flight. It no longer deletes the round at that point. Thrust still follows the original motor schedule; gravity, drag, swept collision and proximity checks remain active. The label becomes `BALLISTIC`.

Player/enemy air-to-air missiles and Standard/ESSM rounds have a shared **300 simulation-second** maximum age. Actual impacts can end flight sooner. Anti-ship cruise missiles retain **720 s**, with the existing 620 s motor/endurance estimate, so the 150 km engagement fixture remains possible. Longer cleanup does not extend SAM or air-to-air motor burn.

The weapon pool remains bounded; inactive entries are recycled and reuse resets ballistic/history state. A ballistic round remains allocated until impact or age expiry. This supersedes the early-retirement behavior in the original evidence below; those earlier runtime captures and retirement times are historical, not revalidated current behavior.

## Evidence and limits

Public background: [U.S. Navy Standard Missile fact file](https://www.navy.mil/Resources/Fact-Files/Display-FactFiles/Article/2169011/standard-missile/) describes the command/inertial/terminal guidance roles. [NASA proportional-navigation discussion](https://ntrs.nasa.gov/api/citations/20170001430/downloads/20170001430.pdf) describes closing-speed/line-of-sight-rate-based acceleration. Neither supplies the class-specific numerical tuning above.

All 765 game tests passed (44.87 s). Four new SAM tests cover intercept roots and command signs, burnout energy loss in turns, spent/missed/age-limited retirement, full supported-target intercepts, and 60/120/240-Hz frame-partition consistency. Full tests retain coverage of radar-support loss, navigation memory, chaff, gimbals, swept fuzes, ship collision, ammunition and kill attribution.

Single-shot fixtures use the actual naval weapon/seeker/fuse path, refreshed radar tracks, a bomber at 10 km altitude and no countermeasures or replacement shots:

| Standard-class target motion | Result | Time from launch, s |
|---|---|---:|
| Approaching 480 m/s, initial horizontal range 16 km | Intercept; fleet kill | 18.46 |
| Crossing 480 m/s, initial horizontal range 12 km | Intercept; fleet kill | 35.41 |
| Receding 300 m/s, initial horizontal range 8 km | Intercept; fleet kill | 28.25 |
| Receding 480 m/s, initial horizontal range 8 km | Escapes; round retires energy-depleted | 61.64 |

ESSM against an approaching 450 m/s bomber at 10 km horizontal range intercepted at 18.717 / 18.725 / 18.721 s for 60 / 120 / 240 Hz. Peak speed was 763.5–764.0 m/s. Standard-class peak speeds in the listed cases were 902–908 m/s. These are calibration fixtures, not measured real-weapon validation points or guaranteed engagement envelopes.

The approved native review inspected boost, terminal approach, successful hit/fleet kill and spent-round removal snapshots through the running app. It used 68.4 seconds of the approved 180 seconds and ran the session-restoration handler. Files are local under `tests/sam-review/`; the capture script is `scripts/fa18f/capture_sam_review.py`. The player aircraft is parked/frozen in these diagnostic snapshots; HUD player airspeed/altitude are not missile telemetry. SAM ages, speed and retirement state are printed in each capture log. This scripted review remains separate from the player's final flight review.

A normal-AI headless mission study ran separately with `scripts/fa18f/study_sam_mission.py`, preserving the earlier study artifact. Over 300 simulation seconds at 120 Hz on Normal difficulty, it recorded one raid, up to four aircraft and eleven airborne naval rounds, two missile interceptions, zero aircraft kills by ships, and an intact carrier (hull 1.0). This short mission sample demonstrates integration and interception activity; it does not establish a raid defeat rate or long-duration balance. Results are local in `tests/sam-review/mission-study.json`.

Measured simulation step cost was 1.660 ms mean and 1.613 ms p95 on this Mac. The mean can exceed p95 when a few expensive steps dominate the tail. These timings include the headless study workload, exclude rendering, and are not an FPS improvement comparison.

The existing `Nord Flight.app` was rebuilt with `./build-mac.sh dev` and its ad-hoc signature passed `codesign --verify --deep --strict`. Relaunch the app to load the updated executable. Native captures validate scripted snapshots, not continuous manual flight; the player's final flight review remains separate.
