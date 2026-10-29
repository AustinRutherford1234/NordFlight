# Hornet gear mesh review — 2026-10-29

This pass concentrates on landing-gear mesh fit and detail. It does not establish that every part of the aircraft is geometrically exact.

## Changes

- Smoothed the cylindrical strut side faces while keeping their caps flat.
- Reduced collar-ring axial thickness from .10 to .008 authored units, with a finer 32×10 ring mesh. The animated parent stretches axially, so the previous ring cross-section became oversized along the strut. The new thickness produces narrower retaining rings through the stroke.
- Raised the nose-wheel-well roof from 2 to 4 authored units. The stowed tire radius is about 2.9 units and previously intersected the roof. The modeled roof underside now clears it.
- Added roof ribs, paired hydraulic tubes and retaining clamps to all three wells. These are representative visual details rather than dimensions verified against maintenance drawings. The original aircraft textures and source asset are preserved.
- Added native close-up capture presets for nose/main gear, both extension and retraction, and a repeatable capture script.

The GLB grows from 7,655,520 to 8,088,308 bytes, from 80,768 to 89,648 triangles and from 269 to 308 nodes. This is an approximately 5.7% file-size and 11% triangle increase; it is not a measured frame-time comparison.

## Validation

`cargo test --locked`: 610 passed, zero failures (34.26 seconds while native captures were running). New checks cover link transforms and endpoints, unchanged radial scale and stowed nose-tire roof clearance.

`blender --background --python-exit-code 1 --python scripts/fa18f/validate_rig.py`: passed. Checks include required rig nodes, retained airframe area after the intentional bay cutouts, no residual source gear skins, hinge attachment, finite vertices and authoring/export axis agreement.

An initial axis experiment was discarded after native rendering and direct GLB inspection showed that Blender Z is exported as glTF Y. Runtime link-axis behavior remains the original Y convention. Failed experiment captures remain locally under `tests/hornet-mesh-review/axis-experiment/` for traceability.

`scripts/fa18f/capture_mesh_review.py` samples 0, .25, .6, 1, 1.5, 2, 2.6, 3.3 and 4.2 seconds for each gear assembly in each direction, producing 36 native app screenshots. These are deterministic animation poses, not a manual pilot review or proof that every mesh intersection is absent.

Local evidence is in the ignored `tests/hornet-mesh-review/` directory. The app is rebuilt using `./build-mac.sh dev`; the final bundle is checked with `codesign --verify --deep --strict`.

All 36 gear captures were reviewed as labeled contact sheets, along with seven store/fold/release captures and six cockpit/stabilator/overview captures. The sampled sequence shows doors opening before gear motion and closing after stowage. This is visual inspection at those sample times; no exhaustive triangle-to-triangle collision claim is made.

The final app asset hash matches the workspace asset: `dcfd1859bbb459d2e335002c17fe3f5e732d4f996c9958754ce10e753b90087d`. The final app executable SHA-256 is `85c62d5f853744e0caea201d12b346ceaf501bdedf140ace7ec21bec58b39bf6`.

## Follow-up: connected curved doors and deployed pose

The rear door arrangement had separate fore, aft, inner, outer and leg patches sharing distant hinge axes. Replaced those patches with one connected cambered shell per main bay, hinged along its inboard edge. The new skins have rounded planform corners, a .36-unit transverse camber, .18-unit thickness, conforming inner ribs and fasteners. Nose-door skins use the same curved-shell construction. Curvature is a visual estimate, not a verified maintenance-drawing profile.

All bay covers now use the greater of gear extension and door-opening demand, so they remain open with the gear deployed for landing. The physical gear sequence still opens the covers before moving the wheels and closes them after retraction.

Validation: 611 Rust tests pass; Blender checks confirm each door skin is closed and connected after welding coincident glTF material/normal seam vertices in a temporary validation copy. Twenty new native app captures (0, 1, 2, 3 and 4.2 seconds, nose/main, extension/retraction) were inspected in `tests/hornet-curved-doors/` contact sheets. The covers remain open at deployment and close at the stowed endpoint in those samples. No exhaustive triangle collision validation is claimed.

The app was rebuilt and its signature verified. Workspace and bundled GLB SHA-256 both equal `0d65f2f76c30634f44b7080f6c0c37930c849367fa91f3981d049564e096d7d2`, superseding the earlier asset hash above.
