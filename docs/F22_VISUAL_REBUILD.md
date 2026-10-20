# F-22 visual rebuild — October 18–19, 2026

This pass addresses the exterior silhouette, cockpit/airframe junction, coating,
landing gear and exhaust hardware. It retains the previously delivered flight,
engine, control allocation, damage and weapon models. Aircraft other than the
player F-22 are outside this pass.

The subsequent [gear/nozzle correction](F22_GEAR_NOZZLE_FIX.md) fixes inward
tire faces and delayed gear visibility, adds mechanical detail, and moves the
glow into the duct. Its validation and installed-build record supersede this
pass for those components.

## Reference study

The references were opened as actual photographs in the browser, rather than
using generated illustrations as aircraft evidence:

The user subsequently supplied an F-22 specifications three-view drawing, saved
as `tests/f22-visual/user-three-view.png`. Its planform was used to correct the
wing's fore/aft placement, clipped tips, moving-panel outlines, and six-sided
stabilators. It also resolves the relative nozzle/tail placement. The drawing is
visual reference data, not a source of instructions or a calibrated CAD model.

- [Overhead operational F-22A photograph](https://commons.wikimedia.org/wiki/File:Lockheed_Martin_F-22A_Raptor_%E2%80%9908-152_-_FF%E2%80%99_%2829102586122%29.jpg): wing/tail planform, coating regions, edge treatment and canopy footprint.
- [USMC Cherry Point taxi photograph](https://www.dvidshub.net/image/8403422/marine-corps-air-station-cherry-point-2024-air-show-f-22-demo): canopy height, fuselage blending, nose gear and surface finish.
- [USAF landing photograph](https://www.dvidshub.net/image/8238494/f-22-landing): front-quarter outline and deployed gear.
- [USAF Kadena arrival photograph](https://www.5af.pacaf.af.mil/News/Article-Display/Article/3759879/27th-fighter-squadron-arrives-at-kadena/): side profile, vertical tails and operational markings.
- [Gear-maintenance photograph](https://www.dvidshub.net/image/3526672/wash-rack-gives-f-22-clean-bill-health): metallic oleo, braces and hydraulic detail.
- [Main-gear trainer photograph, credited to USM](https://www.f-16.net/g3/f-22-photos/F-22-main-landing-gear-trainer): cast structure, wheel hub, wheel-well lining and door arrangement.
- [Nose-gear trainer photograph, credited to USM](https://www.f-16.net/g3/f-22-photos/F-22-nose-landing-gear-trainer): fork, steering assembly, chrome piston and braces.
- [Rear nozzle close-up](https://www.gazeta.press/science/2025/09/29/21761444.shtml): broad upper/lower flaps, side cheeks, recessed mechanism and heat-stained metal. Used as photographic shape evidence, not as a technical performance source.
- [NASA afterburner shock-diamond photograph](https://www.nasa.gov/image-article/diamonds-from-nasas-x-59/): continuous luminous exhaust and repeating shock cells, used as visual reference rather than a nozzle-flow calibration.
- [USAF F-22 afterburner takeoff](https://www.dvidshub.net/image/7101454/f-22-afterburner-take-off): twin-plume appearance and nighttime glow.

Public photographs constrain visible proportions and mechanisms. Hidden details
and exact maintenance geometry are reconstructed approximations.

## Presentation changes

- Tapered, faceted fixed boat-tails replace square exhaust housings. The recessed
  nozzle has layered side seals, refractory flap leaves, stiffeners, fasteners
  and internal actuator/rod detail. Fixed fairings remain attached to the airframe;
  the nozzle follows the simulated pitch-vector angle. Variable exit opening
  follows engine spool. There is no periodic steering animation.
  The visual engine/nozzle origins move 0.95 m forward, and the stabilator outline
  extends aft to the full aircraft length shown in the supplied drawing. Neutral
  nozzle exits finish approximately 1.43 m ahead of the rearmost tail corners,
  alongside the middle of the swept stabilators. Rear skin cutouts,
  engine decks and inspection-camera focus move with this change. The physical
  thrust application points remain unchanged.
- The exhaust has a narrow hot core inside a longer blue envelope, six tapered
  shock cells per engine and subtle simulation-time brightness variation.
  Cell strength, count and spacing rise through the afterburner range. The
  layers use linear HDR color, because Bevy's unlit material path bypasses
  emissive lighting. Nested translucent shells share one mesh per cell instead
  of adding entities for each shell. All exhaust follows the actual nozzle
  transform and disappears on shutdown, fuel exhaustion or engine failure.
  This is a reference-inspired visual effect, not a compressible-flow solution.
- The cockpit assembly and eye are lowered together by 0.28 m. A longer, lower
  canopy, shaped sill and matching fuselage opening replace the elevated cockpit
  junction. Glazing is reflective gold outside. Its exterior surface is culled
  from inside so it does not wash the pilot's view in gold.
  Its base follows the fuselage height instead of a constant-height rail, and a
  short opaque aft collar blends the rear glazing boundary into the dorsal skin.
  Legacy Gripen canopy rails are removed from the F-22 interior, and its HUD
  combiner moves forward/down inside the glazing rather than protruding above it.
- The rendered wings have broader clipped tips and fitted moving trailing-edge
  panels. The vertical tails have a clipped multi-edge outline, matching rudder
  contour, edge treatment and subdued markings. The visual wing/control hinges
  and fixed wing are rebuilt together; a regression checks that hinge edges
  remain seated under deflection. Navigation lights and insignia move with the
  corrected wing footprint.
- Two-tone gray camouflage uses separate irregular coating regions, subtle
  surface variation, lighter edge treatments and fine conformal access seams.
  It is a procedural finish, not a photographic aircraft skin.
- The gear has recessed lined wells, shaped thin doors, white outer cylinders,
  chrome oleos, torque links, folding braces, actuators, hydraulic runs, cast
  forks, profiled tires, wheel hubs, vents and fasteners. Wheel/fork steering,
  compression and retraction use the existing simulated gear pose. Fasteners are
  combined into local meshes to avoid one draw submission per bolt.
  Wheel rotation integrates simulated time, freezes during pause and resets with
  the scenario; airborne wheels coast down rather than tracking changing airspeed.
- Forward wheel-cover doors close at the end of the existing sequence; separate
  leg-exit doors remain open around deployed gear. The doors pivot at their edge,
  rather than translating cuboid housings through the fuselage.

The refined skin and panel contours are presentation geometry. Authoritative
aerodynamic panel definitions, control hinges and collision volumes retain their
previous parameters in this appearance-only update.

## Validation evidence

The starting source and executable identities are preserved under
`tests/f22-visual`. Native captures, each with executable/source manifests, are
under `tests/fleet/f22-visual-pass1` and `tests/fleet/f22-visual-pass2`.
The third pass, `tests/fleet/f22-visual-pass3`, checks the forward engine placement
and corrected HUD/rail fit. `tests/fleet/f22-visual-pass4` includes close main/nose
gear. The later `tests/fleet/f22-reference-final` checks the supplied three-view
corrections; these supersede earlier wing and stabilator shapes. Installed-release
checks and their exact executable/source identities are recorded separately in
`tests/fleet/f22-visual-final`, `tests/fleet/f22-exhaust-release`, and
`tests/f22-visual/delivery.json`.
They include close exhausts at neutral and both vector limits, differential
nozzle commands, cockpit, exterior, front/side/top, gear through intermediate
extension positions, closed gear, ground stance and damaged surfaces.

Capture fixtures set explicit simulated positions for inspection. They do not
add parked motion or alter normal pilot controls. Screenshot FPS counters are
not a matched performance benchmark.

All **339 application tests pass**. Identical-input F-22 and fleet studies are
exactly equal to the preserved starting executable's JSON outputs; see
`tests/f22-visual/equivalence.json`. Tests also cover finite detail meshes,
retraction-link geometry, pause/reset behavior of wheel rotation, and the
nozzle-exit/stabilator relationship. These are implementation checks, not a claim
that every hidden aircraft mechanism is reproduced exactly.

The exhaust also has a regression for independent engine shutdown/failure,
fuel exhaustion, infinite-fuel behavior and zero thrust. Native exhaust captures
cover full afterburner, dry power, shutdown and differential vectoring. Captures
are paused inspection fixtures; they establish appearance and attachment, not
flight performance or calibrated shock spacing.

The final wing has a broad root chord and a trailing edge that advances toward
the tip, rather than sweeping aft along with the leading edge. This planform
relationship has its own regression. `tests/fleet/f22-wing-fix` supersedes earlier
three-view wing captures; the installed release is the final appearance record.
