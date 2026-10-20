# Performance pass — October 15, 2026

The requested work period starts at **19:22:46 UTC (2:22:46 p.m. CDT)** and
continues through at least **20:02:46 UTC (3:02:46 p.m. CDT)**. Development
builds are used during investigation. The fully optimized release build is
reserved for the final delivery stage.

## Retained behavior and appearance

Physics timesteps, FBW authority and limits, aerodynamic equations, nonlinear
surface polars, AI/sensor decision cadence, guidance, damage probabilities,
terrain tessellation, shadow maps, antialiasing, texture resolution, particle
counts and audio synthesis retain their existing settings. No render-resolution
scaling or distance-based reduction of aircraft physics is introduced.

The frozen source inventory covers **39 Rust modules**, including tests and
diagnostics, in `tests/perf40-source-inventory.json`. The review follows runtime
paths across physics/control/propulsion, AI and weapons, collision/damage,
world/terrain/gear, cockpit/HUD/maps and audio. Static setup geometry is retained
where it does not consume recurring frame time. The existing build profiles,
vendored compatibility patch and bounded build-cache policy are retained.

## Changes

- **Prepared panel flow:** calculate each panel's local airflow, dynamic
  pressure, incidence, lift axis and neutral polar once per physics step. Reuse
  them for neutral loads, six FBW derivative probes and actual actuator loads.
  Each command still evaluates the same nonlinear deflection polar and failed
  surface state. Reuse the calculated attached-flow fraction within each polar.
- **Engine intake terms:** reuse atmospheric lapse and Mach recovery terms
  across the AI pilot's ten throttle-search probes. Keep all ten iterations.
- **Parallel BVR aircraft physics:** retain serial sensor/assignment decisions,
  then integrate disjoint aircraft in up to four batches using the existing
  compute pool for formations with at least eight NPCs. Small formations remain
  serial. Commit launches and dispenser events in the original aircraft order;
  shared damage/contact resolution remains serial. No extra pool is created in
  the native app. Headless large-formation diagnostics initialize a matching
  compute budget when needed.
- **Reusable tactical work buffers:** retain trajectory snapshots, neighbour
  contacts, decisions and launch-request capacities between BVR ticks. Stream
  candidate and avoidance iterators without constructing per-aircraft lists.
- **Exact solar and render-frame caches:** bounded thread-local single-entry
  caches reuse deterministic results for identical input bit patterns. Solar
  keys include the relevant day/orbit phases; render keys include position and
  vehicle. Direct edits, movement and vehicle changes invalidate the cache. The
  six-iteration orbital solve and double-precision render-origin subtraction
  remain unchanged.
- **Persistent instrument textures:** upload changing pixels into existing GPU
  textures instead of recreating textures and rebinding their materials. Apply
  this to MFDs, HUD, navball and damage schematic. Keep the existing update
  cadence, format, resolution and brightness. Hidden cockpit screens skip their
  raster work and redraw current data on reentry.
- **Cockpit label atlases:** consolidate the 40 Gripen button-face textures and
  materials into two shared atlases. Each label retains its original 256 × 192
  pixels, duplicated edge padding and physical geometry/animation. Switch caps
  keep their independent illumination and picking behavior.
- **Missile trail streaming:** reuse mesh attribute/index storage, hoist common
  render-frame transforms, remove a trail clone and omit unused lighting-normal
  recomputation for the existing unlit, non-shadowing material. Preserve every
  position, color, UV and index. Update conservative bounds for normal frustum
  culling instead of forcing all trails to draw. Paused unchanged trails retain
  their mesh.
- **Change detection and hidden UI:** avoid unchanged camera projections,
  aircraft root transforms/visibility, mesh handles, warp labels and particle
  visibility assignments. Skip formatting hidden telemetry. Visible text and
  every actual instrument/control change continue updating.

## Evidence and reproduction

The original BVR CPU sample is `tests/perf40-bvr.sample.txt`; panel load
evaluation and trigonometry appear prominently in its active stacks. A separate
cockpit investigation identified GPU texture recreation in Bevy's image-asset
preparation path. Diagnostic-only glass/shadow ablations did not materially
improve the warmed cockpit run, so both remain enabled.

Regressions compare prepared loads bit for bit against the original equations
over 3,200 command cases, cached solar/render values against uncached results,
and parallel versus serial aircraft with uneven batches, engine failure,
jammed/detached controls and a destroyed aircraft. Atlas tests compare every
original label pixel and its padding. Trail tests compare every vertex, color
and topology index with the original construction, validate conservative bounds
and verify steady-size position storage is reused. The complete suite has
**240 passing tests**.

`tests/compare_perf40.py BASELINE CANDIDATE` runs three alternating paired CPU
studies. `tests/compare_ai_perf40.py BASELINE CANDIDATE` compares the expanded
AI flight/combat reports and complete CSV traces, excluding only wall-time fields
from JSON. Run these separately from rendering and compilation.

`tests/compare_perf40.py BASELINE CANDIDATE render-cockpit` runs the paused
cockpit scene in before/after/after/before order, with twenty seconds of warmup
and forty seconds of measurement per run. This longer warmup excludes the
initial cockpit shader-compilation burst. Both executables use the same
development profile and render resolution. The cockpit baseline predates the
instrument-upload fix; the CPU baseline is the saved state at the start of this
40-minute pass. These are separate comparisons, with hashes retained in each
report. Development timing is not a release speedup claim.

Measured results, native captures and final packaging checks are recorded in
[VALIDATION.md](VALIDATION.md). Raw reports retain the individual samples and
variation; CPU study throughput is distinct from renderer FPS.

## Outcome

The final matched development medians improve flight throughput by 1.44×,
close combat by 1.35× and BVR by 1.46×, with exact diagnostic equivalence. The
expanded six-case flight and 28-scenario combat matrix also retains identical
reports and CSV traces across 2,894,400 NPC physics steps. The optimized release
executable reproduces those physical reports and traces as well.

Quiet cockpit FPS is effectively unchanged (32.62 before, 32.31 after) in the
four-run renderer comparison. No steady FPS improvement is claimed from that
test. Texture, trail, label and change-detection work reduces unnecessary
resource/CPU work without cutting visual settings; this test does not establish
an active combat frame-rate gain. Native posed captures were inspected, while
an additional desktop interaction check was unavailable because the Mac was
locked. No unlock bypass was attempted.

The final optimization-level-3/thin-LTO build completes in 2 min 49 s and its
ARM64 application bundle passes code-signature verification. Final delivery
hashes and a measured duration exceeding forty minutes are recorded in
`tests/perf40-delivery-manifest.json`.
