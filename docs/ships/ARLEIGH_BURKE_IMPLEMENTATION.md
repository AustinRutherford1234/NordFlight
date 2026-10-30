# Arleigh Burke escort integration

## Delivered scene

Two representative Flight III destroyers appear whenever the existing carrier scenario is active. DDG 125 is 500 m port; DDG 128 is 500 m starboard. Both use the carrier's straight-course 25-knot reference frame. They share one prepared asset family with individual conforming hull numbers, stern names, flight-deck identities and small mooring-coil differences. Static weapons remain visual geometry.

The freely downloaded SB-129 model supplies the principal hull, superstructure, radar, gun, launcher, boat and helicopter-facility geometry. The preparation script removes the parked helicopter, aligns units and waterline, refines normals/materials, and adds attached aft handrails/footplates, tie-downs, lockers/handles, fasteners, eight static sailors, 54 bridge-window inserts, navigation lenses and local deck lamps. Hangar doors remain closed. The original operational-gray textures are preserved, with a subtle procedural paint normal map. Small fittings, lighting, waterline and identity arrangements are art estimates; no claim is made to reproduce every fitting of either ship on a particular date.

Four shared LODs contain 86,689 / 37,301 / 10,076 / 3,532 triangles before individual markings. Screen-size-adjusted distance thresholds are 300 / 1,000 / 3,000 m, with 10% hysteresis. Static sailors appear only at the closest LOD; hull markings disappear at the farthest. Seven image files are shared across all LODs and both ships. Night window/navigation emissions are updated only when needed; eight local spotlights have no shadows and are limited to nearby night views.

## Collision and reset behavior

The preparation script exports 82,855 nondegenerate triangles from the prepared ship skin before simplification. Paint overlays and static sailors are excluded. A single cached BVH is instanced at both escort offsets. Existing aircraft skin/gear and swept-sphere queries test all three ships and return the earliest contact, including moving-frame motion. Destroyer decks are obstacles, not carrier landing/catapult surfaces. Rendering and collision share the formation-offset function.

Tests cover both hulls, water gaps, earliest-hit ordering over a long sweep, full aircraft impacts in both ship frames, formation over time, LOD hysteresis, shared textures, and hiding escort geometry/markings/lights on scenario reset. Existing carrier and aircraft regressions remain part of the full suite.

## Reproduction and evidence

Asset source, license and modifications: `assets/ships/burke/README.md`. Regenerate assets with Blender 5.x and `scripts/burke/build_asset.py`. Source/collider hashes and measured geometry counts are in `assets/ships/burke/inspection.json`.

`tests/escort-model-review/capture.py` runs the six approved native views and pauses/restores existing game processes in a `finally` block. `benchmark.py` performs the approved paired fullscreen, normal-quality, 10-second-warm-up / 30-second-sample rendering checks. The reference hides escort rendering and asset loads; the comparison renders the same static fleet camera with escorts. These measurements isolate added rendering cost and do not validate live flight physics, combat, or every possible close-up view. Generated captures/logs/reports remain local under the ignored test directory.

## Validation results — 2026-10-29

- Full automated suite: **663 passed, zero failed** (`cargo test --locked`). The final diagnostic-camera framing adjustment changes only screenshot composition; the packaged app was rebuilt afterward.
- Six native views were captured and inspected: fleet overview, DDG 125 bow/identity, DDG 128 bow/identity, DDG 128 stern/clear helicopter deck, and both ships at night. The fleet overview was recaptured with symmetric wider framing after one ship initially fell behind the interface. Close views confirmed the corrected curved-hull number placement; the stern view confirmed closed hangars and a clear deck. Night captures confirmed warm bridge emissions and navigation lamps, although the unlit hull is very dark in the existing nighttime environment.
- Existing game processes were restored after both capture and benchmark harnesses. No additional soak or live-flight performance run was performed.
- `./build-mac.sh dev` rebuilt and updated the existing `Nord Flight.app`. Strict deep signature verification passed. Built and packaged ARM64 executables share UUID `1FA7CC29-13FC-30BD-9F7C-5A64D9F08082`.

Paired render measurement: this Mac, fullscreen **2880×1800**, normal quality, VSync, optimized development build; 10-second warm-up and 30-second sample per case, paused fleet camera, no concurrent builds.

| Case | Frames | Mean frame time | Mean FPS | p50 | p95 | p99 | Frames over 50 ms |
|---|---:|---:|---:|---:|---:|---:|---:|
| Carrier reference, escorts hidden/not loaded | 1,571 | 19.105 ms | 52.34 | 16.691 ms | 33.935 ms | 47.758 ms | 12 |
| Carrier and both escorts | 1,657 | 18.114 ms | 55.21 | 16.681 ms | 33.737 ms | 34.290 ms | 0 |

No added slowdown was resolved in this pair. The faster escort sample is **not evidence that escorts improve performance**: single samples include VSync, scheduling and run-to-run variation. These checks do not establish worst-case close-up/night rendering cost or collision/physics cost during live flight. The existing scene also did not sustain a locked 60 FPS in either sample.

Logs contain Bevy B0004 hierarchy warnings during glTF scene creation, including added detail nodes; the inspected captures show those components attached correctly. Scene-root visibility/reset behavior passes the ECS regression, but the warnings have not been demonstrated harmless for every renderer configuration. Existing source assets also report unused UV attributes during loading. Neither run reported a fatal loading/rendering error. These are retained as diagnostic limitations rather than silently treated as validated engine behavior.
