# F-22 inboard inlet mesh repair

The upper and lower inboard inlet edges now connect to the deformed fuselage
loft on both sides. This is a presentation-only repair; aerodynamic geometry,
flight dynamics, weapon behavior and damage rules are unchanged.

## Cause and correction

The narrowed forebody and inlet cowl used independently authored boundaries.
The cowl's inner edges did not consistently reach the body, while other portions
overlapped the body skin. The old seam-normal calculation also assumed the
undeformed fuselage. Together these produced disconnected skin and jagged
highlights above and below the inlet.

- Added upper and lower inboard fairings whose inner anchors sit inside the
  actual body envelope, while retaining the inlet opening and internal duct.
- Shared the exact body loft and triangle diagonals between the rendered skin
  and the cowl trimming volume. Removed buried cowl surfaces and retained the
  existing cockpit, landing-gear and weapon-bay cutouts.
- Closed the forward splitter transition without covering the intake mouth.
- Computed cowl normals before clipping and interpolated them onto clipped
  vertices. Matched the join to normals from the uncut body loft, avoiding
  fragment-dependent highlights while retaining intentional cowl creases.
- Culled unrelated clipping volumes against each original polygon's bounds.
  This affects mesh construction only; no new per-frame mesh work is added.

## Validation

`cargo test --locked -- --test-threads=1` passed **436 tests**, with zero
failures. The three added regressions cover upper/lower anchor overlap,
agreement between body trimming and rendered loft triangles, and interpolated
normal direction/validity. Existing regressions also cover aperture visibility,
duct enclosure, engine-face occlusion, and assembled side/main bay clearance.

The starting executable and the two edited source files are preserved in
`tests/f22-inlet-seam/baseline/`. Logs are in `tests/f22-inlet-seam/`.
Native captures use fixed, matching cameras at 1440 × 900. Capture manifests
record the executable hash, camera mode and lighting settings.

This focused repair does not claim a performance improvement or a new flight
validation result. Runtime screenshot inspection and final packaging evidence
are recorded in `tests/f22-inlet-seam/delivery.json`.

## Native visual evidence

Matched starting views are in
`tests/f22-bay-inlet-rebuild/inlet-seam-before/`. The intermediate `first`,
`smooth` and `checked` capture sets document geometry and shading iterations.
Accepted release views are in
`tests/f22-bay-inlet-rebuild/inlet-seam-final/`; additional preset/lighting
checks are in `tests/f22-bay-inlet-rebuild/inlet-seam-final-quality/`.

The repaired areas were checked from above and below on both sides, with front
and top views checking that the inlet mouths and outer planform remain intact.
The existing intentional lip and cowl creases remain visible.

The final app is `Nord Flight.app`, built with `./build-mac.sh release` and
verified with `codesign --verify --deep --strict --verbose=2`. Restart an
already-running instance to load the rebuilt executable.
