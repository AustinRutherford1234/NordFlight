# Centre AIM-120 mounting repair

The two fuselage stations (indices 6 and 7) had launcher rails below the missiles but no adapter joining those rails to the imported inlet/fuselage skin. Added a swept, tapered adapter for each rail, an aft rail extension, a lower attachment plate, and visible fastening hardware. The adapters are fitted original geometry, not a claimed exact BRU-series equipment replica.

Station coordinates remain (+/-1.22, -0.52, -0.85) metres in aircraft body coordinates. Attached missile transforms, mass properties and released-weapon origins retain their shared station definition. Existing launch rails and the new brackets remain attached to the aircraft when the missile departs.

The bracket profile follows the imported underside around body z=-0.30 to +0.12 m; its upper skin overlaps the fuselage, and its base overlaps the rail/attachment plate. Each adapter has a closed 36-triangle bracket shell plus a 12-triangle rail extension; both use the existing aircraft root with static transforms and shared materials. No per-frame mesh generation was added.

## Automated validation

- All 758 game tests passed (77.33 s), including the actual AIM-120/adapter mesh export fixture.
- `scripts/fa18f/validate_shoulder_mounts.py` uses actual rendered missile geometry exported by that Rust test and actual imported GLB gear-door shells. It checks both missiles and adapters against every door at 201 opening angles, plus all four tyre paths at 101 retraction samples. No intersections were found.
- Both adapters intersect the fuselage skin, verifying a physical visual connection rather than a floating bracket.
- The existing full gear-sweep validator also passed. The missile rear tips remain 0.180 m forward of the main-door sweep. Door-angle clearance is checked with triangle BVHs, not just the tip-distance bound.

Run the Rust `shoulder_mount_and_actual_missile_mesh_export_for_sweep_validation` test first, then Blender with `--background --python-exit-code 1 --python scripts/fa18f/validate_shoulder_mounts.py`.

## Native review

The approved three-minute diagnostic uses `scripts/fa18f/capture_shoulder_mounts.py` and an aggregate time budget in `tests/shoulder-mount-review/review-budget.json`. It pauses and resumes any running flight and captures close views of the actual app meshes through representative extension/retraction states. Native results are recorded after inspection; this is separate from the player's final flight review.

Final verification: after adding the short aft rail extensions, the actual-mesh export test and complete Blender store/door/tyre intersection check passed again. Six native close-up snapshots were inspected: closed gear, opening doors, intermediate extension, fully extended gear, intermediate retraction, and the opposite-side mount. No visible intersections were found. Aggregate diagnostic runtime was 100.0 seconds of the approved 180 seconds; the original running flight was resumed. The final app was rebuilt and passed `codesign --verify --deep --strict`. Captures remain in `tests/shoulder-mount-review/`.
