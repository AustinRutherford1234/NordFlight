# F-22 fixed sidewalls and broad nozzle flaps — October 19, 2026

The subsequent [articulated-nozzle revision](F22_ARTICULATED_NOZZLE.md)
supersedes this report's three-triangle moving-panel arrangement.

This visual follow-up replaces the narrow slat assembly with broad square
upper/lower flaps and raised triangular panels, using the user's clarification
and dark, heavily weathered metal preference. The aircraft's force laws,
engine output, nozzle-command limits and damage model remain unchanged.

## Geometry and animation

The prior nozzle parent rotated the entire assembly, including lateral cheeks,
duct, burner and trim. The new fixed frame attaches to the airframe. Sidewalls,
faceted side panels, duct and burner remain fixed. Each upper/lower flap pivots
at a fixed hinge and combines the existing simulated pitch-vector command with
the existing spool-dependent opening angle. Actuator bodies and rods span
fixed anchors and moving flap attachments. The exhaust jet and shock cells
follow the vector command separately; their recessed origin remains inside the
engine. There is no decorative TVC movement independent of simulation.

Each broad flap has three shallow triangular external panels, thin worn edge
trim and deterministic heat mottling. The old narrow rectangular leaves and
their longitudinal ribs are removed. The repeated raised liner bars inside
the duct are also removed. Deep flame holders remain. Materials use a metal
response with roughened dark faces and slightly smoother exposed edges.

The [SIAF 2022 nozzle photograph](https://commons.wikimedia.org/wiki/File:F-22_thrust_vectoring_nozzles_at_SIAF-2022.jpg)
was inspected directly for flap proportions, faceted fixed cheeks and surface
appearance. [Pratt & Whitney's F119 documentation](https://www.rtx.com/prattwhitney/products/military-engines/f119)
establishes the two-dimensional pitch-vectoring nozzle. Public photographs
guide visible shape; the mechanism is a procedural approximation, not an
exact reconstruction of manufacturer CAD or classified nozzle internals.

## Faster geometry inspection

The executable now has a developer-only `--export-f22-nozzle` command. It calls
the **same nozzle builder and update system used in the game**, then exports
neutral and both vector-limit poses without starting the window, renderer,
terrain or audio. `tests/f22-nozzle-panels/preview.py` plots those exported
triangles with Matplotlib and asserts that fixed geometry does not change
between commands. There is no duplicate Python nozzle model to drift from the
Rust implementation.

Run from the project directory:

```sh
target/debug/gripen-flight --export-f22-nozzle > tests/f22-nozzle-panels/mesh.json
python3 tests/f22-nozzle-panels/preview.py
```

The plot checks shape, attachments and motion. Its simple lighting does not
validate metallic reflections, bloom, transparency or integration with the
airframe; native captures provide that second check. Measured export/plot
times are in `tests/f22-nozzle-panels/preview-timing.json` and exclude compiling
the executable. A compiled mesh edit still requires a development build.

## Validation

All **344 application tests pass**. The new flap/actuator regression checks
both vector limits and power extremes, fixed hinge positions and connected
rod endpoints. The standalone preview also checks identical fixed geometry
across vector commands. Native checks cover neutral, up/down and differential
TVC, plus powered exhaust. These are sampled animation poses, not a claim of
per-frame video or a performance benchmark.

The accepted signed executable, baseline numerical-equivalence results and
final capture identities are recorded in `tests/f22-nozzle-panels/delivery.json`.
