# Why FPS drops near terrain

Date: October 21, 2026. Scope: the current Nord Flight checkout, especially
Gripen/F-22 flight over land and forests in a large window or fullscreen.

This is a source review and a reanalysis of existing artifacts. No game code,
build settings or assets were changed, and no new runtime diagnostics were run.
The companion implementation proposal is [NEAR_TERRAIN_FPS_FIX_PLAN.md](NEAR_TERRAIN_FPS_FIX_PLAN.md).

## Conclusion

The strongest supported explanation is **rendering and presentation pressure**:
low-altitude flight exposes detailed terrain and forests across a large part of
the image, renders applicable geometry into directional-shadow cascades, and
combines that scene with atmosphere lighting, cockpit transparency and
full-resolution postprocessing. Streaming introduces an additional, separate
source of occasional stalls.

This is not evidence that the flight model is too expensive or that Bevy cannot
reach 60 FPS. The slowdown persists in a moving-camera workload with simulation
paused. Recent forest culling improvements removed much submitted geometry
without materially improving external-view FPS, so triangle count alone is not
an adequate explanation.

**A newly identified scheduling inefficiency is actionable:** the environment
source can be regenerated every frame while the six-stage filter consumes only
one immutable source snapshot per cycle. The latest cockpit artifact records
864 source writes versus 144 captures. Avoiding writes that the current filter
will never consume is a better first target than another arbitrary reduction in
forest batch size. Its precise frame-time cost remains unmeasured.

## 1. Evidence and its limits

| Evidence | What it establishes | What it does not establish |
|---|---|---|
| Recent fullscreen terrain runs: approximately 40 FPS external and 33–35 FPS cockpit at 2880 × 1800 | The large near-terrain workload misses the 16.67 ms / 60 FPS target | A particular GPU pass being the sole bottleneck |
| Moving terrain benchmark pauses simulation | Expensive flight physics, missile guidance and collision advancement are unnecessary for this rendering slowdown to occur | Live combat and landing have no additional CPU cost |
| External forest batches: approximately 66% fewer camera-visible vertices and 28% fewer shadow-visible vertices, essentially unchanged FPS | Current culling waste is real, but reducing it did not remove the critical path | The remaining geometry and shadows are free |
| Separate powered cockpit pair: 34.50 → 33.08 FPS after the previous changes | The earlier battery FPS gain did not reproduce on AC | A reliable average-FPS benefit from that pass |
| Older forward-cockpit Metal capture | Transparent fragment work, opaque fragment work, shadow vertices and environment filtering were substantial in that capture | Current per-pass durations after subsequent optimizations |

Recent raw evidence is in [tests/optimization7](../tests/optimization7), including
`accepted-summary.json`, `matrix/`, `repeat/`, `ac/`, `prepass/` and the accepted
source identity. Project Rust source matches that accepted source snapshot.
The earlier implementation account is [NEAR_TERRAIN_OPTIMIZATION.md](NEAR_TERRAIN_OPTIMIZATION.md).

The older [forward-cockpit aggregate](../tests/terrain-performance/metal-forward-cockpit/aggregate.json)
contains these median active intervals per observed frame:

| GPU channel/event | Historical active interval |
|---|---:|
| Main transparent pass, fragment | 14.125 ms |
| Main opaque pass, fragment | 7.338 ms |
| Environment radiance filtering, compute | 4.177 ms |
| Outer directional-shadow cascade, vertex | 3.907 ms |
| Environment irradiance filtering, compute | 2.227 ms |

These intervals can overlap across GPU channels; **do not add them into a frame
budget**. They are historical evidence for prioritization, not measurements of
the installed candidate. Some of these paths have already been optimized.

The latest measured CPU means in the paused cockpit sample are approximately
0.0063 ms terrain update, 0.555 ms world presentation and 0.501 ms asset
preparation wall span. Simulation is effectively idle. These are partial and
potentially overlapping spans, not a complete accounting of a roughly 29 ms
frame. Unsupported GPU durations are unavailable, not zero.

The recent runs also enabled instrumentation. In particular,
[`performance.rs:255`](../src/performance.rs#L255) performs an additional forest
bounds/visibility survey. Both builds were instrumented, but changing entity
count changes that survey's work. Uninstrumented FPS comparisons and separate
instrumented captures should be used for final acceptance.

## 2. The direct low-altitude trigger: more detailed world coverage

[`terrain.rs:128`](../src/terrain.rs#L128) recursively subdivides terrain based
on horizontal distance and AGL. The minimum tile size is 512 m, within 120 root
tiles spanning the regional world. Every selected ground tile uses the same
64 × 64 surface grid, irrespective of its geographic width.

[`world.rs:513`](../src/world.rs#L513) therefore generates 8,192 surface triangles
and 512 skirt triangles per tile: **8,704 triangles**, with 4,481 vertices.
Lower altitude permits more nearby subdivisions and consequently more patches
at this fixed per-patch geometry cost. Frustum and light-space visibility still
determine which patches are actually submitted; the whole regional mesh is not
necessarily drawn every frame.

[`terrain.rs:231`](../src/terrain.rs#L231) adds forests only to tiles at most
2,048 m wide. As the layout refines, more nearby tiles qualify. Candidate trees
come from a deterministic 32 m grid with density/land filters. Each accepted
tree has 21 canopy triangles and 10 trunk triangles, stored as 93 vertices.
At ground level, nearby trees also occupy substantially more screen space.

The ground and forest use lit `StandardMaterial`, with roughness 0.97/0.95 and
normal shadow participation ([`terrain.rs:463`](../src/terrain.rs#L463)). Forest
faces are two-sided. That is actual opaque material/shadow work, not simply a
texture pasted over a distant planet.

The terrain height's trigonometric functions are evaluated during geometry
generation and physical queries. They are **not** being evaluated for every
terrain fragment: the rendered terrain uses baked vertex attributes and a small
repeating texture. Replacing height math is not an established cure for the
observed paused-render FPS loss.

## 3. Newly confirmed waste: upstream environment work outruns its consumer

The relevant chain is:

```text
camera / local sun / atmosphere inputs change
  → EnvironmentMapScene.source changes
  → canonical probe sky LUT and all six source cubemap faces are generated
  → DownsamplingNode captures one immutable source at the cycle boundary
  → FilteringNode works through six faces over six frames
  → complete diffuse and specular maps are published together
```

Source locations:

- [`atmospheric_render.rs:34`](../src/atmospheric_render.rs#L34): exact transform,
  local sunlight, density, weather and graphics input versions.
- [`environment.rs:386`](../vendor/bevy_pbr/src/atmosphere/environment.rs#L386):
  the environment node keys generation/source/resources and skips identical
  inputs. On a miss it generates the canonical probe LUT and dispatches six
  source faces. It does not consult the filter's pending-face progress.
- [`generate.rs:1448`](../vendor/bevy_pbr/src/light_probe/generate.rs#L1448):
  downsampling explicitly skips new captures while `next_face != 0`, retaining
  the immutable source for the entire cycle.
- [`generate.rs:788`](../vendor/bevy_pbr/src/light_probe/generate.rs#L788):
  one-face stages after full initialization; the default is six stages.

In `matrix/cockpit-1-candidate.json`, 865 sampled frames recorded **864 source
writes, 144 captures, 144 publications and zero cancellations**, with a maximum
six-frame cycle age. The external sample recorded **1,000 writes versus 166
captures**. The long streaming sample recorded **3,912 versus 652**.

Thus the producer/consumer mismatch is supported by both current source and
recent counters. For the sliced consumer, many intermediate source versions
are overwritten without being captured. Before gating them, implementation
must verify all other consumers, ownership, resource changes and scene cuts.
This is a scheduling opportunity, not permission to leave stale maps bound.

This work is not uniquely caused by low altitude: movement and lighting changes
can trigger it elsewhere too. Near terrain it competes with the heavier
foreground/shadow workload. Saving approximately five out of six upstream
captures is a potential work-count reduction, **not an 83% reduction in total
GPU time or a sixfold FPS forecast**.

Static scattering caches, canonical probe orientation, binding reuse,
six-stage filtering, immutable snapshots and complete map publication already
exist. Reimplementing those features would miss this remaining upstream gap.

## 4. Cockpit and full-resolution pixel cost

The larger measured framebuffer contains 5,184,000 pixels, versus 1,296,000 at
1440 × 900: **four times as many pixels**. This does not imply four times the
whole frame cost, but it raises fragment, blending, HDR and postprocessing work.
The camera includes atmosphere, bloom, tonemapping and FXAA
([`main.rs:1307`](../src/main.rs#L1307)). Low flight adds a detailed foreground
across much of those pixels; a cockpit adds overlapping layers to that scene.

Examples of lit blended surfaces include the cockpit combiner
([`cockpit.rs:139`](../src/cockpit.rs#L139)), textured HUD
([`cockpit.rs:565`](../src/cockpit.rs#L565)), the Gripen canopy in `world.rs`, and
aircraft glazing ([`visual.rs:1258`](../src/aircraft/visual.rs#L1258)). Some are
two-sided. The navball texture, in contrast, is already unlit. Instrument
backgrounds are already opaque; converting every display to opaque is not a
new opportunity.

The historical forward-cockpit trace strongly implicates transparent fragment
work in that pose. It does not prove that every listed surface is redundant or
that the same material is today's largest cost. Real canopy tint, reflections,
HUD pixels and smoke must remain visible. Some apparent duplication is genuine
layering, not a removable error.

Zero-alpha blend rejection and zero-contribution local-light skips already
exist in the vendored PBR shaders. They avoid some shading but do not eliminate
all texture/input preparation, rasterization or contributing translucent
pixels. Cockpit texture uploads also already use persistent GPU images, dirty
regions, complete snapshots and queue operations outside the handoff lock.
Small CPU instrument spans cannot explain the entire cockpit slowdown.

Apple documents separate vertex/fragment/compute channels and tile-based
hidden-surface removal. Opaque and blended work therefore need different
optimization strategies; adding geometry or passes is not automatically a
win. See [Apple's Metal optimization guidance](https://developer.apple.com/videos/play/wwdc2020/10632/).

## 5. Shadows and submission remain real costs

[`main.rs:1356`](../src/main.rs#L1356) configures two directional cascades, with
a 180 m first bound and 2,500 m maximum distance. Shadow-map dimensions are
1,024 by default and 2,048 in the higher preset. Nearby forest/terrain can
contribute to the main view and applicable shadow views, each with its own
culling. The historical outer-cascade vertex interval was significant.

[`light.rs:2340`](../vendor/bevy_pbr/src/render/light.rs#L2340) builds and renders
shadow passes; no accepted exact cascade-depth reuse eliminates their recurring
work. However, an offscreen tree can cast an onscreen shadow. Main-view culling
cannot replace shadow-caster culling, and changing shadow resolution or proxies
would change quality.

The current 512 m forest groups tighten bounds without changing geometry, but
also create more entities and submissions. Existing metrics count vertices in
visible batches, **not actual draw calls**. This explains why a large reduction
in that metric can coexist with flat FPS: fragment work, lighting, waits and
submission overhead may still determine the critical path.

World-relative presentation updates each `WorldObject`'s render transform when
the origin/basis changes ([`world.rs:796`](../src/world.rs#L796)). Forest pieces
sharing one tile anchor still have separate world-object transforms. More
groups can add transform propagation, extraction, bounds and queue work even
when the physical forest is static. Rigid aircraft batching and identical-mesh
sharing already run at startup; neither proves that terrain draws are batched.

## 6. Streaming explains some stalls, not all sustained low FPS

Existing replacement streaming is asynchronous and now bounded by source bytes
and mesh count. Old coverage stays visible until every new GPU mesh is ready.
Retired resources are hidden at publication and reclaimed incrementally.

Remaining costs in [`terrain.rs:495`](../src/terrain.rs#L495):

1. One task serially builds the missing tile vector; the result arrives only
   when the entire task completes. The configured async pool has one worker.
2. New meshes still need packing, allocation, extraction and GPU preparation.
   A 2 MiB source-data budget is not a hard GPU-time or transfer-byte limit.
3. Publication still updates visibility/transforms for the whole completed
   layout in one frame. That operation is not time-budgeted.
4. A task can finish for an older cell after the aircraft has moved; the old
   result is published before the next required layout is requested. The recent
   cleanup guard prevents a second same-frame cleanup budget, but does not
   coalesce stale generation work.
5. Initial terrain construction remains synchronous at startup.

These are credible sources of spikes while moving or changing scenarios. They
do not, by themselves, explain a persistently slow stationary cockpit view.
The readiness mutex is a short poll, not a blocking wait for the worker or GPU;
there is no evidence that it alone accounts for tens of milliseconds.

## 7. Physics, AI, weapons and other systems

Near the surface, CPU work can genuinely increase:

- Tree obstacle collection activates within roughly 30 m of terrain
  ([`terrain.rs:77`](../src/terrain.rs#L77)).
- Shaped-airframe terrain checks activate below roughly 12 m and perform swept
  pose/vertex tests ([`collision.rs:249`](../src/collision.rs#L249)). Wheel
  contact/suspension adds terrain normals and contact forces.
- AI terrain avoidance, radar/missile line-of-sight samples and fragment ground
  contacts add terrain queries during active combat. The LOS cache uses exact
  endpoint bits and is bounded; it is not guaranteed to hit for moving objects.

These costs deserve live-flight checks, particularly landing and large BVR
scenes. They cannot be the primary cause of the paused render-only result:
[`main.rs:1975`](../src/main.rs#L1975) exits simulation immediately on pause.
The low-warp simulation also advances a configured amount per rendered update;
it does not accumulate real-wall-time catch-up steps when FPS falls. There is
no demonstrated classic catch-up spiral here.

Ground-effect aerodynamics is a small coefficient calculation, not a separate
expensive ground-flow solver. Reducing AI frequency, collision fidelity,
double precision or physics steps would compromise simulation without addressing
the confirmed paused-render problem.

Audio has bounded voices/nonblocking callback handling; low-altitude warnings
do not introduce a demonstrated terrain-dependent mixing workload. Map and
maneuver drawing are view-gated. Lunar surface generation is not the aircraft
terrain path. Effects are pooled or cached; dense air can increase visible
transonic/vapor effects during live flight, but those effects are unnecessary
for the paused forest slowdown. Hidden fleet geometry consumes resources and
some CPU bookkeeping; its presence does not mean every hidden aircraft is drawn.

## 8. What previous experiments rule out—and what they do not

- **Smaller forest batches:** confirmed culling improvement; no consistent FPS
  win. Do not keep subdividing blindly.
- **Generic depth prepass:** the recent same-executable trial was flat externally
  and slightly slower in cockpit, so it remains off. Additionally,
  [`EarthExtension::enable_prepass`](../src/earth_gpu.rs#L37) returns false. That
  trial was not a complete prepass of every custom Earth material.
- **CPU-only physics optimization:** cannot solve a problem reproduced paused.
- **More cores:** can help suitable CPU tasks, but does not directly eliminate
  fragment work, blending or GPU dependencies.
- **40 FPS plateaus / 17–34 ms intervals:** compatible with presentation effects
  or missed refreshes, not proof of a fixed engine cap. Shipping VSync is still
  enabled. GPU execution, CPU submission and presentation waits remain separate
  quantities to establish.

## 9. Repository-wide review coverage

The review inventoried all **65 project Rust files and two project WGSL files**,
searched project/vendored rendering code for terrain-dependent work, and traced
the relevant runtime paths. It is not a line-by-line certification of every
third-party library. Runtime source/config fingerprints cover 201 files and
were checked for no changes during this documentation pass.

| Module family | Review outcome |
|---|---|
| `main`, `world`, `terrain`, `terrain_ready`, `atmospheric_render`, `earth*` | Direct terrain/render scheduling and publication findings above |
| Vendored atmosphere, environment generator, PBR, shadows | Upstream producer mismatch, full material work, transparency and repeated shadow paths |
| `aircraft*`, cockpit/interior/labels/controls, instruments, navball, gear | Submission, visibility, material and upload paths; existing batching/caches retained |
| `render_batch`, `mesh_reuse`, `render_cache`, `display_upload`, `performance`, `benchmark` | Existing optimization mechanisms and measurement limitations verified |
| `physics`, aerodynamics, control surfaces, propulsion, collision, airframe, warp | Surface-dependent CPU work distinguished from paused-render behavior |
| Combat, AI/BVR/dogfight, guidance/sensors, contact/damage/effects/smoke, HUD/RWR | Active-scene CPU/effect costs; no evidence they cause the idle forest result |
| Sound, aero effects, skin damage, aircraft detail | Conditional live effects and bounded/cached work; not established primary causes |
| Space, orbital/celestial/lunar/navigation/map/maneuver systems | Aircraft-mode/view guards and separate terrain path; no proposed spacecraft rewrite |
| Hangar/start screen, studies, build/package configuration | Inactive-view/resource and benchmark controls; compiler choice is not a rendered-frame fix |

The best next step is the specific upstream environment scheduling correction,
then contributing cockpit pixel work and measured shadow/submission costs—not
another general simulation optimization or an unsupported engine migration.
