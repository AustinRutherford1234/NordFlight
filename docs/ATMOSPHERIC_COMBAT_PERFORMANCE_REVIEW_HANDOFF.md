# AI Handoff: NORD Flight Atmospheric Performance Review

## Objective and constraints

Review the current NORD Flight code and produce a detailed report identifying the greatest sources of low FPS, lag, and frame-time spikes **inside the atmosphere, especially during combat close to terrain**. Recommend concrete fixes and assess whether switching engines would be worthwhile.

**Do not implement any optimization or engine migration. This handoff authorizes production of the report only.**

User decisions:

- Target hardware: Apple M1 MacBook Air, 8 GB unified memory.
- Primary workload: low-altitude combat.
- Give fullscreen cockpit and external views equal priority.
- Target steady 60 FPS, corresponding to a 16.67 ms frame budget.
- Preserve existing visual and simulation fidelity.
- Use current source and saved performance evidence; do not run fresh benchmarks or profiling.
- Engine comparison should emphasize performance, visual quality, development tools, and migration cost.

Workspace: `/Users/Austin/Downloads/GripenFlight`

Deliver the report as:

`/Users/Austin/Downloads/GripenFlight/docs/ATMOSPHERIC_COMBAT_PERFORMANCE_REVIEW_2026-10-23.md`

Only create or edit the report. Preserve all existing source, assets, application bundles, configuration, and unrelated working-tree changes. Inspect current repository instructions and status before working. The workspace already contains unrelated modified and untracked files; do not reset, clean, stage, or commit them.

## Starting evidence and interpretation

The project uses **Bevy 0.18.1**, with locally patched rendering code. Confirm this against the current manifest. Frame the engine decision as remaining with Bevy versus migrating to another engine.

Start with:

`/Users/Austin/Downloads/GripenFlight/docs/NEAR_TERRAIN_FPS_IMPLEMENTATION.md`

Cross-check it against current source and raw artifacts under:

`/Users/Austin/Downloads/GripenFlight/tests/optimization8/`

Earlier optimization reports contain superseded findings. Verify whether each proposed fix already exists before recommending it. The observations below are leads from an initial read-only review, not substitutes for checking the owning source.

- Near-terrain slowdown was reproduced with authoritative physics paused. Rendering can therefore cause the baseline slowdown independently of combat simulation.
- The latest recorded fullscreen external forest comparisons improved from roughly 35–36 FPS to 46–47 FPS, but p95 remained around 34.6 ms.
- The same pass did not establish a repeatable live-cockpit improvement.
- The saved cockpit Metal capture reports substantial transparent and opaque fragment activity, followed by environment filtering, shadows, and postprocessing.
- GPU-channel activity intervals overlap. **Never add them into an invented frame total or call them exclusive pass durations.**
- Historical fullscreen measurements used 2880 × 1800 actual pixels. Do not confuse this with the 1440 × 900 windowed configuration.
- Historical results have power, background-load, instrumentation, and sampling limitations. They are not new measurements of the current working tree or necessarily representative of active low-altitude combat.

## Performance review

Produce a prioritized findings table, followed by enough explanation for another engineer to understand and implement each proposed remedy later.

For every finding include:

1. Triggering conditions and affected views.
2. Exact current source locations with line references.
3. Evidence strength: measured historically, confirmed by source, or hypothesis.
4. Existing optimizations and the remaining problem.
5. A concrete proposed fix.
6. Expected benefit category, implementation effort, risks, and verification requirements.

Separate sustained frame cost, intermittent spikes, and perceived simulation lag. Rank cockpit and external-view priorities separately where appropriate. Do not invent numerical speedups.

### Rendering

- **Cockpit transparency:** examine canopy, combiner, HUD, contributing layers, and shader features. Consider material specialization while retaining reflections, lighting, fog, alpha, and blending. Check compiled behavior before claiming that repeated shader expressions cause repeated physical texture fetches.
- **Terrain and forest shading:** distinguish fragment costs from geometry and submission costs. Consider shaders specialized for the actual material features. Lower resident geometry counts alone do not prove lower critical-path frame time.
- **Forest geometry and shadows:** evaluate shared buffers or instancing that preserve tree shapes, positions, colors, orientation, bounds, and shadow participation. Static shadow reuse requires complete invalidation for sun, cascade matrices, render origin, terrain, and moving or damaged casters.
- **Atmospheric environment filtering:** inspect remaining kernel work and redundant invocations while retaining sample counts, update cadence, immutable captures, and complete publication. One lead is the mirror radiance mip using a generic multi-lane dispatch even though most lanes return without sampling; its actual benefit is unmeasured.
- **Postprocessing:** examine render-target traffic and unnecessary copies while preserving resolution, bloom, tone mapping, and antialiasing. Pass fusion is conditional on preserving intermediate values and neighborhood sampling.

### Terrain streaming and frame spikes

- Examine complete-layout visibility activation, mesh preparation, GPU allocation, retirement, and asynchronous worker backlog.
- Assess publication through tile roots, compatible buffer reuse, and a bounded tile cache. Hierarchical publication can reduce explicit commands but does not eliminate descendant propagation work.
- Preserve complete terrain coverage and correct first-visible-frame transforms.
- Distinguish startup stalls from persistent flight costs.
- Treat source-byte staging budgets as different from actual render-thread or GPU execution budgets.
- Check serial work inside asynchronous jobs; adding workers alone does not parallelize a sequential job.

### Combat and very-low-altitude CPU work

- Investigate cheap sensor eligibility checks before expensive terrain line-of-sight queries. Preserve detection decisions, observable diagnostic reasons, and event order.
- Examine smoke mesh rebuilding and the existing experimental GPU smoke path. Determine which calculations and buffers are already reused; do not describe all updates as fresh allocation.
- Review effect bounds, culling, per-particle material updates, and contention caused by broad asset-write access. Off-camera rendering may be culled while authoritative particle simulation continues.
- Examine tree collision queries, swept hull tests, suspension, and debris contact.
- Consider exact reuse and conservative broad-phase rejection without reducing collision samples, AI cadence, physics accuracy, or deterministic behavior. Render LOD geometry must not replace authoritative collision terrain.
- Treat cockpit rasterization and damage-display work as secondary until evidence supports a higher ranking.

### Simulation timing and perceived lag

Verify and explain that the current simulation advances a configured amount per rendered frame. At default settings, 30 rendered FPS advances approximately half a simulated second per wall-clock second.

Separate this behavior from the causes of slow frames. It is not an accumulating real-time catch-up spiral. Discuss real-time fixed-step scheduling and interpolated presentation only as an optional timing-contract change, including overload behavior, integration fidelity, and event-ordering implications. Do not propose silently increasing timesteps or dropping physics work.

### Existing optimizations to preserve

Verify and acknowledge current asynchronous terrain generation, bounded staging and retirement, forest partitioning, shared tile transforms, stale-work coalescing, atmosphere caching and sliced filtering, HUD cropping, opaque shader specialization, persistent display uploads, exact LOS caching, and large-formation physics parallelism.

Do not recommend these as missing features. Do not assume the experimental GPU smoke path or generic depth prepass improves performance. Check the saved results and current activation conditions.

### Source navigation leads

All paths below are relative to the workspace; resolve current line numbers when citing them:

- `src/terrain.rs`, `src/terrain_ready.rs`, `src/world.rs`: terrain layout, forest generation, staging, publication, retirement, and coordinate transforms.
- `src/main.rs`, `src/performance.rs`, `src/benchmark.rs`: scheduling, simulation timing, camera configuration, diagnostics, and saved benchmark semantics.
- `src/cockpit.rs`, `src/cockpit/coverage.rs`, `src/aircraft/visual.rs`, `src/display_upload.rs`: transparent materials and instrument updates.
- `vendor/bevy_pbr/src/atmosphere/`, `vendor/bevy_pbr/src/light_probe/generate.rs`, and associated renderer shaders: atmosphere generation, filtering, caching, and shader specialization.
- `src/combat/ai.rs`, `src/combat/bvr.rs`, `src/combat/guidance.rs`: sensor eligibility, terrain LOS, warnings, and aircraft integration.
- `src/combat.rs`, `src/combat/aim9x.rs`, `src/combat/smoke_gpu.rs`, `src/combat/effects.rs`, `src/aero_fx.rs`: trails and combat effects.
- `src/collision.rs`, `src/physics.rs`, `src/warp.rs`: terrain contact, double-precision flight state, and configured timestep.

## Engine comparison

Compare:

- Continued Bevy development.
- Unity with URP.
- Unity with HDRP.
- Unreal Engine.

Evaluate each against:

- Near-ground performance potential on the actual M1/8 GB machine.
- Terrain, vegetation, lighting, profiling, editor, and asset workflows.
- Migration effort for procedural aircraft, cockpit instruments, combat effects, terrain, and custom shaders.
- Preservation of double-precision physics and camera-relative rendering.
- macOS and Windows support.
- Maintenance burden, licensing, and platform feature restrictions.

Use current official documentation and link claims directly to supporting sources. Treat Unity URP and HDRP separately. Account explicitly for Unreal’s Mac memory requirements and feature restrictions. Distinguish editor requirements from runtime requirements where the documentation allows. Avoid treating licensing for games and non-game industry use as interchangeable.

Useful official documentation starting points, which must be checked for current applicability:

- Unity native plug-ins: https://docs.unity3d.com/6000.0/Documentation/Manual/plug-ins-native.html
- Unity HDRP compatibility: https://docs.unity3d.com/Packages/com.unity.render-pipelines.high-definition@17.0/manual/System-Requirements.html
- Unity pricing updates: https://unity.com/products/pricing-updates
- Unreal macOS requirements: https://dev.epicgames.com/documentation/unreal-engine/macos-development-requirements-for-unreal-engine

Assume preservation of the Rust simulation is preferable to replacing its behavior. Describe the work required to extract it from Bevy coupling and expose a coarse native interface for commands, simulation advancement, and state snapshots. Do not imply that this interface already exists. Keep double-precision state authoritative and convert to camera-relative presentation data at a deliberate boundary. Discuss packaging, data ownership, and crossing the native boundary in batches rather than per entity.

End with a clear recommendation. Distinguish potential tool/workflow benefits from unproven FPS benefits. Describe a limited future comparison prototype using matched terrain, cockpit, effects, resolution, and simulation workload before committing to migration. Do not build it or install engines.

## Validation requirements and completion criteria

No fresh performance tests are requested. Instead, specify how future implementations should be validated:

- Stationary forest views and moving terrain-cell crossings.
- Cockpit and external cameras.
- Low-altitude dogfights and larger formations.
- Simultaneous missile trails, explosions, and ground impacts.
- Camera transitions, weather changes, and rapid descents.
- Matched resolution, preset, VSync, power state, and background load.
- p50/p95/p99 frame times and long-frame counts alongside average FPS.
- Separate uninstrumented timing from profiling captures.
- Visual equivalence, terrain coverage, collision outcomes, and seeded simulation checks.

Keep orbital and lunar performance outside the investigation. Discuss planetary precision only where necessary for migration compatibility.

The work is complete when the report contains:

- A clear, evidence-qualified ranking of the largest likely bottlenecks.
- Concrete fixes and tradeoffs for each material finding.
- Separate priorities for cockpit and external views.
- An accurate inventory of relevant existing optimizations.
- An engine comparison and recommendation.
- Explicit uncertainties and future verification requirements.
- A concise chat summary linking to the finished report.

Before delivery, check that source references and artifact links resolve and that only the report was changed by this task. No application tests or builds are needed for writing this report.

Do not claim that 60 FPS is guaranteed or that switching engines inherently improves performance.
