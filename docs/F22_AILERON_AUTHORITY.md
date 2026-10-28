# F-22 aileron authority and trailing-edge repair

This is the requested gameplay increase to aileron authority, plus a repair of
fixed geometry left behind the moving flaperons/ailerons. The gains remain
simulation tuning values, not claimed F-22 control derivatives.

## Roll response

The two outer ailerons (surface indices 2 and 6) now have **2× incremental
aerodynamic effectiveness** at the same physical deflection. Their forces,
associated drag and moments use that gain in both actual loads and the FBW's
effectiveness probes. Other surfaces retain their existing effectiveness.
Mechanical travel remains ±25° and existing rate, hydraulic, jam and detachment
limits still apply.

The FBW's nominal roll-rate ceiling rises from 2.8 to 5.6 rad/s in attached flow
with adequate dynamic pressure. The additional command fades between 6 and
15 kPa and between 0.10 and 0.30 radians of incidence. Existing sideslip,
high-incidence, signed-G and weak-authority protections remain active. This
allows the extra power to be used in normal roll maneuvers without demanding
the same rate during post-stall or thin-air recovery.

The aileron gain does not promise exactly twice the aircraft's total rolling
moment at every state: flaperons, stabilators, nozzles, damping, inertia and
actuator saturation also contribute to the aircraft response.

## Trailing-edge geometry

The detailed wing's perimeter finish was a stationary, complete outline across
the moving surfaces. Deflecting a flap or aileron exposed that outline as a
fixed frame projecting aft. A fixed outer wedge and narrow inter-panel rib
also remained next to the aileron.

- The fixed edge finish is clipped using the same panel cutouts as the fixed
  wing skin. Each moving surface keeps its own finish attached to its hinge.
- The outer trailing wedge belongs to the aileron, and the flap/aileron split
  shares a boundary instead of leaving an exposed fixed rib between them.
- Trailing control edges are derived from the wing's trailing-edge line,
  avoiding thin leftover strips from separately rounded coordinates.
- Rendered F-22 surfaces now follow simulated individual hinge angles, including
  rate limits and failures, instead of the filtered control-demand estimate.

The clipping occurs during asset construction. Hinge rendering uses a small
fixed-size angle array and adds no render view or per-frame mesh rebuilding.

## Verification and reproduction

The roll study uses the production physics with both Q/E directions, FBW on/off
and starts at 80, 160 and 240 m/s. The 240 m/s FBW regression now requires more
than 270°/s in both directions after two seconds. BVR handling, engine-out,
fixed-step, high-incidence, takeoff and landing regressions remain part of the
full suite. A geometry regression checks that neither the fixed skin nor its
finish bridges a moving panel and that the outer trailing wedge belongs to the
aileron.

```sh
cargo test --locked --offline
cargo build --locked --offline
target/debug/gripen-flight --study-f22-roll > tests/f22-aileron-authority/after-roll.json
NORD_AIRCRAFT=f22 NORD_FLEET_CAPTURE=detail-wing-right target/debug/gripen-flight --visual-check dry
```

Other native close-up modes are `detail-wing-left`, `detail-wing-flaps-right`
and `detail-wing-neutral-right`. Saved before/after captures, traces and delivery
metadata are under `tests/f22-aileron-authority/`.

The final suite passes **495 tests**. In the two-second production-physics roll
trial at 240 m/s, FBW Q/E rates increase from +159.62/−159.71°/s to
**+319.49/−319.32°/s**. At 160 m/s they increase from about ±158 to ±213°/s,
consistent with the pressure schedule. The slower high-incidence trials also
retain finite, controlled response. Other aircraft flight-study rows and the
Su-27 cobra trace match the preceding build exactly.

Native before/after captures include both deflected wings, extended flaps and a
neutral-control check. The local Mac app is updated, ad-hoc signed and verified.
