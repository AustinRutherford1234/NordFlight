# F-22 moving nozzle: aft triangular tip — October 19, 2026

The previous exit-panel outline pointed its triangular extension toward the
nose. This correction puts the straight rectangular edge at the forward hinge
and the triangular tip at the aft end, away from the nose. All four moving
upper/lower flaps use the corrected perimeter, including the worn edge trim.
The gray weathered finish and existing hinges/actuators are preserved.

Only `src/aircraft/visual/raptor_detail.rs` changes in application source.
The flight model, corrected Q/E roll mapping, FBW response, differential thrust,
nozzle commands and stabilator effectiveness are untouched.

All **351 tests pass**. The outline regression explicitly requires the centered
triangular tip to be the aftmost vertex. The aperture sweep includes the new
tip with the actual skin offset, and the stabilator-clearance regression now
uses the actual transformed tip instead of a stale hard-coded exit position.
The exact runtime mesh preview and native top/up/down captures confirm the
direction in neutral and vector-limit poses. They are sampled poses, not a
per-frame motion recording or a performance benchmark.

Release signature, source/executable identities, baseline numerical comparisons
and capture manifests are recorded in `tests/f22-aft-nozzle-tip/delivery.json`.
