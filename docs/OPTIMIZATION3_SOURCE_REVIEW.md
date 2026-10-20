# Optimization pass: source review and implementation boundaries

This review builds on `OPTIMIZATION_AUDIT.md` and the earlier 40-minute review.
It prioritizes measured costs instead of changing numerical models to improve a
benchmark. Final measurements and acceptance results belong in `OPTIMIZATION3.md`.

| Subsystem / sources | Review and action in this pass |
| --- | --- |
| Earth: `earth.rs`, `earth_region.rs`, `earth_gpu.rs`, `earth_vertex.wgsl` | Move the existing camera-dependent color formulas to the original mesh vertices on the GPU. Preserve textures, geometry, alpha, unlit materials, thresholds and interpolation. Retain a same-executable CPU fallback. Avoid unchanged surface/water material mutations. |
| Cockpit: `cockpit.rs`, `cockpit_controls.rs`, `cockpit_labels.rs`, `instruments.rs`, `navball.rs`, `display_upload.rs` | Retain display resolution, refresh/input behavior and label atlas. Reuse producer/staging buffers, detect exact dirty bounds, coalesce pending changes and issue compact texture writes outside the producer lock. Keep pending data while resized GPU images are unprepared. Retire removed images. |
| Presentation: `main.rs`, `world.rs`, `render_cache.rs`, `aircraft_detail.rs`, `spacecraft_detail.rs`, `control_surfaces.rs`, `gear.rs`, `skin_damage.rs` | Compare unchanged material/light/layout/transform values before mutable assignment. Retain shared model geometry, detailed gear, lighting and surface animations. Broaden scheduling only after simulation; Bevy still serializes conflicting borrows. Preserve input → simulation → camera → final UI ordering. |
| Flight: `physics.rs`, `aerodynamics.rs`, `airframe.rs`, `propulsion.rs`, `flight_study.rs`, `collision.rs` | Add a bounded conservative edge index for the lake polygon; run the original crossing arithmetic in original edge order. Preserve all force equations, double precision, integration, FBW, collision skin and successive-step ordering. Keep the existing prepared panel-flow and collision geometry optimizations. |
| Combat: `combat.rs`, `combat/ai.rs`, `combat/bvr.rs`, `combat/bvr/cockpit_radar.rs`, `combat/bvr/setup_screen.rs`, `combat/bvr/ui.rs`, `combat/contact.rs`, `combat/dogfight.rs`, `combat/guidance.rs`, `combat/damage.rs`, `combat/effects.rs`, `combat/hud.rs`, `combat/rwr.rs` | Reuse prior-aircraft-pose and decoy buffers. Keep persistent smoke buffers and their explicit conservative bounds; suppress duplicate automatic bound construction. Preserve weapon/damage RNG order, contact order, seeker/LOS logic and all physics/AI updates. Existing large-formation integration uses disjoint aircraft with immutable decisions and stable shared-event commits; no new approximate or lower-frequency AI path is introduced. |
| Terrain: `terrain.rs`, `lunar.rs` | Preserve bounded background jobs and complete old tile coverage until replacements are ready. Retain terrain detail, forest meshes, landable slopes and lunar contact. Avoid unchanged opacity writes. Measure generation separately from frame time. |
| Space: `space.rs`, `space_navigation.rs`, `orbit.rs`, `warp.rs`, `map.rs`, `maneuver_editor.rs` | Retain patched conics, double precision, orbital integration/propagation, prediction/cache invalidation and map geometry. Add timing only to orbital coast work. Do not parallelize dependent propagation steps or change warp safety/step limits. |
| Effects: `aero_fx.rs`, `combat/effects.rs`, `combat.rs` | Retain particle populations/lifetimes, flame geometry and simulation-time advancement. Decoy position/color/UV/index equivalence is tested across pool growth and shrinkage. Smoke bounds include rounding margin at distant camera-relative coordinates. |
| Audio: `sound.rs` | Verify the real-time callback retains nonblocking `try_lock`, bounded voices/events and existing buffers. Keep synthesis, sample rate, cannon continuity, engine mix, acoustic propagation and Doppler unchanged. Compare the complete audio diagnostics. |
| Measurement: `performance.rs`, `benchmark.rs`, `optimization_study.rs`, `tests/optimization3/*.py` | Opt-in spans and counters, exact diagnostic comparisons, native captures, scripted same-process stress transitions and recoverable benchmark isolation. Benchmarks use explicit timestep/visual settings rather than persisted user preferences. |
| Engine: `vendor/bevy_pbr/src/atmosphere/node.rs` | Cache only transmittance/multiscattering LUTs with exact model/settings bits, medium/output texture identities and prepared pipeline identities. Track both outputs independently to handle texture-cache reassignment. Bound stale records. Leave sky/aerial LUTs and environment convolution at their original resolution, sample counts and cadence. |

The Metal capture identifies environment-map radiance/irradiance convolution as
a large GPU workload. The additional `light_probe/generate.rs` cache reuses
prepared bind groups by complete GPU resource/layout/constant identities. A
source-write revision cache avoids recomputing convolution only for unchanged
atmosphere-owned source textures, identical bindings/pipelines and one 3D view.
It records actual bound-output texture writes, not newer pending asset handles.
Generic or multiple-view scenes retain original work. Dynamic atmosphere
updates still trigger the complete original convolution immediately.

Active convolution results depend on camera/lighting inputs.
Reducing sample counts, resolution or update frequency would be a different
optimization with a separate fidelity tradeoff, so this pass retains them.
More CPU threads cannot remove that GPU work. Any future engine comparison
must reproduce these lighting and geometry settings rather than compare
different scenes or quality presets.

`vendor/bevy_pbr/NORD_PATCH.md` records the narrow engine patch, its invalidation
contract and diagnostic disable switch. The original upstream licenses remain.
The unrelated Objective-C compatibility patch remains in `vendor/block`.
