# Ground handling and launch-station blast deflector

## Controls

- Hornet: A/D steers the nose wheel in normal gain.
- Hold **S** while taxiing, then use A/D for high-gain steering. This is a keyboard adaptation requested by the player, not a reproduction of the real stick-mounted NWS button.
- High gain is available below 8 m/s ship-relative ground speed (15.6 kt), with full travel below 5 m/s (9.7 kt) and a smooth transition back to normal between those speeds.
- At taxi speed S is reserved for NWS HI. At takeoff speed and airborne it commands pitch-up as before. Down arrow remains available for pitch-up at any speed.
- B applies both main-wheel brakes. Throttle controls are unchanged. NWS / NWS HI appears on the existing Hornet HUD when steering is available.
- Existing catapult attachment (Option+Shift+C) and launch (Enter) controls are unchanged. The blast deflector raises automatically while attached/preparing and stays raised through the shot, holding for four simulation seconds afterward before lowering.

## Physical changes

The Hornet's published steering travels are approximately 22.5 degrees normal and 75 degrees high gain. Rate limiting, hydraulic authority, estimated normal-gain yaw damping, and speed scheduling act on the steering actuator. Catapult attachment locks steering straight. Unloaded nose gear recentres. Running engines now maintain pump pressure at idle: the normalized idle-power signal is no longer mistaken for insufficient physical engine RPM. Ground speed is relative to the actual support surface, so a stationary aircraft carried at 25 knots does not lose taxi steering authority or spin its tyres visually.

Both the dedicated Hornet solver and the other aircraft's shared suspension now apply tyre forces at wheel contact positions. Longitudinal and lateral forces share a friction circle. Contact-point velocity includes angular motion, steering rotates the nose tyre's rolling frame, and the resulting lever arm supplies turning moments. Main wheels own wheel braking; the nose wheel rolls freely. Effective contact mass bounds low-speed impulses. The shared model no longer adds yaw directly from a held steering key or multiplies velocity by an artificial stopping factor. Its emergency height guard now permits the full suspension travel and bump-stop response before clipping penetration; the old guard clipped compression early and silently carried weight outside the contact solver. The Hornet retains its dedicated suspension.

This is a reduced-order combined-slip tyre model. It does not simulate a separate wheel-spin/anti-skid controller, tyre temperature or wear. Grip and cornering stiffness are engineering estimates: coefficient 0.65 paved, 0.35 carrier deck, 0.50 firm off-field ground; rolling resistance 0.015–0.016 paved and 0.038 off-field. The deck value preserves existing arrest/settling calibration. Off-field grip represents the game's existing firm landing terrain, not mud, ice or arbitrary real surfaces. Braking transitions sharply near rest while rolling resistance has a softer transition; no wheel force adds slip energy.

The visible Hornet tyres, twin-wheel axle and fork use the same physical steering state. Steering signs account for the imported model's coordinate frame. Wheel spin uses ship-relative ground speed.

## Deflector geometry and costs

The usable bow catapult has an original four-panel jet-blast deflector behind the parked Hornet. Estimated fitted dimensions are 14 m wide by 4 m deep, 65 degrees raised, with four-second smooth raising/lowering. The hinge is recessed so the closed upper skin meets the imported flight-deck surface. Panels have separate seams, rear stiffeners, hinge blocks, edge strips and moving support rams. These are fitted simulation geometry, not measured Mk 7 engineering drawings. The launch rail remains level.

The panel pose and its dynamic closed-box collision skin share the same hinge and rotation function. Aircraft convex sweeps, wheel/strut sphere sweeps, projectiles and debris include it in the moving ship frame. Static ship BVHs remain cached: only twelve additional stack-allocated triangles are considered for the raised panel. The deflector uses shared mesh/material handles and five moving parent transforms, with no per-frame mesh rebuilding. Hornet contacts compute mass properties/inverse inertia once per contact evaluation. No rendering-performance improvement is claimed without a same-scene benchmark.

## Public references and estimates

- [F/A-18E/F NATOPS, A1-F18EA-NFM-000, sections 2.8.2–2.8.3](https://sinref.ru/000_uchebniki/04600_raznie_15/683-F18-FLIGHT-Manual-EN-2008/018.htm): 22.5/75 degree steering, yaw-rate damping, weight-on-wheels and launch-bar behavior, hydraulic operation, main-wheel brakes. This is a public mirror of the aircraft manual. Keyboard mode selection, scheduling, damping gains and failure simplifications are game estimates.
- [NAVAIR launching systems](https://www.navair.navy.mil/product/launching-systems): raised deflector panels protect the deck and waiting aircraft during launch preparation.
- [U.S. Navy deflector photographs and description](https://www.dvidshub.net/image/6033948/air-department-jet-blast-deflector): water-cooled panel arrangement behind the catapult. Cooling and exhaust-fluid interaction are not simulated.

## Validation

The complete game test suite passed: **752 tests**, no failures (79.05 seconds). After recessing the deflector hinge to improve closed-panel fit, all **33 carrier tests** passed again (21.05 seconds), including physical launch, wire catches, moving-frame colliders and raised/lowered panel collision.

The six-second, 3 m/s coasting taxi fixture (engines off, hydraulic backup available) measured:

| Steering | Turn angle | Final speed | Final roll |
|---|---:|---:|---:|
| Normal left | 55.54 deg | 2.025 m/s | 0.271 deg |
| High left | 143.31 deg | 1.042 m/s | 0.247 deg |
| High right | 143.16 deg | 1.039 m/s | -0.244 deg |

Equivalent 60/120/240 Hz caller partitions produced matching final trajectories. Low-speed impulses and force signs are checked over both velocity directions, braking states and combined-slip conditions. Keyboard tests verify S+steering at taxi speed, S pitch at takeoff speed and airborne, and map-input isolation. Wheel/fork tests verify both sides use the shared steering axle.

The moving-carrier, 30-second idle/brakes test finished at 0.00327 m/s relative to the deck with hydraulic pressure 0.99999. Eighteen arrest scenarios covered assist on/off, -3/0/+3 degree initial bank and -6/0/+6 m lateral offset: all arrested without crashing; peak roll remained below 4 degrees and stopped roll below 1.8 degrees. The off-field 70 m/s landing/braking regression finished at 0.00012 m/s after a 40-second test window. That window is longer than the old 20-second shortcut-based assertion; it is not a measured real-aircraft stopping-distance calibration.

Native review: six initial in-app screenshots covered closed/half-raised/raised/lowered deflector states and left/right high-gain taxi poses. After the hinge fit repair, the final closed and raised states were captured and visually inspected again. Aggregate native diagnostic time was **165.6 seconds**, within the approved 240-second window; the existing running flight was suspended and resumed through each capture batch. These are scripted frozen snapshots of physical taxi states and staged animation poses, separate from the player's flight review. Captures remain locally in `tests/ground-review/`; `jbd-flat.png` and `jbd-raised.png` show the final recessed hinge, while the half/lowered captures precede that small fit adjustment.

The existing Mac app was rebuilt and its ad-hoc signature verified with `codesign --verify --deep --strict`. Raw numeric samples, test commands, native timing and the delivered executable hash are in [validation.json](validation.json). No FPS or memory gain is claimed.
