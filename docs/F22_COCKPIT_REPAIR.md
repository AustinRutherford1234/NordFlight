# F-22 cockpit interior repair — October 20, 2026

The F-22 now has a separate fitted cabin and ejection-seat assembly instead of
the shared cockpit tub. Camera eye position, display positions and cockpit
control/picking coordinates are unchanged.

## Confirmed causes

The nose-wheel well used a wheel-height ceiling across its entire length. That
opaque light-gray surface passed above the cockpit floor, hid the front of the
seat and cut through the lower radar-display area. Mesh-intersection tracing
identified the wheel-well material and geometry as the visible obstruction.

The cabin opening also stopped above much of the interior, and its long generic
sidewalls extended into the panel area. Those now terminate behind the display
faces, with the cabin hollowed to its lined lower floor.

During visual iteration, the new upholstered mesh exposed a batching issue:
missing UV attributes produced inconsistent vertex buffers when combined with
ordinary cuboids. The seat meshes now provide complete, indexed vertex
attributes, including UVs, and are compatible with rigid batching.

## Final geometry

- A stepped nose-wheel-well ceiling retains full height around the forward
  stowed tire, lowers over the strut pocket and lowers again beneath the seat.
  Matching well ribs and sidewalls follow the partition. Door outlines and
  gear actuation remain unchanged.
- A dark matte raised footwell, lower seat floor and closed kick panel cover
  the compartment. Pedals sit above the raised footwell.
- The seat has a chamfered padded pan, reclined back, headrest, side bolsters,
  guide rails, support frame, shoulder/lap restraints, buckles and striped
  center pull handle. It is a game-model interpretation, not an exact certified
  ejection-seat reconstruction. Public USAF material identifies the F-22's
  [ACES II seat](https://www.af.mil/News/Photos/igphoto/2000060351/).
- Compatible fixed cabin and well parts use the existing rigid batching path;
  clickable and animated controls retain their separate entities.

## Validation

All **396 Rust regressions passed**. New checks cover the first opaque surface
beneath the cockpit eye at the footwell, seat and both radar-panel sides; complete
vertex attributes after seat-mesh batching; and tire clearance below the stepped
nose-wheel ceiling. Existing gear sequencing, door/tire clearance, flight and
display tests also pass.

Two native development capture passes were inspected. The first exposed the
remaining white well ceiling and headrest corruption; the second confirmed the
dark footwell, visible seat, solid headrest and unobstructed lower radar screen
from right, left, downward and rear-facing cockpit views.

Developer captures use `--visual-check cockpit`, `NORD_AIRCRAFT=f22` and
`NORD_FLEET_CAPTURE=cockpit-right`, `cockpit-left`, `cockpit-floor` or
`cockpit-seat`. These switches pose the normal native renderer; they do not
replace geometry with an illustration.
