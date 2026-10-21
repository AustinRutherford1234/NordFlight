# Focused atmospheric rendering optimization

## Goal and boundaries

Target 60 FPS at 1440 × 900 on the M1 MacBook Air. Preserve both graphics
presets, geometry, texture resolution, shadows, lighting, effects, working
instruments, refresh behavior and simulation/AI fidelity. Work is limited to
atmosphere, shadows/transparency, submission and synchronization.

Implement from source inspection first. Run correctness tests, native visual
checks, Metal diagnostics and acceptance comparisons together at the end.
The installed signed executable and source/asset hashes are preserved under
`tests/atmospheric6/baseline`. No engine migration or gameplay changes are part
of this pass.

## Implementation

### Atmosphere and indirect lighting

1. Cache atmosphere and probe bind groups by complete buffer identity/range,
   texture-view identity, sampler and layout. Continue supplying current dynamic
   uniform offsets at dispatch. Missing bindings must retire stale groups.
2. Split static scattering dependencies from view-dependent lookup settings.
   Keep scattering formulas, sample counts, formats and resolutions unchanged.
3. Build an independent canonical sky lookup for indirect lighting. Generate and
   sample it with matching axes; keep the visible sky/aerial path camera based.
   Exclude camera rotation only from the canonical probe's source identity.
4. Key indirect results on observer position, planetary/render frame, sunlight,
   medium, weather, settings and resource/pipeline generations. Preserve
   camera-relative precision and invalidate incomplete work on discontinuities.
5. Reuse exact unchanged visible sky/aerial results only with all shader inputs
   and outputs accounted for. Recompute for jitter or unsupported projection
   overrides. No movement thresholds or approximate keys.
6. Preserve immutable source capture, front/back lighting, six-stage filtering
   and atomic complete publication. Full initialization and discontinuous updates
   remain immediate. Never expose partially populated outputs.

### Shadows and transparency

Audit opaque airframe, cockpit structure, glass, displays, HUD, plumes and damage
layers. Preserve existing valid exclusions, light-space culling, cascade count,
resolution, distance, filtering and moving shadows. Fix participation only when
its output demonstrably cannot affect the image.

Reuse a cascade only when its depth target has persistent, proven ownership and
all matrices, relevant caster geometry/poses/visibility/alpha-test inputs and
pipelines match. Any changed dependency rerenders it. Pooled depth targets with
unproven ownership must continue rendering normally. Do not add fixed-rate updates.

Only guaranteed opaque surfaces may enter the opaque pass. Existing numeric
backgrounds and label atlases are already opaque. Genuine glass, HUD and
transparent radar/effect pixels remain transparent. Preserve exterior geometry
that contributes color, depth, reflections or shadows in cockpit views.

### Submission

Use an explicit `RigidGeometry` author marker. Batch opaque rigid leaves only
within a common parent, material, render-layer and shadow-behavior group.
Exclude independent animation, clickable targets, damage-controlled pieces,
detachable parts, glass and effects. Retain structural entity IDs, remove their
individual render mesh components, and attach the combined mesh to their parent.
Use the same authored geometry in the reference path. Preserve spatial culling
boundaries; do not combine whole forests or distant objects.

Initial eligible groups are short canopy rails and frame segments. They inherit
vehicle/root motion and visibility. Test transformed vertex positions, attachment,
excluded leaves and parent visibility/damage behavior before acceptance.

### Display handoff and frame pacing

Separate producer comparison/storage from the render-consumer queue. Transfer
owned complete revisions through a short handoff lock. Pack dirty GPU regions
and issue queue writes outside that lock. Union unflushed regions, retain the
newest complete revision, handle resize/replacement readiness and bound recycled
storage. Retirement must prevent older work from resurrecting removed images.

Preserve input → simulation → camera/origin → presentation ordering. No GPU
readback, busy-wait or synchronous compilation loop enters ordinary rendering.
Retain shipping VSync. A developer-only frame-latency override permits final
comparison of one/two-frame limits; change the default only with repeatable pacing
benefit and no tearing, input-delay or material average-FPS regression.

All new types remain internal. `NORD_RENDER_REFERENCE` selects reference rendering
behavior for new atmosphere/batching work and full environment filtering.
`NORD_PROBE_REFERENCE`,
`NORD_LUT_REFERENCE` and `NORD_BATCH_REFERENCE` isolate individual changes.
`NORD_FRAME_LATENCY=1|2` is diagnostic only. Existing generation/readiness
safeguards stay active in every mode.

The reuse gate must count active extracted 3D cameras, not retained `Camera3d`
components belonging to inactive menu views. A focused ECS regression covers
inactive menus, non-3D extracted cameras, genuine multiple active cameras and
deactivation. This correction enables the already-implemented scheduler during
ordinary flight; it does not lower its samples or output resolution.

## Final validation and acceptance

1. Run focused invalidation, probe-axis, publication, batching/attachment and
   concurrent display-handoff tests, then the application regression suite.
2. Compare authoritative seeded baseline/candidate outputs once, excluding timing
   fields only. Presentation changes must not affect simulation results.
3. Capture matched cockpit/external views and motion for sun/weather changes,
   glass, shadows, damage and terrain streaming in both presets. Reject visible
   regressions, missing geometry, stale lighting or altered controls.
4. Obtain a focused Metal capture for paused cockpit and live BVR. Report valid
   GPU durations, CPU submission and presentation waits separately. Record
   unavailable counters explicitly.
5. Benchmark paused cockpit, live BVR cockpit and deterministic low-altitude
   terrain streaming sequentially against the preserved baseline. Use matched
   release builds, 10-second warmups and 25-second samples. Repeat inconclusive
   comparisons or >5% regressions once initially.
6. Run scripted transition captures and a two-minute mixed-scene stability check.
   Inspect switching, transparency and bounded resources. Keep capture storage
   bounded and preserve the existing 12 GiB Cargo-cache policy.
7. Retain changes with repeatable targeted benefit and no visible regression.
   If six-stage lighting lags visibly, test three stages and full updates. Revert
   or keep developer-only optimizations whose costs exceed their benefit.
8. Package/sign the accepted app, verify native launch, and publish measured
   frame times, stutters, GPU costs, correctness, visual evidence and remaining
   bottlenecks in `docs/ATMOSPHERIC_RENDERING_RESULTS.md`.

Use external power and stable host conditions; record deviations. Do not reduce
resolution, effects, shadows or simulation/AI work to meet the frame budget.
Sixty FPS remains a target; an unmet target requires a measured explanation,
not an unsupported hardware/engine-limit claim.
