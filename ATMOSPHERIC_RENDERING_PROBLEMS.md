# Atmospheric rendering problems

## What the measurements establish

The preserved shipping BVR sample averaged **22.23 ms/frame**, with about
**34 ms p95**, missing the **16.67 ms** budget for 60 FPS at 1440 × 900.
Simulation CPU work averaged 1.03 ms/frame, cockpit processing 0.038 ms and asset
preparation 0.609 ms. These are incomplete, potentially overlapping spans, not
an additive accounting of the frame. A paused cockpit also remained near
43 FPS with essentially no simulation work.

Rendering and presentation are the strongest remaining bottleneck category.
The most expensive individual GPU pass has **not** been established. Previous
reports have neither valid per-pass GPU durations nor draw-call counts. They
cannot establish that atmosphere, shadows, transparency or synchronization
individually dominates. See `docs/OPTIMIZATION_IMPLEMENTATION_2026-10-20.md` and
`tests/optimization5/shipping/bvr.json` for the earlier evidence.

## Atmosphere and environment lighting

The original atmosphere basis consumes camera heading. Its inverse sampling
function also consumes the camera forward vector. Consequently, deleting
orientation from a cache key alone would return incorrect indirect lighting.
The probe needs a separate basis and matching inverse sampling before its
orientation dependency can be removed.

Both the ordinary atmosphere bind groups and the probe bind groups were
recreated during preparation even when their bound resources were unchanged.
Dynamic uniform offsets are dispatch inputs; they do not require rebuilding
the bindings while the underlying buffers, binding ranges and layouts remain
valid.

Static scattering lookup tables already have exact reuse, but their original
key included view-dependent lookup settings. Camera-space aerial-view settings
do not change the static transmittance or multiscattering integrals. The visible
sky and aerial-perspective lookups also dispatched in a completely unchanged
paused view. Reuse requires the complete camera, projection, medium, lighting,
settings, output textures and compiled pipelines; temporal jitter and custom
projection overrides need explicit treatment.

Six-stage environment filtering, immutable source snapshots, complete-buffer
publication and scene-generation cancellation already exist. This pass extends
those safeguards rather than replacing them. Their GPU cost remains to be
measured in the final validation phase.

### Reuse gate defect found during validation

The first candidate's paused-cockpit sample recorded **993 filter bypasses and
zero source writes during the sampling window**. The source was unchanged, but
filtering still ran. `prepare_generated_environment_map_intermediate_textures`
counted all `Camera3d` components to decide whether single-view reuse was safe.
The inactive main-menu camera retains that component. Bevy removes its
`ExtractedCamera`, however, so it submits no camera encoder.

The correction counts `Camera3d` entities with an `ExtractedCamera`. Genuine
multiple-view rendering retains the conservative fallback. This is a demonstrated
gate defect; its frame-time benefit still requires the final matched release
comparison. The initial candidate is not an accepted performance result.

## Shadows and transparency

The default preset retains two directional cascades and a 1024 shadow map; the
high preset uses 2048. Geometry can contribute to the main view and relevant
shadow views. Bevy already performs visibility and shadow-caster culling. A
main-camera-invisible object can still cast a visible shadow.

Depth attachments come from Bevy's texture pool and include view/cascade layer
assignment. Equal cascade matrices alone do not establish persistent ownership
of the depth contents. Skipping a clear/draw without verifying target ownership,
all caster dependencies, compiled pipelines and both early/late shadow passes
would risk stale or missing shadows. Exact cascade reuse is therefore conditional
on establishing that ownership, not a fixed-rate shadow update.

Many displays and effects already exclude shadow casting. Numeric instrument
backgrounds and the cockpit label atlas already use opaque materials. HUDs,
glass, radar/RWR alpha pixels, plumes and damage layers need meaningful
transparency. Blanket conversion or hiding the exterior in the cockpit would
change the image. No previous measurement establishes redundant layers or their
individual GPU cost.

## Submission and synchronization

Earlier mesh sharing reduced the paused-cockpit mesh-allocation gauge from
2,339 to 1,548 without reducing reserved GPU slab capacity. That does not prove
that draw calls decreased. Many rigid frame segments remain separate entities.
They are candidates for explicit batching; controls, clickable parts, damage
pieces and movable surfaces are not.

The previous display handoff held a shared mutex during pixel comparison and
packing. GPU uploads were already outside that mutex. Separating producer state
from complete render-consumer snapshots can reduce lock duration, but adds a
bounded snapshot copy whose benefit must be measured.

VSync, render-world handoff, driver submission and presentation waits remain.
Frame times near 17/34 ms are consistent with missed refreshes; they do not
identify which wait or GPU pass caused them. Final reporting must separate GPU
execution, CPU submission and presentation waits, leaving unsupported counters
unavailable rather than estimating them.

## Evidence boundary

This document identifies source-confirmed work and earlier measurements.
It is not a claim that the new changes improve FPS or that 60 FPS is achieved.
Acceptance results belong in `docs/ATMOSPHERIC_RENDERING_RESULTS.md` after the
focused end-of-pass validation. The implementation and acceptance procedure is
in `ATMOSPHERIC_RENDERING_FIX_PLAN.md`.
