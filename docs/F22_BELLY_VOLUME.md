# F-22 lower fuselage volume

The earlier model applied the same shallow `0.83 * half_height` superellipse
under the whole fuselage. Its main weapon doors were flat cuboids. This made
the aircraft look like a thin slab below the wing plane.

The revised lower skin has a fuller forward/bay keel, beveled shoulders into
the chines, and two lower engine housings with a shallow centre channel aft.
It tapers back into the existing nozzle fairings. At the central body station,
the lower depth increases from about 0.51 m to 1.04 m below the section centre.
These are photo-guided exterior proportions, not measured internal dimensions.
The upper fuselage, canopy, wing planform, nozzle exits, engine thrust locations,
aircraft mass and aerodynamic/control coefficients retain their existing values.

The follow-up intake pass reduces the early belly depth at the intake stations
and uses a monotone cubic depth curve. The lower nose approaches the central
bay along a shallow slope without the former sudden bulge at the lips.

The subsequent side-profile correction flattens the underside beneath the middle
of the cockpit further (roughly 0.48 m below the aircraft centre at z = -4.85 m).
The interpolation now smooths the actual keel before converting to the changing
section-centre frame; smoothing relative depths alone retained silhouette kinks.
The deeper central weapon-bay volume is preserved.

Side-bay apertures and their thin, closed door shells now sit below the wing
root. A straight outboard hinge and a small hinge clearance let each door swing
outward/downward without lifting its curved surface through the wing. Recessed
walls enclose the opening. Side-missile stations are aligned with these cavities
(x = +/-1.72 m, y = -0.40 m), shared by rendering and missile release.

Each swept intake lip now feeds a continuous, four-wall S-duct ending at its
engine housing (z = 5.02 m). The duct bends upward and inward above the weapon
bays before returning to the engine axis. The fuselage is clipped against the
actual stream-tube segments, rather than a rectangular hole or a short capped
recess. The only dark termination is at the distant engine inlet, concealed
behind the bends. Internal routing is an illustrative approximation, not a
reconstruction of unpublished F-22 engineering dimensions.

The intake mouth is recessed into a gray, camouflaged surround and outer cowl.
The lower lips sit higher to follow the flatter belly. Separate culled exterior
and interior surfaces prevent the dark duct from appearing as exposed exterior
skin. The former overlapping shoulder patch is removed. Both the body loft and
the cowl are clipped as their actual triangles: six tetrahedra per duct segment
share the render mesh's face diagonals, avoiding the old nonplanar-quad cuts
that left body slivers and flashing seams around the lips.

Main weapon doors and both faces of the gear doors are tessellated onto the
same lower-skin function. Weapon-bay walls follow the deeper rim. Main wheel
openings sit outside the weapon bay while preserving the original wheel and
strut contact positions. Two convex collision halves per body segment follow
the new underside and retain the channel between the aft engine housings.

## Real photographic references

- [USAF underside demonstration photo, DVIDS 8891358](https://www.dvidshub.net/image/8891358/f-22-demonstration-team-arrives-heritage-flight-2025):
  broad central weapon-bay body, bevels at the wing roots, paired aft engine
  housings and the transition to the rectangular nozzles. DVIDS credits the
  photograph to U.S. Air Force Staff Sgt. Michael Bowman in its caption.
- [USAF landing side view, DVIDS 7026841](https://www.dvidshub.net/image/7026841/3rd-wing-action-jber):
  body depth below the inlet/wing plane and gradual aft taper. Photograph by
  U.S. Air Force Airman 1st Class Andrew Britten, January 2022.
- [USAF side-weapons-bay test photo, 031014-F-0000A-002, mirrored on Commons](https://commons.wikimedia.org/wiki/File:F-22_Raptor_open_missile_doors_-_031014-F-0000A-002.jpg):
  bay position aft of the inlets, recessed cavity and outward-opening doors.
  U.S. Air Force photograph, Edwards AFB, October 2003.

The photos are reference material, not textures bundled into the game.

## Validation and repeatable views

Regression checks cover additional cross-sectional volume, left/right symmetry,
continuous station transitions, door thickness/watertightness and closed-skin
fit, folded tire clearance, and collision coverage. Existing gear, cockpit,
nozzle and F-22 flight regressions remain applicable.
The intake/side-bay regressions additionally check 101 door angles, clearance
from the wing/body/main doors, continuous duct walls and engine-axis endpoints,
unobstructed entry cutouts, separation from weapon cavities, smooth nose slopes,
and sampled sightlines from the lip to the engine face.

The native capture path accepts `NORD_AIRCRAFT=f22` and
`NORD_FLEET_CAPTURE=belly-front`, `belly-side`, `belly-bottom`, `belly-open` or
`belly-gear`, with `--visual-check dry`. These paused, engine-off views make the
lower geometry inspectable without manipulating the flight camera by hand.
Additional views: `belly-intake`, `belly-bay-side`, `belly-quarter`, `belly-half`.
`belly-profile` and `belly-profile-left` put the camera level beside the
aircraft to inspect the cockpit-belly transition on both
sides without the earlier low-angle perspective.
