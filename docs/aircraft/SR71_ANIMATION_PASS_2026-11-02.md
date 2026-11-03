# SR-71 animation and actuation pass

Work began at 03:39:35 UTC on November 3 (November 2 local time). Iteration took place in standalone Blender, with repeated close-up renders and a baked review timeline, rather than by repeatedly launching Nord Flight.

## Delivered mechanisms

- Three gear pivots: nose gear retracts forward without twisting around the fuselage; triple-wheel mains fold inboard. Gear stays present throughout the cycle, including full stow.
- Seven doors: paired nose, aft nose, paired main inner and paired main outer. Door opening follows the existing sequencer; they stay open while the legs move and close after full stow. Deployed legs retain open doors.
- Eight spinning wheels with their original hubs; nose steering; lower main-gear carriers follow suspension compression. SR-71 contact locations and tyre radii now use the supplied model's fitted geometry instead of generic fighter locations.
- Four independently driven elevons, both all-moving fins, both intake cones, exhaust-ring expansion and two opening canopy shells. Windscreen remains fixed. Option+O toggles the canopies while parked; G controls gear. Pitch/roll/yaw use existing controls. Spike travel is an estimated Mach schedule, not an inlet-performance simulation.
- Fourteen two-piece telescoping assemblies follow computed fixed and moving attachment points. Source-derived hinge barrels and shallow interior stiffeners accompany the moving doors.

The aircraft menu label is now SR-71A BLACKBIRD; its flight model is still provisional and it remains unarmed.

## Geometry and fidelity

The supplied CC BY model remains the basis. Gear, wheel hubs, braces, fins, elevons, spikes, nozzle rings, canopy shells, inner doors and their textures were separated/reparented rather than replaced by generic primitive parts. Rams, hinge barrels and stiffeners are derivatives of existing source mesh shapes.

The source has open wheel wells without complete outer closure skins. Seven small fitted closure patches are the deliberate procedural exception. They follow sampled original belly profiles; nearby source UVs/materials provide their finish. Recessed well liners, inner door structure and metal thickness prevent open voids through the closed underside. The finish still inherits the original asset's texture resolution and approximate panel layout.

The close-up workflow caught and repaired folded door faces, projection onto the well roof instead of the outer belly, doubled source glazing, omitted light housings, rigidly rotating nose braces, and a well cut that removed upper-wing faces. Hinges, strokes and clearances are fitted visual estimates, not certified SR-71 dimensions or a complete hydraulic model. A detailed cockpit interior and drag chute are outside this pass.

## Actuation and runtime

`src/aircraft/sr71.rs` keeps bounded actuator state in the existing Systems structure. Nominal elevon/fin rates are 0.75/0.65 rad/s, canopy travel is 3.2 s, spike travel is limited to 0.12 m/s over a 0.66 m range, and nozzle response is limited to 0.5 normalized units/s. Hydraulic damage reduces authority and can freeze motion. These rates are estimates. There is no new flight-control law or dedicated aerodynamic calibration.

`src/aircraft/visual/sr71.rs` binds named imported pivots once. Subsequent frames update bounded transform queries without scanning the whole imported hierarchy or regenerating meshes. Wheel motion uses simulation elapsed time, preserving pause/reset and time-warp behavior. Ram endpoints are calculated from the same parent poses as the visible doors/legs. The simulation continues to use its existing gear sequencer.

`rigged.glb` intentionally contains articulated nodes rather than baked gameplay animation clips: actual simulation state drives those nodes. The separate Blender review file has baked animation for standalone inspection. The old static asset is retained as provenance, not as the active runtime model.

## Reproduction and standalone review

- Build the derivative: `blender -b --python-exit-code 1 --python scripts/sr71/build_rig.py`.
- Render and bake the standalone review: `blender -b --python-exit-code 1 --python scripts/sr71/review.py -- --states up half down --views main-close nose --save`.
- Triangle clearance sweep: `blender -b --python-exit-code 1 --python scripts/sr71/validate.py`.
- Editable source: `assets/aircraft/sr71/sr71-editable.blend`.
- Local review timeline: `tests/sr71-rig/sr71-animation-review.blend`. Frames 1–16 open the doors, 16–76 extend the legs, 76–100 hold down, 100–160 retract, and 160–176 close. Press Space to play/scrub. Every frame is baked so the two ram pieces stay connected between endpoints. The review was also loaded in the native Blender application.
- Local rendered evidence and clearance JSON: `tests/sr71-rig/`. Those generated images are local inspection artifacts, not shipped textures.

## Evidence and limits

The full automated suite passed 824 tests, zero failures, one intentionally ignored timing test. The final focused SR-71 run passed six tests after asset refinements. Tests cover asset/node contracts, finite continuous gear poses, actuator limits and 60/240 Hz agreement, hydraulic loss, binding only beneath the imported root, reset restoration, ram endpoints, doors staying open through leg movement, and aircraft-menu selection.

The independent Blender triangle sweep sampled 41 gear positions with eight wheels and seven closure skins and found zero wheel/skin intersections. That does not prove clearance for every brace, hose or fuselage surface; it is specifically a tyre-versus-door-skin check. Close-up up/half/down renders supplement it. No new live Nord Flight session or performance soak was run for this task, and no FPS neutrality is claimed. Final interactive flight review remains separate.

References used for visible geometry/mechanism context:

- Jonathan Cutrer's photographed gear/doors, CC BY 2.0 (reference only, not bundled as a game texture): https://commons.wikimedia.org/wiki/File:SR-71_Landing_Gear_and_Entry_Door_(49071408071).jpg
- NASA SR-71 fact sheet (control-surface configuration): https://www.nasa.gov/wp-content/uploads/2021/09/495839main_FS-030_SR-71.pdf
- Peter Law's J58/SR-71 propulsion presentation (inlet context): https://www.enginehistory.org/Convention/2013/SR-71Propul/SR-71Propul.shtml

Model attribution and modification notices remain in `assets/aircraft/sr71/ATTRIBUTION.md`. Public appearance references do not validate estimated actuation schedules or the provisional flight model.

## Delivery build

Final runtime asset: 188 glTF nodes, 54,736 triangles, 3,480,332 bytes. Mesh growth comes from well cuts, door gauge/closure skins, and source-derived hardware. It is bounded static geometry; no claim of unchanged frame time is made.

Final Mac build succeeded (35.82 s); strict deep signature verification passed. Executable SHA-256: `df0af07a07a02556fe7ff3f392f2d27ef3b91355b287bf4d32e55bf26ef04c0b`. The final top render confirms the upper-wing openings were covered after the cut refinement. Generated closed-door renders, intermediate positions, canopy view and triangle-clearance JSON remain available in the local review folder.
