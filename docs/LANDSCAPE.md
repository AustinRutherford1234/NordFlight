# Landscape and off-field landing

The landscape uses bounded quadtree terrain tiles across the airfield and BVR
theater. Tiles become finer around the aircraft, reaching 8 m vertex spacing near
the ground. Floating-origin tile anchors retain precision, retired meshes are
removed, and shared height-field normals avoid visible shading seams. The same
height function supplies wheel contact, airframe collision, terrain clearance
and radar line of sight. The landscape is procedural; the lake outline is from
Natural Earth and the elevations are not a surveyed height map.

Grass and field colors, rock at higher elevations, repeating ground grain, small
rolling relief, root-and-layer pine geometry and forest patches provide detail.
Trees use batched meshes and deterministic placement. Nearby trunk and canopy
collision boxes are generated from that same placement, so forest is an obstacle
rather than a safe runway. The canopy collision volumes are approximations.

The Gripen can touch down on open, gentle ground above the former 48 m/s
off-runway cutoff. Each wheel samples terrain height and normal. Grass has higher
rolling resistance and weaker braking than pavement. Excessive sink, excessive
bank relative to the surface, slopes above 12 degrees, gear-up contact, airframe
strikes and water impacts remain destructive. This does not make rough or steep
ground suitable for a jet.

A wide, tree-free meadow is centered at simulation coordinates **x = 6,500 m,
z = 3,200 m**, elevation **62 m**, with a flat inner area approximately
420 × 2,000 m. It lies on the positive-x side of the airfield, toward its
positive-z end. Other open, gently sloping ground also accepts landing. Lower
the gear, use landing flaps, keep sink modest, and hold **B** to brake after
touchdown. The touchdown message identifies RUNWAY or OFF-FIELD.

Regression coverage includes a 70 m/s off-field touchdown and full stop, the
existing runway approach, destructive gear-up contact, an obstacle-free meadow,
tree collision boxes, outward surface normals, and bounded non-overlapping LOD
tiles. Native render checks use `terrain-field`, `terrain-landscape` and
`terrain-forest`; these posed captures are separate from dynamic landing tests.
