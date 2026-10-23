# F-22 side-bay and inlet rebuild

**Follow-up:** the user-supplied reference-photo pass in
[`F22_INLET_REFERENCE_REFINEMENT.md`](F22_INLET_REFERENCE_REFINEMENT.md)
supersedes the inlet route and adjoining bay/gear-pocket layout described
below. The results here remain the historical validation of the first rebuild.

This update is limited to the F-22 side missile bays, their launch sequence, and
the inlet assemblies. Flight-model coefficients, missile guidance laws and
damage probabilities are unchanged.

## Reference and interpretation

- [USAF inlet inspection](https://www.holloman.af.mil/News/Photos/igphoto/2000617743/): swept thin lips, softened inner corners and light coated duct surfaces.
- [USAF launcher maintenance](https://www.holloman.af.mil/News/Photos/igphoto/2000516347/): pale interior finishes, mechanical supports, fasteners and plumbing.
- [USAF weapons-bay flight-test photograph](https://commons.wikimedia.org/wiki/File:F-22_Raptor_open_missile_doors_-_031014-F-0000A-002.jpg): extended side launchers and door arrangement.
- [USAF inlet-coating restoration](https://www.af.mil/News/Article-Display/Article/1132962/robotic-technology-developed-for-f-22s/): public evidence of specialized inlet coatings.

Visible proportions and finishes follow these photographs. The duct routing,
structural depths, linkage dimensions and timing are detailed visual
approximations; these references do not specify production internal geometry.

## Assemblies and operation

The side openings use a shared beveled outline for the airframe cutout, cavity,
door perimeter and seal. The forward floor follows the shallower fuselage.
The bays sit ahead of the main gear wells. Each has a closed backing and wall
mesh, formed inner-door ribs, a recessed frame and fastener row, catches,
bearings, fittings, supported plumbing, launcher brackets, pins, a slotted rail
and a telescoping hydraulic cylinder.

Each side has its own simulation-time door/launcher state. Opening first
unseats the supported door, then rotates it down from the lower edge. The rail deploys only after
the door is fully open. Retraction reverses that order. The launcher follows a
level outward path on fixed-length folding supports, rather than stretching bars.
The full-size existing AIM-9X mesh follows the same pose used for release.

- One accepted IR fire command queues one missile; repeated commands during
  opening do not add rounds. Ammunition and launch effects are emitted at release.
- Pause freezes motion. Scenario reset clears pending commands. Loss of the
  target, a weapon change, aircraft destruction, hydraulic failure, deployment
  of landing gear, insufficient ammunition or an invalid firing cue cancels a
  pending launch without spending the round.
- **Alt+B** retains manual inspection of both bays. Automatic launches operate
  one side at a time. The main radar-missile bay keeps its existing mechanism.

The inlet has a thinner swept surround and a rounded inner perimeter. Its
continuous S-duct closes at the engine end and conceals that end from frontal
view. Exterior cowl and fuselage use matching camouflage coordinates and shared
join normals; internal surfaces use lighter coated materials with a depth gradient.
The bay and inlet share normal/roughness detail textures created once at startup.
Rigid details are batched within their attachment groups. Animation updates
transforms only when their values change; it does not rebuild meshes.
The authored hinge location is computed once, and unchanged door/rail states
skip linkage-pose preparation altogether. Switching away from the F-22 clears
the pose cache so returning to it refreshes the mechanism.

## Evidence and validation

The original 18 native screenshots remain in
[`tests/f22-bay-inlet-review/current-source/`](tests/f22-bay-inlet-review/current-source/).
Rebuild evidence, capture manifests, logs and the preserved development baseline
are under [`tests/f22-bay-inlet-rebuild/`](tests/f22-bay-inlet-rebuild/).

### Completed checks

- **423 application regressions passed, zero failures**, on the delivery source.
  Final log: `tests/f22-bay-inlet-rebuild/regressions-cached-delivery.log`.
- **27 focused geometry tests passed.** The assembled sweep checked **402
  door/launcher poses at 1% increments across both sides**, with **8,512,493
  triangle candidates**. It checks door and loaded-rail meshes against the
  surrounding opaque airframe and fittings, and missile/rail clearance against
  the door. Intended hinge and seal mating contacts are excluded. Closing
  traverses the same geometric path in reverse. Animated link motion was also
  inspected in native captures; this is not a physical mechanism certification.
- Launch regressions cover both close-combat and BVR paths, independent sides,
  one-command deduplication, pause, reset, correct release pose and zero initial
  missile age, ammunition accounting and cancellation conditions.
- All **13 F-22 flight-study cases are byte-for-byte identical** to the preserved
  development baseline. SHA-256 of both JSON outputs:
  `ebf89491b63264ad8893504892340961f0e3c5f0f5f9f8ece617fbd70e853b72`.
- Repeated native screenshots exposed holes in the inlet's thin corner bevels:
  the camouflage subdivision produced faces below the generic degenerate-face
  cutoff. The lip now retains its authored faces while applying the same paint
  colors. A focused regression preserves its vertices and indices. The final
  front views confirm continuous coated corners.
- Default and higher graphics presets were captured, including changed daylight,
  closed/open/partially opened doors, left/right operation, an empty bay and an
  independent-side pose. Captures are native 1440 × 900 output; contact sheets
  and labeled crops are composed from those originals.

### Visual review

- [Matched before/after crops](tests/f22-bay-inlet-rebuild/before-after.jpg).
- [Nine final side-bay views](tests/f22-bay-inlet-rebuild/final-bays/contact.jpg),
  with full-resolution originals and a capture manifest in the same folder.
- [Nine final inlet views](tests/f22-bay-inlet-rebuild/final-inlets/contact.jpg).
  The manifest points to the refreshed left-front capture in `final-sequence/`;
  an earlier shot overlapping the development link is excluded from the sheet.
- [Native launch sequence, slowed](tests/f22-bay-inlet-rebuild/final-sequence/launch.gif).
  The underlying simulation-time captures show no ammunition consumption at
  0.67 s, one release by 0.80 s and no duplicate launch at 1.00 s. Additional
  higher-preset captures show rail retraction followed by door closure at 1.20 s,
  and both mechanisms fully closed at 1.50 s.
- [Higher-preset and alternate-daylight views](tests/f22-bay-inlet-rebuild/final-quality/contact.jpg).

### Package

The final `./build-mac.sh release` completed successfully in **3 min 45 s** and
packaged and ad-hoc signed `Nord Flight.app`. `codesign --verify --deep --strict` confirms
that the app is valid on disk and satisfies its designated requirement. The
previous installed app is preserved as
`tests/f22-bay-inlet-rebuild/baseline/installed-app-before.zip`. The generated
Cargo cache remains **8.3 GiB**, below the existing 12 GiB cleanup threshold.

The packaged executable launched successfully for fresh bay and inlet captures
in `tests/f22-bay-inlet-rebuild/packaged-app-final/`. This verifies native rendering
and launch in the application's capture mode; it is not a claim that every
cockpit control was manually exercised during this pass.

### Short rendering guardrail

These are **matched development builds**, Apple M1, Metal, 1440 × 900,
default graphics, paused simulation and the shipping VSync setting. No
compilation or other benchmark ran concurrently. This is a short guardrail,
not the previous broad optimization campaign or a release FPS guarantee.

| Workload | Baseline FPS | Candidate FPS | Baseline p95 | Candidate p95 |
|---|---:|---:|---:|---:|
| Both bays open, external view, final cached mechanism | 59.92 | 59.92 | 17.19 ms | 17.27 ms |
| Cockpit, earlier candidate before the pose-cache cleanup | 50.84 | 59.17 | 33.00 ms | 18.05 ms |

The final external pair used 10-second warmups and 12-second samples. Each
recorded one frame above 25 ms and zero above 50 ms. The cockpit pair used
5-second warmups and 12-second samples. Power was connected during the first
external baseline run; that battery-to-AC pair is inconclusive. Subsequent
pairs were on AC, as recorded in their process manifests.

There was substantial variation: an earlier all-AC external pair measured
118.48 versus 59.75 FPS, while the final pair measured 59.92 versus 59.92.
All samples are retained in `performance/summary.json`; no broad FPS improvement
or robust absence of regression is inferred from this short set. GPU pass
costs and the cause of the between-run variation were not established here.
The final p95 also exceeds a strict 16.67 ms frame budget.

The guardrail did identify redundant fixed-hinge and unchanged-pose work.
Caching it reduced the candidate's measured `world` CPU span from 0.402 to
0.282 ms per frame in the open-bay samples, without changing geometry or motion.
The final source again passed all 423 tests and produced identical flight JSON.

Visual acceptance remains the user's review; undocumented internal dimensions
remain approximations. The complete intermediate captures, final nine-view
sets, native launch sequence and raw performance records are retained for that
review.
