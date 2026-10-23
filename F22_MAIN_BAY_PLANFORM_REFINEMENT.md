# F-22 main weapons bay and inlet planform refinement

This pass follows the user's approval to rebuild the entire visible bottom
weapons bay and adjust the adjoining airframe around the inlets. It supersedes
the main-bay description in the earlier side-bay report and the inlet layout
in `F22_INLET_REFERENCE_REFINEMENT.md`.

## References and scope

The two user-supplied three-view drawings are preserved in
`tests/f22-mainbay-planform/reference-plan-1.png` and `reference-plan-2.png`.
The native top camera puts the nose at the left, matching the drawings.
These are comparisons of shape and relative placement, not calibrated
photogrammetry or a claim of production CAD accuracy.

The open-bay arrangement also uses the
[USAF underside photograph](https://www.dvidshub.net/image/1356704/raptor-showcases-skills-over-langley)
and the illustrated hydraulic hardware in
[the contemporary industry article](https://www.powermotiontech.com/applications/aerospace/article/21883505/hydraulics-efficiently-operate-weapon-bay-doors-on-f-22-raptor).
That article describes bifold main doors and a nine-inch launcher stroke.
The game uses that stroke, with modeled folding supports and rotary drive
housings. Hidden dimensions, gearing, internal structural detail and timing
remain visual/gameplay approximations.

## What the top and underside inspection found

- The outboard inlet corners stood forward of the wing-root leading-edge
  line, creating the horn-like projections identified by the user.
- The main bay had a dark rectangular lining and two single-piece doors.
  The radar missiles were narrowed to 70% in two axes and appeared only after
  opening passed 65%.
- Tucking the inlet back exposed a second problem in the first native front
  capture: the old broad forebody cheeks obscured the relocated openings.
  The adjoining closed loft needed reshaping, rather than another cosmetic
  rim around the mouth.

## Changes

### Inlets and adjoining skin

- Pull the outer upper mouth corners from 2.47 m to 2.15 m from the centerline
  and move them 0.90 m aft, toward the wing-root line. Preserve the opening's
  height, outward cant, swept lower lip and thin coated edge.
- Shape the forebody cheeks toward the inboard splitter boundary while
  retaining a closed loft. The deformation blends out ahead of the inlet,
  above it and behind it. Aerodynamic coefficients are unchanged.
- Update the continuous duct route: an aft-running entrance precedes the
  inboard/upward bend. Keep the engine face hidden from the mouth.
- Retain the five-sided exterior roof envelope and matching body cutouts.
  A finite 12 mm shell separates the triangulated cowl from the duct walls.
- Move the complete adjoining side bays another 0.55 m aft and adjust their
  stored rails 40 mm inboard for fin clearance. Launcher motion, stored missile
  pose and release pose remain shared. This changes physical store locations;
  it is not a claim of bit-identical loaded mass properties.

### Main bay, doors and launchers

- Replace the old box cutout with a fitted, enclosed bay and shared serrated
  fore/aft boundaries. Four thick door leaves follow the belly when closed
  and fold outward in two pairs.
- Add inner skins, formed longitudinal/cross ribs, fasteners, perimeter
  bulkheads, hinge journals, bearing brackets, rotary-drive housings, roof
  beams, hydraulic cartridges, plumbing clips, launcher shoes and hooks.
- Six detailed full-size AIM-120-style missiles remain physically present
  inside the closed bay. The doors occlude them; no visibility threshold
  makes them pop into existence. Their colors now use the authored missile
  finish. Two groups of three have staggered longitudinal stations.
- Each launcher has two articulated, fixed-length suspension-arm pairs and
  a 0.2286 m ejection stroke. Visible stores and both weapon-release paths
  use the same station and stroke calculation.
- One command queues one missile. The doors open before the selected
  launcher extends; inventory is consumed at release. The missile receives
  an 8 m/s downward ejection velocity and uses the existing 0.18 s motor
  ignition delay. Guidance laws and airborne motor behavior are unchanged.
- Retraction finishes before door closure. Simulation time controls every
  stage; pause freezes it. Invalid targets, aircraft/weapon changes, missing
  rounds, reset, destruction and blocked hydraulics cancel pending releases.
  Alt+B continues to open the assemblies for inspection.

Meshes and materials are authored once. Compatible rigid detail uses the
existing renderer batching marker within each moving attachment group;
animated launcher links retain independent transforms.

## Evidence and validation

The starting executable, selected source files and hashes are preserved in
`tests/f22-mainbay-planform/baseline/`. Native capture manifests record each
binary hash and inspection pose. The capture originals remain unchanged;
any contact sheets or comparison crops are labeled compositions.

- `tests/f22-bay-inlet-rebuild/mainbay-planform-close-before/`: matched close
  top/underside baseline views.
- `tests/f22-bay-inlet-rebuild/mainbay-planform-first/`: first geometry pass,
  including the front-view cheek obstruction that was then corrected.
- `tests/f22-bay-inlet-rebuild/mainbay-planform-refined/`: second native
  inspection pass, including empty and extended-launcher views. This pass
  exposed a narrow side-bay bulkhead protrusion in each duct; it is retained
  as diagnostic evidence, not the accepted final result.

### Correctness results

`tests/f22-mainbay-planform/regressions-delivery.log` records **433 tests
passed, zero failures** on the final source. The focused checks include:

- 404 main-door poses (four leaves at 1% increments), checked against the
  assembled opaque airframe, fixed bay hardware and full-size stored missiles.
  Bearing contacts inside the explicitly documented journal envelope are
  intentional mates.
- 606 full-size radar-missile ejection poses, checked against the fully open
  door assemblies.
- The existing 402-pose side-bay sweep, including loaded missile/door clearance.
- Shared release geometry and ammunition accounting in close-combat and BVR
  paths; duplicate-command handling; ordered opening/retraction/closure;
  timestep independence; pause; reset; and failure cancellation.
- Inlet closure, absence of cowl/duct crossings, matching skin-cut boundaries,
  concealed engine faces, and forebody aperture visibility.
- A new live-pose assembly check that rays into the affected aperture region
  hit the duct lining before side-bay hardware. The earlier vertex-only volume
  check did not catch the thin bulkhead protrusion exposed in native captures.

These are sampled geometry and software regressions, not proof of calibrated
production mechanism dimensions or hardware flight certification.

The preserved baseline and packaged candidate also produced **byte-for-byte
identical JSON in all 13 F-22 flight-study cases**, including low-speed/high-AoA
handling, acceleration, turns, approach, direct control and engine-out roll.
See `tests/f22-mainbay-planform/flight-comparison.json`. This bounds the claim
to that study; release timing and store locations deliberately changed.

### Native delivery

The optimized app is packaged at `Nord Flight.app`; strict deep code-signature
verification passed. Its executable SHA-256 is
`7136ba4c3aecba770e787c701ad0627943b25bffde28a93a880839379ae0a96e`.
Cargo reused the already-current release executable during final packaging;
the script did not need another compilation. No installed binary was accepted
solely from a development build.

Final native captures are in
`tests/f22-bay-inlet-rebuild/mainbay-planform-accepted/`. The comparison sheets
are `tests/f22-mainbay-planform/top-before-after.jpg` and
`tests/f22-mainbay-planform/bay-before-after.jpg`. The full-resolution originals
and capture settings remain beside their manifests.

The development launch capture in
`tests/f22-bay-inlet-rebuild/mainbay-launch-verified/` exercises the real
single-command launch path. The earlier multi-view runner was interrupted
before its final image was saved; that incomplete folder is retained as
diagnostic evidence and is not used as the accepted delivery set.

### Short rendering guardrail and launch verification

One sequential baseline/candidate pair ran on **battery**, at 1440 × 900 in a
paused F-22 atmospheric external view, with a 10-second warmup and 15-second
sample per build. This is a narrow regression guardrail, not a whole-game
performance result or statistically established optimization claim.

| Measurement | Preserved baseline | Packaged candidate |
|---|---:|---:|
| Average FPS | 60.00 | 59.93 |
| Mean frame time | 16.668 ms | 16.685 ms |
| p95 frame time | 17.171 ms | 17.197 ms |
| p99 frame time | 17.382 ms | 17.667 ms |
| Frames above 50 ms | 0 | 0 |

Mean frame time changed by +0.106%. Raw reports, host power information,
process samples and executable hashes are in
`tests/f22-mainbay-planform/render-check/`. No simulation-throughput or cockpit
FPS conclusion is drawn from this paused external scene.

The accepted packaged-binary set contains ten default-preset views, covering
top/front, closed/half/open doors, both filled and empty bay, launcher stroke
and real launch states at 0.55 and 0.75 simulated seconds. Two further native
views use the higher preset with a different sun phase, in
`tests/f22-bay-inlet-rebuild/mainbay-planform-quality/`. Both manifests match
the signed executable hash above. The front view no longer contains the
thin side-bay bulkhead protrusions found in the intermediate inspection.

The signed app was also launched normally through macOS. The start screen,
Return-to-fly control, F-22 scenario launch and Escape/Main Menu return were
verified through native UI interaction. The app is left at the main menu with
the F-22 selected. A large ground G-meter readout was observed during that
smoke check; this visual/mechanism pass did not diagnose that separate telemetry
issue. The 13-case baseline comparison above does not cover every ground-contact
telemetry state.

Final `git diff --check` and strict deep signature verification passed. Cargo
artifacts remain approximately 8.6 GiB, below the existing 12 GiB cache policy.
The detailed delivery hashes and evidence paths are in
`tests/f22-mainbay-planform/delivery.json`.
