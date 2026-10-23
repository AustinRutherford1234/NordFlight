# F-22 lower nose and intake profile

Deepened the lower forebody progressively from the nose toward the inlet,
using the supplied airborne and runway side photographs as silhouette
references. The intake opening stays in place; the surrounding nose now meets
its lower lip instead of leaving the inlet hanging below a shallow forebody.
This is a photo-guided approximation, not a production geometry specification.

The revised profile reaches approximately 0.81 m below the section frame at
the inlet station, versus 0.505 m previously. Its transition into the existing
weapon-bay keel preserves the original aft tangent and geometry. An initial
version changed that tangent and failed the main-door sweep test; that version
was corrected before delivery.

Nose-gear door and well-rim projection now uses the deformed lower body around
the inlet, including the narrowed cheeks. The folded nose-wheel lift is reduced
by 0.11 m to use the deeper well and improve cockpit separation. The fully
deployed contact/compression pose is retained. Gear sequencing is unchanged.
The shared lower contour also updates the shaped collision envelope; no engine
or aerodynamic force coefficients were changed.

Validation: **436 regression tests passed**, zero failures, in
`tests/f22-lower-nose/tests-accepted.log`. These include folded-wheel containment,
cockpit clearance, continuous gear motion, door mesh closure, and full side/main
weapon-door sweeps. Native side, front-quarter and underside captures are in
`tests/f22-bay-inlet-rebuild/lower-nose-check/`. Packaged release verification
is recorded in `tests/f22-lower-nose/delivery.json`.

The source snapshot and previous installed executable are preserved in
`tests/f22-lower-nose/baseline/`. Restart Nord Flight to load the rebuilt app.
