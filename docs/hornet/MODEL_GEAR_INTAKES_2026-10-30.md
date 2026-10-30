# F/A-18F gear, doors, stores and intake revision

The previous nose-wheel path translated the two wheels sideways, rolled them flat about the aircraft longitudinal axis and independently interpolated its links. The new path rotates the complete wheel/knee geometry in the forward/upward vertical plane about a common transverse hinge. Both tyre axles retain their direction. The nose well is extended/deepened to accommodate upright tyres instead of hiding or flattening them. The extended wheel positions, physical contact points and suspension parameters are unchanged. Link orientation uses a stable orthonormal basis near an inverted local-Y direction.

Door shells now follow sampled perimeter heights from the original hull and closed-gear covers, with outward camber, closed thickness and continuous edges. Connected hinge feet join the straight hinge barrels to the curved skins. Inner ribs, flush fasteners, sill-shaped well walls and overlapping closure lands complete the openings. Exterior UVs project onto the existing source finish; interiors retain their own material. The original noncommercial/share-alike asset license applies to this derivative.

Fuselage AIM-120 stations move ahead of the main-gear/door sweep. The shared station definition also drives mass properties, rails and weapon release origins. Geometry checks measure 0.180 m longitudinal clearance between the missile rear tips and the main-door front edge. This is a fitted model clearance, not a manufacturer station drawing.

The intake blanking sheets are removed, and the retained inner liner is trimmed against the replacement stream tube. The new duct rises above the main-wheel well, with the well roof lowered while retaining tyre clearance. Each recessed engine face includes a fan case, 36 twisted rotor blades in one indexed mesh, a spinner and fixed stators. The two rotor transforms follow their respective engine spool states, including spin-down/windmilling already represented by that model, and freeze when paused. The 140 rad/s full-scale visual rate is an animation estimate, not measured F414 RPM. Internal routing and blade geometry are visual approximations.

The [U.S. Navy main-gear door maintenance photograph](https://www.dvidshub.net/image/5289299/us-sailors-reconnect-main-landing-gear-doors) was inspected for door curvature, inner structure and finish. The [photographer's F/A-18F walkaround](https://b-domke.de/AviationImages/Superbug/4293.html) identifies the Super Hornet nose-door arrangement. The hinge travel, generated fittings and internal engine dimensions remain estimates; this revision does not claim dimensioned aircraft CAD.

## Validation

- `cargo test --locked`: 673 passed, zero failed, including the existing carrier stability cases and a new independent-engine rotor/pause check.
- `validate_rig.py`: required nodes, accounted removed-skin area, connected hinge feet, finite vertices, exported strut axes and closed connected door shells pass.
- `validate_gear_sweep.py`: both nose and both main tyres clear the generated doors at 101 stroke samples and at closed stowage; stowed tyres do not intersect the well walls/roofs. Fuselage missile clearance passes.
- `validate_intakes.py`: ray tests of both inlets show modeled fan geometry in 43 of 49 targeted samples per side; the remaining samples hit the duct, rather than a blanking wall.
- The final GLB contains 146,424 triangles and 345 nodes, versus 95,896 and 328 previously. Its size increases from 8,279,672 to 9,809,912 bytes. The additional detail is static indexed geometry; runtime rotor work updates two transforms. No frame-rate improvement or performance benchmark is claimed.

The final native review completed 19 captures: five extension and four retraction stages for each close-up gear view, plus the intake close-up. Selected initial, intermediate, deployed and closed states were inspected directly. The nose tyres remain upright, the main tyres pass their open doors, the closed door skins cover the bay openings, and the intake fan is visible through the smooth liner. No application errors or assertions were reported. These are sampled scripted running-app checks, separate from the user's interactive flight review; rotor speed/pause behavior was checked by the automated visual-system test. Any previously active game process was resumed after the captures.

Local native evidence is in `tests/hornet-model-2026-10-30/`; the tracked runner is `scripts/fa18f/capture_model_review.py`. Generated screenshots and logs remain local.

`./build-mac.sh dev` rebuilt the Mac app. The final generated assets were synchronized into the bundle after the intake refinement and the app was signed again. Build and packaged ARM64 UUIDs match: `743E3A4B-68ED-3066-90A6-DA03160B35CD`. Strict deep signature and asset hash checks passed.
