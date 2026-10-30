# Carrier collision skin

The carrier now uses a collision skin derived from its imported Nimitz mesh, in addition to the existing compliant wheel/deck contacts and carrier attachment forces. Aircraft, wreckage, particles, dropped tanks, ejected bodies, missiles and cannon rounds query the same moving ship geometry.

Previously, the deck was only a height lookup inside a polygon, valid above `DECK - 4 m`. The island check used the aircraft centre against an estimated box at z=-25..55 m. The imported island's upper structure is actually around z=46..87 m. Hull sides/underside were absent, the airframe collision broad phase could reject carrier strikes based on sea-level clearance, and projectiles tested terrain rather than the ship. Fast trajectories could end below the deck's lookup window.

## Geometry and queries

`scripts/nimitz/export_collision.py` reads `assets/ships/nimitz/source.glb` in Blender without altering it. The export uses the existing visual scale/centering transform, welds coincident vertices, dissolves planar edges up to 0.5 degrees, triangulates, and excludes decal/paint overlays and light-only materials. Structural deck, hull, island, railings and equipment remain. The export contains 69,799 triangles (2,512,764 bytes); `collision.json` records its source and output SHA-256 hashes, materials and coordinate convention. This is a derivative of the credited CC BY 4.0 asset, not a surveyed real-carrier hull.

A cached bounding-volume hierarchy rejects distant triangles. It is prepared during carrier scenario setup rather than on the first impact. Convex aircraft pieces use continuous separating-axis tests against candidate triangles, retaining articulated/deformed surface poses. Attitude changes are split into short angular intervals with a conservative rotational margin; translation is swept across each complete interval. All paths transform both endpoints into their respective ship frames, so the carrier moves through the interval.

Smaller objects use swept spheres, including triangle faces, edge capsules and vertex spheres. Contacts select the earliest time of impact, not the endpoint. Missiles and rounds stop at the carrier before testing aircraft beyond it. Tanks retain the existing stop-on-impact behavior. Wrecks/debris respond in ship-relative velocity; parachuting bodies that land on deck move with the carrier. Normal suspension, arresting-wire and catapult loads retain their force-based integration.

Tires ignore the supporting deck face, which remains owned by the wheel suspension. They can strike vertical hull/island faces and raised equipment. A wheel exiting the deck from inside can roll off its outward-facing edge rather than being falsely arrested by the bow wall. Deployed Hornet tire locations/radii use the physics wheel geometry.

The collision skin adds no ship damage model and does not alter radar line of sight. Tiny decorative/paint/light overlays are not separate physical obstacles. This is a real-time triangle skin, not volumetric ship deformation.

## Validation

- `cargo test --locked`: **656 passed, 0 failed**, including the existing integrated landing, hook-up bolter, arrest-energy, catapult departure, mass variation, crosswind, and frame-partition tests.
- New tests cover swept face/edge/vertex contacts, deck/bow/stern/sides/underside/island, crossing both sides in one interval, wing-only island contact with a clear centre path, moving-frame invariance, inactive-carrier isolation, BVH agreement with exhaustive triangle queries, dropped-tank interception, ejected-body landing/transport and wreck debris retained above the moving deck.
- Native app review: six captures completed and were inspected. Deck, island, hull-side and 1000 m/s downward-impact presets all terminated with `CARRIER COLLISION` before crossing the ship surface. The full simulated arrest retained 100% aircraft health; catapult release also retained 100% health. Logs/captures are in `tests/carrier-collision/`. No errors or panics were logged.
- `./build-mac.sh dev`: successful optimized Mac build; `codesign --verify --deep --strict "Nord Flight.app"` passed. Built and installed Mach-O UUIDs match: `D1382574-959D-36A0-895A-BFB0414FA5DB`. Existing unrelated project warnings remain.
- These are automated trajectories and native paused inspection captures, not a human flight review or a frame-time benchmark.

Reproduce the approved native review with `python3 tests/carrier-collision/capture.py`. It checks deck, island, hull-side and 1000 m/s downward-impact presets, followed by actual simulated arrest and catapult release. Collision presets advance the normal physics to a fatal carrier contact, assert the collision cause and then render the paused result. Images/logs remain local in `tests/carrier-collision/`; the harness is tracked.
