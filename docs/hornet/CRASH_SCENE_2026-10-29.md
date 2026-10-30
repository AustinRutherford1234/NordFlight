# Hornet crash presentation

The F/A-18F now has a dedicated crash scene instead of the generic explosion and cuboid fragment effect. It is presentation driven by the aircraft's recorded impact position, velocity, energy and remaining internal fuel. It does not replace the aerodynamic, structural or collision solver, and its fragments are not independently collidable aircraft components.

## Sequence

- A brief fuel-dependent flash and 32 short-lived hot fragments mark the initial breakup.
- Twenty-four persistent wreck pieces include two fuselage sections, two engine sections, swept wing/tail panels with torn edges and exposed ribs, three wheels, and smaller bent skin panels.
- Debris retains forward momentum, tumbles, rebounds with low restitution, slides, and damps to rest. Contact height accounts for oriented piece dimensions. Panels settle flat rather than freezing on a corner.
- Sixteen dust puffs and a soft scorch mark start after debris actually reaches a supporting surface. Airborne breakup does not place dust in the sky or an immediate scorch mark far below.
- Sixteen flame layers and 32 wind-advected smoke puffs remain near the larger wreck sections when internal fuel remains. Fire dies away over 40 simulation seconds; smoke dissipates by 52 seconds. Wreckage and the impact mark persist until reset.
- The crash camera pulls back to frame the wreck field. Existing ejection and water cameras retain their behavior.

A fixed pool contains 122 independently updated entities plus 30 fixed detail children. Meshes, materials and the soft particle texture are allocated once. Rendering updates transforms/visibility without creating new materials or particles. Debris contacts are substepped at at most 1/120 second; pause and reset are respected. Carrier contacts use the existing deck outline and ship-relative velocities. Visual debris also accounts for the runway, apron, taxiway and grass slabs standing above the spherical terrain; this does not change wheel or aircraft collision physics.

The Hornet scene owns player-crash debris/fire in both normal flight and combat. Existing shared effects continue to handle AI aircraft, in-flight damage before destruction, and water spray/foam. Empty internal tanks suppress the Hornet fire/flash/smoke; water impacts suppress all dry-ground fire/dust/scorch effects. Other aircraft retain their existing wreck presentation.

These are deterministic visual approximations. They do not model fuel dispersion/combustion, fragment-to-fragment impacts, detailed penetration, or real aircraft breakup certification. The smoke uses soft textured cards, not volumetric fluid simulation.

## Reproduction

Run `cargo test --locked`, then `./build-mac.sh dev`. With permission to open disposable test flights, run `python3 tests/hornet-crash/capture.py`. It captures 0.1, 0.7, 3, 12 and 45 simulation-second poses through the same scene integrator, plus empty-tank and fast-water-impact cases. Images and logs are retained locally under `tests/hornet-crash/`; the harness is tracked.

## Validation

- Final `cargo test --locked`: **646 passed, 0 failed**, including nine crash-scene tests. Checks cover finite/settled debris trajectories, 60/120 Hz stepping agreement, pause, reset/restart visibility, fixed mesh/material counts, empty-tank fire suppression, a later transition into water, airborne dust suppression, outward-facing torn skin after settling, and vertex clearance above the visible runway slab.
- `./build-mac.sh dev`: successful optimized Apple-Silicon app build. Existing project warnings remain; no build errors.
- `codesign --verify --deep --strict "Nord Flight.app"`: passed. Target binary and installed app share Mach-O UUID `55AE6AA3-59B4-3CCE-9C6A-902001582034`.
- Native visual review: all seven final app captures completed successfully and were inspected: 0.1, 0.7, 3, 12 and 45 seconds, empty-tank impact at 3 seconds, and fast water impact. The settled panels remain visible over the runway, camera framing retains the debris field, fire ends while wreckage remains, and the empty-tank/water cases contain no fire. Capture logs contain no errors or panics. Local evidence is in `tests/hornet-crash/`.

The visual review found and corrected an inspection-camera override that cropped the debris field, inward-facing panel skins and invalid corner-fan triangles across torn notches, unlit flame materials that ignored their emission, and thin debris settling below the airport render slabs. Flame cores now share both burn sites; panel skin has a focused winding regression test. Native frames are paused deterministic inspection stages, rather than a claim of completed human flight testing or a frame-time benchmark.
