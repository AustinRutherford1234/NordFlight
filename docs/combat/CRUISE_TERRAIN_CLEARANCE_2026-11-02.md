# Cruise-missile ground-strike repair

## Reproduced faults

The anti-ship missile plant selected zero guidance acceleration whenever its RF tracking channel returned no target. This also removed radar-altimeter/terrain stabilization: a live, powered missile fell under gravity after terminal track memory expired. A new integrated weapon regression reproduces this with an unavailable target, rather than supplying hidden target coordinates.

Separately, the old altitude controller sampled terrain only 1.5 seconds ahead. A 290 m/s missile crossing the sea corridor's western coastline struck terrain at 5.167 seconds: altitude 22.19 m against terrain at 41.20 m. This regression runs the same thrust, drag, gravity and force-limited steering equations used by the weapon plant.

## Implementation

- Run anti-ship altitude stabilization independently of target tracking. During target loss, hold the current course with no target-truth steering. Tracking can still reacquire normally. Show INERTIAL NAV with the reason `TARGET LOST / ALTITUDE AND COURSE HOLD`, rather than claiming purely ballistic flight.
- Preview terrain at 2, 4, 8, 12, 20, 30 and 40 seconds along the current horizontal course, in addition to the current surface. Use the highest required altitude and stop preview at the navigation goal, so land beyond a target ship does not induce an early climb. This is a fixed-size loop with no heap allocation; offshore future samples do not evaluate the wave spectrum. Current local wave height still participates in the altitude command.
- Keep the existing 12 m offshore / 5 m terminal clearance schedule. Over land, use a 30 m terrain margin. These values are simulation estimates, not verified specifications for a real weapon.
- Keep the existing climb, descent, acceleration and total maneuver limits. Never reposition a missile or bypass impact detection.

Terrain following does not guarantee survival from an already unrecoverable launch or terrain encounter. This remains a reduced-order estimated controller, not a validated operational terrain-following system. Spent propulsion, interceptions and real hull/ground impacts remain effective.

## Validation

Before changing production guidance, both new regressions failed against the old code: target-loss flight descended below its clearance threshold, and the coastline route struck land. After the initial repair, the focused naval suite passed 50 tests, with zero failures and one opt-in timing diagnostic ignored (8.81 seconds). This includes the new coast and target-loss checks at 60, 120 and 240 Hz, high-altitude descent, long-range cruise interception, ship missiles and CIWS.

The initial full-suite run passed 794 tests but failed the new eight-bearing route test on the 90-degree approach, striking mountainous terrain at 193.14 seconds and approximately 2,235.80 m altitude. This exposed a distinct limitation of the intermediate eight-second preview. The final implementation extends preview to 40 seconds to command the necessary climb before the slope, with seven bounded forward terrain queries. The first extended-preview trial avoided that ground strike but overshot the fleet because it sampled land beyond the target; limiting preview distance to the navigation goal corrects that separate terminal-preview error.

Automated simulation evidence is separate from native visual review and real-world flight validation. This task does not interrupt a running user flight or claim a measured rendering-performance improvement.

## Final route evidence

The integrated weapon plant now reaches the moving carrier from all eight 149 km launch bearings, without disabling ground collisions or increasing climb/turn limits. Each hit reduced carrier hull from 1.0 to 0.901. These runs retain normal propulsion, drag, seeker updates and physical hull/fuze collision checks.

| Launch bearing (fixture frame) | Impact time (s) | Impact altitude (m) |
|---|---:|---:|
| 0 degrees | 540.325 | 5.310 |
| 45 degrees | 533.458 | 5.327 |
| 90 degrees | 517.358 | 5.356 |
| 135 degrees | 501.467 | 5.322 |
| 180 degrees | 494.508 | 5.294 |
| 225 degrees | 501.608 | 5.326 |
| 270 degrees | 517.808 | 5.336 |
| 315 degrees | 534.042 | 5.251 |

Focused route test: one test passed, zero failures (0.87 s). This is simulation regression evidence, not real-missile flight validation.

## Complete regression results

`cargo test --locked`: **795 passed, zero failures, one opt-in timing diagnostic ignored**, in 21.39 seconds. This final run includes the coast and target-loss fixtures at 60/120/240 Hz, high-release sea-skimming descent, all eight long-range bearings, naval engagements, Hornet/other-aircraft physics and the existing carrier/reset regressions.

## Mac delivery

Rebuilt the existing `Nord Flight.app` using `./build-mac.sh dev`. The packaged app passed `codesign --verify --deep --strict`. Executable SHA-256: `ee915b9d278501e4fcd5aa39f3444c81c22871ecc70152a177ad33d1c3825fbc`.

The current user flight was left running; quit and reopen the app to load the new executable. No native flight interruption, visual review or new performance/soak test was performed for this task.
