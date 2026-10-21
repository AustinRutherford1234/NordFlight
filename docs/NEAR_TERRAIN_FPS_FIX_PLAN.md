# Plan to fix near-terrain frame cost

Date: October 21, 2026. Status: compatible implementation slices completed and
focused native validation recorded in
[NEAR_TERRAIN_FPS_IMPLEMENTATION.md](NEAR_TERRAIN_FPS_IMPLEMENTATION.md).
Conditional tree-instancing, static-shadow and backend rewrites remain follow-ups;
60 FPS and a reliable live-cockpit gain remain unmet.

This plan follows [the source analysis](NEAR_TERRAIN_FPS_ANALYSIS.md). No game
code was changed while originally writing it. It targets low-altitude aircraft flight over
land/forests and the additional cockpit cost, while retaining Bevy.

## Goal, boundaries and implementation order

Reduce sustained frame cost and streaming stalls at the user's actual large
framebuffer. Keep 60 FPS / 16.67 ms as a target, not an assumed result. Evaluate
1440 × 900 separately from native 2880 × 1800 fullscreen; never silently lower
resolution to claim success.

Preserve aircraft physics, configured timestep, integration/precision, AI and
guidance frequency, collision accuracy, deterministic damage and fuel behavior.
Preserve tree population, terrain/aircraft geometry, texture resolution, shadow
cascades and resolution, visible lighting, effects and cockpit display cadence.

**First implement the environment producer/consumer correction.** Then address
contributing cockpit fragment work. Treat geometry/shadow architecture and
streaming as distinct follow-up changes rather than mixing all four into one
unattributable optimization. Runtime diagnostics and tests belong at the end of
each concrete implementation slice; another broad baseline campaign is not the
first task.

| Priority | Work | Expected benefit | Main risk |
|---|---|---|---|
| P1 | Gate upstream indirect-source generation at actual filter capture boundaries | Eliminate intermediate GPU source writes that the sliced filter does not consume | Incorrect invalidation, another consumer, or stale resources |
| P2 | Reduce contributing/empty cockpit transparent-fragment work | Lower pixel cost in the historically expensive forward cockpit | Changed tint, reflections, HUD coverage or blend order |
| P3 | Improve shadow/forest submission without changing geometry | Less duplicated geometry/buffer work where GPU/submission evidence supports it | More draws, precision drift or incorrect shadow culling |
| P4 | Coalesce stale streaming work and bound remaining publication costs | Fewer movement/reset spikes and unnecessary uploads | Coverage holes, delayed completion or starvation |
| P5 | Remove remaining CPU presentation churn and real synchronization stalls | Better pacing if those costs are on the critical path | Broken ordering, extra latency or chasing noncritical work |

No numerical FPS forecast is justified yet. The identified six-to-one source
write/capture mismatch is not a sixfold whole-game speedup opportunity.

## P1 — Make indirect-source production follow its actual consumer

### Implementation

Work primarily in the vendored atmosphere environment node and environment
generator:

- [`environment.rs:386`](../vendor/bevy_pbr/src/atmosphere/environment.rs#L386)
- [`generate.rs:774`](../vendor/bevy_pbr/src/light_probe/generate.rs#L774)
- [`generate.rs:1448`](../vendor/bevy_pbr/src/light_probe/generate.rs#L1448)
- [`atmospheric_render.rs:34`](../src/atmospheric_render.rs#L34)

1. Trace every reader of the canonical source texture. Establish that the
   gated, owned source is used only by the immutable filtering chain. If another
   consumer requires current pixels every frame, keep that consumer current or
   separate its resource; do not silently make its input stale.
2. Expose a small render-world capture decision from the generator instead of
   exposing its private mutex/state layout. Decisions distinguish initialization,
   next-cycle capture, active-cycle reuse, invalidation and unsupported ownership.
3. During an initialized six-stage cycle, skip the **dedicated canonical probe
   sky LUT and six-face environment-source dispatch** on frames where the
   immutable source is still being filtered. Continue filtering the scheduled
   face. Keep the visible camera sky and aerial-perspective LUT path current.
4. At the next cycle boundary, capture the latest exact inputs, as the original
   path would on that boundary. Retain the same six-face source contents,
   downsampling, filter sample counts, formats, sizes and mip coverage. Do not
   replace this with a movement threshold or a reduced lighting update rate.
5. Keep latest requested inputs separate from the signature actually written
   to the source texture. Skipping a write must not mark new pixels as present.
   Increment `TrackedEnvironmentWrites` only after an actual encoded write;
   filter keys continue to identify the revision actually copied.
6. Preserve immediate complete initialization on discontinuities. Camera/scenario
   cuts, atmosphere replacement/removal, light/medium/resource changes, shader
   or pipeline replacement, graphics changes and output rebinding must override
   cycle reuse. Validate the texture actually bound, not just its requested handle.
7. Retain conservative full-update behavior for multiple active views, unowned
   textures and unsupported layouts. Never share one view's capture decision
   across unrelated probe owners.
8. Keep complete front/back publication and old valid lighting during pipeline
   preparation. Missing readiness must not expose incomplete maps or a blank frame.

The expected steady moving-scene pattern is approximately one upstream capture
per six-face cycle rather than six upstream writes per cycle. Initialization,
invalidation and full-update reference paths are legitimate exceptions.

### Why this should preserve appearance

The existing filter already selects one immutable source and ignores later
versions until it starts another cycle. The change removes intermediate versions
that this consumer does not read; it does not substitute an older snapshot at a
capture boundary. This argument depends on the all-consumer and invalidation
checks above and must be verified against captured source revisions.

### Focused correctness cases for final validation

- Steady changing inputs: consumed snapshot revisions and completed outputs
  match the reference while intermediate source writes disappear.
- Every face position in a six-stage cycle, including the final face and the
  next boundary.
- Initial full update; one-/three-/six-stage developer paths; unchanged inputs.
- Scene generation changes mid-cycle, including weather, camera/map and preset.
- Replaced textures, density LUTs, bind groups and shader/pipeline identities.
- Not-ready pipelines, atmosphere removal/re-addition, disabled maps, multiple
  active views and unowned sources.
- Bounded resources and no delayed-input or main-sky change.

Implement and validate this small change before assuming a larger renderer
rewrite is necessary.

## P2 — Reduce cockpit pixel work while retaining its actual lighting

### Assign explicit render roles

Classify canopy glazing, interior combiner, HUD texture, lit display, unlit
navball, solid bezel, effect and damage overlay. Record parent, sides, material,
shadow flags, visibility rules and why each contributes from cockpit/exterior.
Use this inventory to identify actual duplicate submissions. Existing opaque
display backgrounds, unlit navball, hidden external HUDs and shadow exclusions
should remain; they are not unimplemented optimizations.

### Implement exact coverage reduction first

- For textured `AlphaMode::Blend` overlays, compute conservative nonzero-alpha
  coverage from the CPU image already being rasterized. Bound the drawn quad or
  use a small set of persistent strips where it meaningfully removes empty area.
- Preserve original texture dimensions, UV-to-plane mapping and refresh cadence.
  Include the bilinear/filter/MSAA support footprint in the bounds; do not clip
  faint glyphs, antialiased edges or bloom contributions.
- Hide genuinely empty overlays and restore them immediately when content or
  scene changes. Use reusable geometry or per-instance rectangle parameters;
  avoid rebuilding procedural meshes and bind groups every HUD update.
- Keep existing zero-alpha rejection. Consider moving necessary alpha rejection
  earlier only after decals and all alpha inputs are resolved and only for
  blend modes where zero alpha guarantees zero contribution. Premultiplied and
  additive emission are not interchangeable with ordinary alpha blending.
- When two material reads use the identical texture, sampler, UVs and sampling
  semantics, inspect whether the compiled shader already shares the result.
  Reuse one sample explicitly only when it preserves the material calculation.

The broad canopy/combiner has nonzero alpha and meaningful reflections; sparse
HUD coverage optimizations do not automatically apply to it. Remove a second
layer or a back face only after proving it contributes no color, reflection,
depth or shadow in every supported view. Do not turn lit HUD/glass materials
unlit merely to save shader work: that would change their current response.

If the contributing glass remains expensive, specialize its shader to the
features it actually uses while retaining the current BRDF, IBL, direct lights,
fog, alpha and color-space behavior. Check existing shader specialization before
duplicating it. Avoid replacing full lighting with a guessed constant tint.

Apple's architecture guidance recommends minimizing avoidable dependencies,
attachment traffic and noncontributing rendering work. That supports this
targeted approach, not an automatic benefit from adding another pass.
[Apple: Optimize Metal Performance for Apple silicon Macs](https://developer.apple.com/videos/play/wwdc2020/10632/).

## P3 — Improve forest and shadow submission where it matters

### Small, compatible changes

1. Measure actual submissions separately from the existing visible-vertex
   metric during final validation. Do not choose smaller batches solely because
   that metric falls.
2. Prototype one render-origin transform per terrain attachment group, with
   forest pieces inheriting it. Preserve original world-space anchors, global
   transforms, bounds, first-frame placement and retirement behavior. Avoid
   duplicating equivalent world transforms on every forest group.
3. Preserve separate spatial bounds for camera and each shadow view. Bevy
   already culls; fix participation/bounds errors instead of running a second
   production visibility survey. Keep the survey diagnostic-only.
4. Evaluate grouping under both forward cockpit and external views, with both
   presets. Reject a configuration that saves vertices but costs more frame time.

### Larger follow-up only if submission/geometry is still critical

Prototype instanced trees or persistent spatial tree buffers using the exact
authored 31-triangle tree shape, original face colors and deterministic tree
placement. Keep CPU collision trees unchanged. The conversion must preserve
planet-radial orientation, trunk/canopy scale and camera-relative precision;
GPU reconstruction of formerly double-precision CPU-baked vertices is not
automatically bit-identical. Require native visual/precision comparison before
shipping it. Share geometry/buffers without enlarging culling bounds to an
entire forest or changing tree population.

Static shadow reuse is a separate, more difficult option. First demonstrate
stable light/cascade matrices and ownership plus complete caster invalidation.
Camera rebasing, moving aircraft, animated surfaces, gear, damage, alpha inputs,
terrain replacement and sunlight all matter. Any changed dependency requires
the affected depth to be rendered again. A world-static forest is not proof
that its camera-relative shadow inputs are unchanged. Do not begin with fixed-rate
shadow updates, lower-resolution maps or reduced-detail shadow proxies.

The existing generic depth-prepass experiment did not help. Do not re-enable it
without new critical-path evidence. If a targeted occlusion solution is later
justified, account for custom Earth materials currently opting out of prepass,
their exact vertex transforms, additional geometry cost and Apple GPU behavior.

## P4 — Fix remaining streaming spikes without reducing coverage

Work in `terrain.rs`, `terrain_ready.rs` and the mesh-preparation boundary.

- Coalesce obsolete pending cell requests. Reuse completed tiles still needed
  by the current selection and cancel/discard obsolete work before unnecessary
  GPU staging. Cancellation should be checked at bounded tile boundaries.
- Do not demand a perfectly current cell every time a worker finishes: continuous
  motion must not starve publication. Define a complete valid coverage set,
  retain old coverage during preparation, and progress toward the latest request.
- Prioritize current near-view coverage, with stable tie-breaking. Preserve the
  chosen LOD/geometry and collision surface; this is work ordering, not a hidden
  reduction in terrain detail.
- Keep CPU mesh generation, insertion, GPU readiness and publication distinct.
  A byte budget bounds source work, not shader time, interleaved GPU bytes,
  allocator cost or whole-layout visibility/extraction work.
- Stage replacement render metadata before the final swap where possible;
  keep the visible coverage change coherent. Avoid spreading a visibility swap
  in a way that shows parent/child tiles simultaneously or leaves holes.
- Reuse scratch allocations and investigate compatible fixed-grid/index or
  mesh-buffer reuse. Bevy's ordinary `Mesh` handles do not automatically provide
  a shared per-tile index-buffer scheme; a custom path must justify its complexity.
- Keep active/pending/retiring resources bounded and retain the corrected
  cleanup-budget guard. Budget disposal of large cancelled CPU results too.

Only parallelize generation if final evidence shows worker completion limiting
streaming. Use a bounded shared pool, common immutable inputs and stable result
commit order; adding workers cannot make a fragment-bound frame faster and may
compete with physics/render submission. Initial menu/startup terrain generation
is a separate loading-latency task, not the explanation for a stationary slow view.

## P5 — Presentation churn and synchronization

Audit changed component/resource assignments in world, aircraft, cockpit and
effects presentation. Extend existing `set_if_neq`/material guards where useful.
Respect the actual stage dependencies: input → simulation → world/origin →
camera/sky state → presentation → extraction. Remove a scheduling barrier only
when its dependency is proven unnecessary; these systems also share broad asset
resources, so deleting `.chain()` is not proof they will execute concurrently.

Keep persistent display textures and the short handoff queue. GPU queue writes
already occur outside that lock. Do not introduce GPU readback, busy waits or
synchronous pipeline-compilation loops to make another subsystem appear faster.

Use a final GPU/CPU/presentation timeline to distinguish GPU execution, CPU
submission, drawable/queue waits and VSync pacing. Pass dependencies and copies
must be justified individually; a dependency is not a bug merely because it
serializes work. Do not rewrite wgpu/Metal or merge core render passes based
only on a 40 FPS plateau. If a dependency needs a core-pipeline patch, vendor the
pinned crate explicitly rather than editing the global Cargo registry.

Shipping VSync and frame latency stay unchanged unless a controlled final test
demonstrates better pacing without tearing or additional input latency.

## Final validation and acceptance — bounded, after implementation

### Correctness and visual equivalence

Run focused tests for the new capture policy, complete publication, resource
replacement/cancellation and changed attachment/bounds logic; then run the
application regressions once. Compare seeded flight output once, excluding
only existing timing fields. No physics behavior should change.

Capture matched cockpit/external forest poses and short motion sequences:
low flight, terrain-cell crossings, landing approach, camera/map returns,
moving sun/weather, both presets, glass/HUD and a damaged aircraft. Check every
contributing layer and frame around publication, not just average image error.
Use the existing transition harness for capture-policy cuts and pipeline
readiness; retain bounded representative images/contact sheets and state records.
Verify pause, reset and timestep changes. Do not shorten effects or lower quality
to pass comparison.

### Performance acceptance

1. Preserve the installed signed baseline, source/asset identities and final
   candidate identity. Use the same release settings; no compiler runs during
   timing samples.
2. Use three focused workloads: paused moving forest external, paused moving
   forest cockpit and live low-altitude flight with actual contacts/AI as
   appropriate. Keep path, seed, view, preset and framebuffer dimensions matched.
3. Record battery power and host conditions. The user explicitly requires
   remaining on battery; do not request AC power. Start with 10 seconds warmup and
   25 seconds sampling per version/workload. Repeat in reverse order only when
   differences are inconclusive or a percentile/throughput regression exceeds 5%.
   Keep windowed and native-fullscreen results separate.
4. Disable pass/forest instrumentation for accepted FPS runs with the existing
   `NORD_BENCH_FPS_ONLY=1`. Use a separate short instrumented run for counters.
   Report average, p50/p95/p99, >25/>50 ms frames and simulation throughput.
5. Check the specific P1 claim: source writes should approach consumed captures
   during steady six-stage updates, while captured revisions, output maps and
   cycle age stay correct. Report exceptions for initialization/invalidation.
6. Take one current, focused native Metal capture of the remaining slow view.
   Record actual GPU passes, CPU submission and presentation waits separately.
   Unsupported counters stay unavailable. Do not add overlapping GPU-channel
   intervals or CPU spans into an invented frame total. Apple's
   [GPU performance tools](https://developer.apple.com/documentation/xcode/optimizing-gpu-performance)
   provide the appropriate profiling workflow.
7. Run a two-minute moving-terrain/cockpit stability check for resource growth,
   coverage and switching failures. Do not repeat a broad 30-minute soak/matrix.

Keep only changes with demonstrated work reduction and no visible regression;
separately state whether they actually shorten the critical frame path. Revert
or keep developer-only a change whose repeated runtime costs outweigh its gains.
The last pass's flat external FPS is a reason to demand that distinction.

Build/package/sign the accepted native app and verify launch. Keep the existing
12 GiB Cargo-cache policy; retain only bounded baseline/candidate artifacts and
captures. Deliver before/after results, tests, visual checks, resource behavior
and the ranked remaining costs. If 60 FPS is still unmet, say so without
declaring an unsupported Bevy or hardware limit.

## Recommended first implementation slice

Implement **P1 alone** with its invalidation tests and a developer reference
switch. Its success criteria are unchanged consumed snapshots/publications and
elimination of unused upstream writes. Then use the bounded final comparison to
establish the frame-time effect. This gives a concrete, source-supported fix
before another large cockpit/shadow rewrite.
