# F-22 articulated nozzle shape follow-up — October 19, 2026

The later [weathered gray panel revision](F22_GRAY_NOZZLE_PANELS.md) replaces
this report's square aft slabs and raised triangular shoulder decorations
with actual rectangular-plus-triangular aft outlines and a lighter texture.

This revision replaces the three small raised triangles on each rigid flap
with a broad rectangular aft flap and a larger triangular forward shoulder.
The forward shoulder and exit flap now have separate angles and a shared
hinge. This supersedes the moving-panel geometry in
[F22_NOZZLE_PANELS.md](F22_NOZZLE_PANELS.md).

## References and limits

The [SIAF 2022 close-up photograph](https://commons.wikimedia.org/wiki/File:F-22_thrust_vectoring_nozzles_at_SIAF-2022.jpg)
was inspected for the broad moving surfaces, fixed faceted cheeks and metal
finish. [Nozzle-motion footage](https://makeagif.com/gif/f-22-thrust-vectoring-nozzles-C0mygK)
was inspected in different poses for the changing flap silhouette relative to
the airframe. The clip links its [original video](https://www.youtube.com/watch?v=cTCGJAoAz_8).
[Pratt & Whitney's F119 page](https://www.rtx.com/prattwhitney/products/military-engines/f119)
documents two-dimensional vectoring and the 20-degree pitch range.

These references do not establish exact hinge locations, internal linkages,
or separate flap angle schedules. The model remains a procedural visual
approximation. In particular, the forward shoulder's vector fraction and
spool schedule are inferred animation parameters, not measured F119 data.
The separate motion should not be described as an exact reproduction of the
manufacturer's mechanism.

## Changes

- A 0.50 m forward section supports a 0.98 m broad aft flap. The forward
  rectangular backing carries one broad triangular shoulder, represented by
  two adjacent triangular facets. The old row of three small triangles is gone.
- The aft hinge position is computed from the forward section's transformed
  endpoint. Both sections use the actual simulated nozzle command, with
  different visual opening angles; the hinge remains connected throughout
  the tested command range.
- Fixed lateral cheeks, duct and burner retain their airframe attachment.
  The fixed boat-tail ends before the moving shoulder instead of covering it.
- Actuator bodies and rods follow the moving aft attachments. Edge trim sits
  on the outside of the flap instead of being buried inside its thin slab.
- Dark metallic materials, heat mottling and the recessed burner are retained.
  No propulsion, aerodynamic, damage or control-law parameters were changed.

## Validation

All **345 application tests pass**. The mechanism regression checks shared
hinge positions and actuator attachments at power extremes and both vector
limits. A new sweep checks 21 spool settings and 41 vector commands: all
sampled flap points remain finite, and the upper/lower exit sections retain
a clear aperture. This is a presentation clearance check, not fluid dynamics
or a complete mesh-intersection proof.

The developer-only `--export-f22-nozzle` path exports the actual runtime
builder and update-system output. The Matplotlib preview at
`tests/f22-articulated-nozzle/geometry-preview.png` checks neutral and both
vector-limit poses, including identical fixed geometry. It is not a PBR
renderer. Native captures additionally check the upper view, neutral,
up/down and differential vectoring, and powered exhaust. These are sampled
poses, not a per-frame motion recording or performance benchmark.

The baseline executable and starting source archive are retained in
`tests/f22-articulated-nozzle/`. Final installed-executable identity, signature,
flight-study comparisons and native capture manifest are recorded in
`tests/f22-articulated-nozzle/delivery.json`.
