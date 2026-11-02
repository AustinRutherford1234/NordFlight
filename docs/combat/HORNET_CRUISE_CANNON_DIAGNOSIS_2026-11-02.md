# Hornet cannon versus cruise missiles: close-range diagnosis

## Report and reproduction

The player reported aiming with the funnel from approximately 30 m and seeing tracers pass close to the missile body. The previous tests manually placed a bullet across the missile centre; they did not validate the visible wings/cylindrical body or the close-range aiming cue.

New diagnostic tests against the old implementation established:

- The real U-key firing path already destroys a body-aligned cruise missile at 30 m at 60, 120 and 240 Hz. The collision code was executing.
- A swept bullet through the visible aft cylindrical body at local y = 0.23 m, z = 2.4 m was incorrectly rejected. The old ellipsoid's radius at that axial location was only approximately 0.17 m, while the rendered body radius is 0.26 m. The visible main wings and tail fins were also outside that surrogate.
- The Hornet close-gun-cue regression failed: the funnel started at 180 m and used the Gripen muzzle offset `(0.4, -0.1, -6.0)`, instead of the selected aircraft's gun origin. For the Hornet this introduces 0.4 m lateral and 0.35 m vertical muzzle disagreement, before considering longitudinal/camera parallax. Those offsets are significant compared with the cruise body's 0.26 m radius.

These findings reproduce concrete collision/cue defects; they do not prove that every tracer seen close to a missile in the user's flight intersected its geometry.

## Timestep handling and repair

The previous test already swept the full relative bullet segment rather than checking endpoints. Simply reducing the timestep would not restore omitted wings or correct the wrong shape. The new narrow phase uses the exact representative cruise render mesh, including body rings, nose/nozzle, main wings and tail fins, with a 0.01 m projectile radius.

An immutable projectile BVH is built/cached once from that mesh. It reuses the existing ship collision module's segment broad phase and swept-sphere/triangle face, edge and vertex tests. A conservative cheap world-relative box first rejects distant bullets. The ray is transformed into the same attitude used to render the missile, and translation is predicted over the current physics interval. Entry fraction is used to choose the earliest missile contact. No arbitrary large hit sphere or aim-assisted damage is added.

Cruise-body orientation is held over a physics substep; missile translation uses constant velocity over that interval. This is a reduced-order collision approximation, separate from a claim of fully continuous rotating-mesh dynamics.

A separate ordering defect was also repaired: ship intersection used to consume the bullet before testing cruise missiles, even if the bullet crossed a missile first within the same tick. The ship contact fraction now limits missile hits, allowing a missile in front of the hull to be hit while retaining ship obstruction for a missile behind it. The regression exercises both orders.

## Aiming cue

The external-view funnel now uses `aircraft::gun_origin` for the selected aircraft and starts at 30 m. A small `GUN 30 M` ring identifies the close ballistic reference. The funnel/cue remains visible without a live selected aircraft, which matters when engaging remaining cruise missiles after their bombers are destroyed. The aircraft-target director cue remains separate.

The 30 m ring is a fixed close-range ballistic reference, not a lock on a missile or an automatic lead solution. Pilot aim, gun dispersion, relative target motion and misses still apply. The same 8.4 m reference rail spacing is retained; fitting a small missile somewhere between the wide rails alone does not imply a hit.

## Automated evidence

The final full suite passed **814 tests**, zero failures, with one opt-in timing diagnostic ignored (25.99 seconds). New coverage includes actual U-key firing at 30 m across 60/120/240 Hz, between-endpoint crossings through the visible fuselage/wing/fin at those rates, a miss through empty space ahead of the wing, exact Hornet muzzle/close-cue agreement, and missile-before-ship versus ship-before-missile ordering. Existing gun, collision, cruise, ship-defense, flight and reset regressions remain passing.

Native diagnostics are separately bounded to the user's approved two-minute window. They use actual cannon firing rather than manually deleting a target; before/after views are intended to check the close cue, firing result and impact presentation. No performance/soak test is included.

## Native result and delivery

The approved native review consumed **34.33 seconds**, below its two-minute limit, with the restoration handler completing. Both before/after images were inspected. The before view shows the cruise missile aligned with the new `GUN 30 M` ring and the funnel visible without a selected enemy aircraft. The after view shows the impact, the missile removed and one interception recorded. Diagnostic output reports:

- Before: gun 412, missile alive, interceptions 0, strikes 0.
- After actual U-key firing: gun 411, missile destroyed, interceptions 1, strikes 1.

Artifacts are under `tests/cannon-cruise-diagnosis/`, including both PNGs, logs and the bounded review record. This is a scripted, body-aligned close-range firing check, not a claim of automatic hits on arbitrary manual aim or a continuous user flight review.

The existing `Nord Flight.app` was rebuilt with `./build-mac.sh dev` and passed `codesign --verify --deep --strict`. Executable SHA-256: `ee90390beea105aec2b96234bbb8360a8281eb86ffd6eb159ad218f20e20c58b`. Relaunch the app to load this version.
