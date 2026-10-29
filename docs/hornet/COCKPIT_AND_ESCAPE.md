# Tandem cockpit and escape sequence

The Hornet now has a dedicated code-native tandem cockpit instead of the generic single-seat tub. The added interior is narrowed to fit the imported canopy; extra canopy bows draw only in cockpit view, while the exterior retains the original glazing/frame. Both stations have fitted seats, cushions, harnesses/buckles, seat rails, retaining hardware, lower striped firing handles, restraints, floor/footwell structure, console switches and canopy bows. The rear station is slightly raised and has three repeaters using the existing avionics pages. The existing selected dog occupies the front station; a different dog breed occupies the rear station. The front pilot's mesh remains excluded from first-person vision, while the rear crew remains visible.

## Fit changes

The previous nose-wheel-well roof intersected the cockpit floor and hid the seat pan. The well is now below the interior floor; nose wheels tuck inward, stagger longitudinally and rotate flat as they enter the narrow bay. New fork links connect the wheel hubs through that motion. The tire bounds were measured from the actual imported meshes rather than inferred from the slicing planes. This is estimated game kinematics rather than a manufacturer-derived retraction path. Deployed wheel positions and physical wheel contacts remain unchanged. The Hornet cabin/eye position has moved aft to fit the opening behind the fixed windscreen. Cockpit look-down travel now reaches 1.50 radians so the seat and lower controls can be inspected.

## Escape controls and stages

**Alt+Shift+E** initiates the Hornet escape sequence before aircraft destruction. **R** resets the scenario. Both crew escape; no selectable command/single-seat selector is modeled.

- The moving canopy and its frame separate using a derivative extracted from the existing GLB.
- The rear seat launches at 0.30 s; the front launches at 0.55 s. Each receives a launch impulse and brief under-seat rocket boost, with visible exhaust and launch audio.
- Stabilizing drogues appear, followed by seat separation and deployment/inflation of the larger four-color main parachutes with suspension cords.
- Crew posture changes from braced to hanging. Seats and canopy continue their own ballistic motion. The camera frames both crew members as their trajectories separate.
- The aircraft continues its existing flight integration with pilot control commands released. Escape-body trajectories use bounded substeps, gravity and estimated drag. Main parachutes stabilize the descent; contact ends the crew trajectory. Ground-contact survival/injury, detailed canopy aerodynamics, real high-speed sequencing, aircraft mass changes and liferafts are not modeled.

The existing scenario reset and aircraft selection clear the escape state. Escaped crew do not inherit the aircraft’s later destruction state. Pause stops sequence progression. Interior seats disappear only when their corresponding seat leaves; canopy details leave at initiation.

## References and limits

NAVAIR identifies NACES as the common seat for the F/A-18 family: https://www.navair.navy.mil/product/Navy-Aircrew-Common-Ejection-Seat-NACES

Martin-Baker's public NACES sheet describes the under-seat rocket, canopy jettison, interseat sequencing, drogues, leg restraints and four-color main parachute: https://martin-baker.com/wp-content/uploads/2023/11/NACES-2025-Data-Sheet-1.pdf

The geometry, timings, impulses, drag and deployment schedule here are original game approximations. They are not a validated reproduction of NACES sequencing or its real operating envelope.

## Reproduction

- `cargo test --locked`
- `./build-mac.sh dev`
- `python3 scripts/fa18f/capture_cockpit_escape.py`
- `python3 scripts/fa18f/capture_mesh_review.py --steps 0 100 200 300 420`
- Blender: `--background --python-exit-code 1 --python scripts/fa18f/validate_rig.py`

The escape canopy is regenerated with `scripts/fa18f/export_escape_canopy.py` after rebuilding the rig. It retains the existing asset's attribution and CC BY-NC-SA license; see `assets/aircraft/fa18f/README.md`.

Local native evidence is under ignored `tests/hornet-cockpit/` and `tests/hornet-mesh-review/`. Deterministic captures establish sampled visual behavior, not manual flight-review acceptance.

## Delivery evidence

The final full regression run passed **626 tests, zero failures** (22.12 s). Checks include tandem-seat mesh attributes, unchanged HUD optical scale, staged launch/reset/aircraft selection, stationary near-level escape, descent convergence, escape input/reset and separation of escaped crew state from subsequent aircraft destruction. The nose-wheel envelope uses the measured imported tire half-width (approximately 1 source unit), with clearance for the curved door's inner face.

Native inspection covered forward/seat/floor/rear/exterior cockpit views and escape ages 0, .4, .65, 1.2, 2.2, 3.6 and 8 s. The gear was also checked through both directions; ten final nose-gear poses confirm the corrected stowage, including closed doors without a protruding tire. The forward view was recaptured after the final build to verify the optical-scale correction. Blender rig validation passed, including required fork links, finite geometry, hinge attachment and connected closed door skins.

The optimized development app was rebuilt and its ad-hoc signature verified. Final executable SHA-256: `05c70566275d86a3ba44c950ff8445a81a3fab88bd732131819ce1492c73f478`. Workspace and bundled asset hashes match: rig `863988b4444347e8e14c0043e87d2296ac87c7b59d9e0f38f88fe8100d1ad982`; escape canopy `d833e32863b3b1bd6221c26ab5fecf777db1b9088bf196683f4db8946ded7003`.

Manual user flight review remains pending. The native captures are deterministic render checks rather than a claim of validated real-world escape performance.
