# F-22 nozzle finish — October 19, 2026

The fixed forward nozzle housings and lateral cheeks now share the actual
fuselage paint material rather than the heat-worn exhaust material. Their
camouflage is evaluated in aircraft body coordinates, including each engine's
offset, so the gray pattern follows the surrounding airframe. Roughness and
metallicity match the fuselage because these parts reuse its material handle.

Moving nozzle sections retain a distinct metal finish with reduced grain,
staining and scuff contrast. Their texture uses a cool gray palette in place
of the previous warm tint. The duct liner and small actuator hardware retain
their existing finishes. No nozzle commands, aerodynamic forces, control
authority or articulation transforms change.

All 353 regression tests pass, including deterministic texture coverage and
the cool-gray palette check. Native top, side and downward-vector underside
captures inspect the material transition under the game lighting. These are
sampled visual checks, not a performance benchmark or a claim of calibrated
real-world coating reflectance.

The preserved installed executable and starting source archive are under
`tests/f22-nozzle-finish/`. Final packaging, signature, numerical equivalence
and native capture identities are recorded in that directory's `delivery.json`.
This finish revision supersedes the material appearance shown in
[F22_NOZZLE_BLEND.md](F22_NOZZLE_BLEND.md); the blend and actuator geometry
from that revision remain in use.
