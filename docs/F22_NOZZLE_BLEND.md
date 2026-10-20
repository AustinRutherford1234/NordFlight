# F-22 nozzle/fuselage blend and actuator clearance — October 19, 2026

The fixed forward nozzle housings now follow the actual F-22 fuselage profile
at their forward rims, instead of starting with an oversized independent
cross-section. Their forward rims sit 6 mm inside the skin. A smooth loft
transitions into the existing nozzle opening, with more longitudinal sections
and smoothed normals. The aft interface, fixed sidewalls, weathered gray finish
and rear-pointing triangular flaps remain in place.

The previous actuator ran from a distant fixed anchor to an attachment on the
aft flap. At downward deflection its line crossed the flap near the forward
edge. The cylinder now runs from a raised clevis on the moving forward section
to a raised clevis on the aft flap. Shorter bodies/rods and visible bracket feet
and cross-pins keep the parts connected while clearing the panels. Each end
follows its own section's transform, including differential nozzle commands.
This is procedural presentation linkage, not manufacturer-calibrated F119 CAD.

All **353 tests pass**. New coverage checks both forward rims remain inside
the fuselage and that the loft ends at the unchanged nozzle opening. A second
sweep checks actuator cylinder/rod radii against both moving panel planes over
their longitudinal footprints, with a 5 mm clearance margin, across 21 spool
settings, 41 nozzle angles and both upper/lower assemblies. Existing hinge,
aperture, aft-tip, roll-control, damage and flight tests continue to pass.

The exact runtime mesh preview checks neutral and both vector limits. Native
captures check the top join, side/rear join, both vector limits from above,
and downward vectoring from below. These are sampled poses, not a continuous
video or a complete CAD interference analysis.

Only nozzle presentation geometry and developer capture fixtures change in
application source. The roll-direction fixes, FBW response, forces, nozzle
commands, actuator-rate limits and damage physics are untouched. Baseline
and release F-22, roll and fleet diagnostics are compared for exact equality.
Source/executable hashes, signature verification and final capture identities
are recorded in `tests/f22-nozzle-blend/delivery.json`.
