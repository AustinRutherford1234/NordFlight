# F-22 inlet reference refinement

This pass supersedes the inlet and adjoining-layout description in
`F22_SIDE_BAY_INLET_REBUILD.md`. It uses the four images supplied by the user:
three exterior photographs and one illustrative internal-routing comparison.
The illustration informs the direction of the bend; it is not a production
duct specification.

## What was wrong

The exterior cowl followed the contracting inner duct too closely. That made
the housing resemble a separate horn rather than a broad shoulder flowing
into the wing root. The mouth was too short vertically, and the first duct
segments turned inward/upward too early. The immediately adjacent side bay
and oversized gear pocket also occupied space needed for a longer entrance.

## Changes

- Rebuilt the taller, swept mouth with approximately 28-degree outward cant,
  the upper corners farther outboard and
  the lower lip swept aft, using one plane and a thin coated bevel.
- Separated exterior shoulder geometry from the internal route. The shoulder
  stays broad toward the wing root while the duct bends inside it. Deliberate
  panel creases remain sharp; longitudinal joins are smoothed.
- Added an approximately 0.8 m aft-running entrance with nearly constant
  height before the inboard/upward bend. The continuous enclosed duct hides
  the engine face without a blanking plate.
- Recut the adjoining skin from the union of the exterior and inner volumes,
  and cut the side-bay opening from the cowl as well as the fuselage.
- A packaged underside capture exposed a thin shoulder panel crossing the
  inner duct. Trimming it cleared the inside but opened the outer roof in an
  upper-angle view. Raising the complete shoulder would create a broad hump.
  The final correction adds a separate narrow roof crest over the inner duct,
  with a sloping panel down to the wing-root shoulder. Its five-sided cross
  sections and matching skin-cut volumes keep the outside closed without
  crossing the inner roof. Mirrored faces retain the same triangulation
  diagonals while reversing winding. Regressions now
  verify both absence of internal crossings and coverage of every authored
  roof triangle on both sides.
- Moved the side-bay assembly 1.05 m aft and 0.06 m up, adjusted its backing
  and fitted hardware, and made the stowed rail parallel to the airframe.
  The full-size AIM-9X is clocked 45 degrees for fin clearance; its animated
  control frame and airborne mesh use that same clocking.
- Narrowed the main gear pockets around the existing tire envelope. Wheel
  positions, suspension and ground-contact physics are unchanged.
- Retained pale coated lining and added a shared depth-occlusion texture.
  This reduces flat indirect illumination deeper inside while retaining
  direct lighting. Materials and meshes are created at setup, not rebuilt
  each frame.

The hidden routing and dimensions remain visual approximations based on
public references. This pass changes presentation and shared bay launch
poses, not aircraft aerodynamic coefficients or missile guidance laws.

## Inspection evidence

The final native 1440 × 900 screenshots are in
`tests/f22-bay-inlet-rebuild/reference-accepted/`. The manifest records the binary
hash and capture settings. Front, front-quarter and near-side camera
directions match the supplied photographs; focal lengths, lighting and
backgrounds are not calibrated to those photographs. An upper-oblique view
corresponds to the supplied routing illustration.

- `tests/f22-inlet-reference-pass/photo-comparisons.jpg`: supplied photographs
  beside labeled crops from the native captures.
- `tests/f22-bay-inlet-rebuild/reference-accepted/contact.jpg`: nine-view sheet;
  uncropped originals remain beside it.
- `tests/f22-inlet-reference-pass/duct-routing.png`: side and plan plots of
  actual exported mesh cross sections, showing the aft run and later bend.
- `tests/f22-inlet-reference-pass/duct-geometry.json`: exported cross sections.

## Validation

- **426 regressions passed, zero failures** in
  `tests/f22-inlet-reference-pass/regressions-delivery.log`.
- Actual assembled triangles pass **402 door/launcher poses**, at 1% steps on
  both sides, including the full-size missile. Intentional hinge/seal mating
  contacts are excluded. Closing traverses the same path in reverse.
- A reported near-parallel contact was a positive 21-micrometre gap expanded
  into a false intersection by the test's 50-micrometre tolerance. Narrow-phase
  projection now uses double precision and does not inflate the triangles.
  New checks cover that gap, a genuine crossing and tangent contact.
- Focused regressions cover mouth orientation, delayed rise, broad shoulder,
  cutout continuity, concealed engine face, bay/gear-pocket clearance and
  shared stored/release poses. Existing launch tests cover close-combat and
  BVR sequencing, cancellation, pause/reset and ammunition accounting.

- All **13 F-22 flight-study cases** match the preserved development baseline
  byte for byte. Both output hashes are
  `ebf89491b63264ad8893504892340961f0e3c5f0f5f9f8ece617fbd70e853b72`.

Visual acceptance remains the user's review.

## Delivery

`./build-mac.sh release` completed successfully in **3 min 54 s**, packaged the
assets and ad-hoc signed `Nord Flight.app`. `codesign --verify --deep --strict`
confirmed that the bundle is valid and satisfies its designated requirement.

The previous installed app is preserved in
`tests/f22-inlet-reference-pass/baseline/installed-app-before.zip`; archive
integrity was verified. Its executable hash matches the recorded starting
installed hash. The separate baseline executable is retained only for the
short matched-release rendering check.

Final screenshots use the packaged executable through native capture mode.
They confirm native launch/rendering, rather than a manual exercise of every
cockpit control. Source hashes, output equivalence, signature verification
and capture manifests are retained under the evidence folders above.

The packaged higher-preset/alternate-daylight checks are in
`tests/f22-bay-inlet-rebuild/reference-final-validation/`. Native launch
snapshots record **zero releases at 0.67 s** with ammunition `[2, 6]`, and
**one release at 0.80 s** with ammunition `[1, 6]`. The other side stays closed.
The release-build flight study also matches the preserved installed release
in all 13 cases, byte for byte. Cargo artifacts occupy **8.4 GiB**, below the
existing 12 GiB cleanup threshold.

## Short rendering guardrail

This is one sequential release-build pair on Apple M1/Metal, 1440 × 900,
default graphics, paused simulation, external front view, existing VSync,
5-second warmup and 12-second sample. Both runs were on **AC power**; earlier
builds and native checks included battery operation. No compilation or other
benchmark ran concurrently. Raw reports and power records are under
`tests/f22-inlet-reference-pass/performance/`.

| Installed baseline / candidate | Mean FPS | p50 | p95 | p99 | Frames >25 / >50 ms |
|---|---:|---:|---:|---:|---:|
| Baseline | 55.07 | 8.22 ms | 10.42 ms | 12.24 ms | 3 / 3 |
| Candidate | 119.27 | 8.23 ms | 9.90 ms | 10.79 ms | 1 / 0 |

The baseline's few long stalls strongly skew its average. Their cause was not
established. Median frame times are essentially equal; this short pair does
not establish a general FPS improvement or a robust absence of regression.
It provides a limited rendering-cost check for this visual change, rather
than the previous broad performance campaign.
