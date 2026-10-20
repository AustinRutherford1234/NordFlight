# F-22 gear and recessed exhaust refinement — October 19, 2026

This presentation-only follow-up addresses tire transparency, gear appearing
partway through extension, mechanical detail and glow beginning at the nozzle
exit. It preserves the aircraft force model, deployed wheel-contact geometry,
compression, engine output and gear-system timing.

## Causes and changes

The tire profile was revolved with inward-facing tread triangles. The opaque
rubber material culled those faces when viewed from outside. The corrected mesh
faces outward and closes the inner bead barrel; the wheel rim overlaps its bead
instead of leaving a narrow ring-shaped gap. Rubber stays opaque. This fixes
the geometry rather than disabling culling to mask the winding error.

The wheel, fork and struts were explicitly hidden until extension exceeded
0.005. Because the door-opening phase happens first, the open well could be
empty before the entire assembly became visible. The assembly now remains
present inside the bay; the doors and airframe naturally occlude it. A smooth
visual stow offset gives the folded tires clearance above the closed doors.
The offset goes to zero at full extension, preserving the physical contact and
compression pose. Bay roofs follow the upper fuselage envelope and provide
space for the stowed wheels. This remains a reference-inspired mechanism,
not maintenance-CAD geometry.

Additional details include stepped shock housings and collars, seals and
servicing placards, open brace sections, pivot bosses, cross pins, washers,
hex heads, contoured forks, axle bosses and main-wheel brake stacks. Repeated
joint fittings reuse mesh assets. Detail follows the same gear pose and health
state; destroyed gear hides its attached fittings as well.

The plume now begins 0.20 m ahead of the nozzle's local origin, inside the duct,
instead of 1.42 m aft at the lip. A recessed luminous burner face sits in front
of the dark duct backing, behind flame-holder bars. Liner ribs add depth inside
the nozzle. The existing shock cells remain downstream of the lip. The glow
and plume still respect each engine's running/health state, thrust and fuel.
Their geometry follows actual simulated nozzle commands.

The mechanical appearance was checked against the [USM main-gear photograph](https://www.f-16.net/g3/f-22-photos/F-22-main-landing-gear-trainer)
and [USM nose-gear photograph](https://www.f-16.net/g3/f-22-photos/F-22-nose-landing-gear-trainer).
These are photographic references for visible structure, not calibrated
kinematic or structural models.

## Validation

All **343 application tests pass**. New regressions check watertight tire
topology and outward tread normals, smooth motion and continued presence
through the actual door/extension sequence, closed-door clearance, unchanged
fully deployed contact pose, and recessed glow geometry. Existing tests retain
engine-shutdown, pause/reset, compression and damaged-surface coverage.

Native development and installed-release inspection records live under
`tests/fleet/f22-gear-nozzle-*`; `tests/f22-gear-nozzle/delivery.json` records
the accepted executable identity, signature check, numerical-equivalence
results and exact capture manifest. Cycle captures advance the existing gear
system to stated times, then pause for inspection. They are sampled animation
poses, not a claim of per-frame video or an FPS benchmark.
