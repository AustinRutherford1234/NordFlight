# Terrain and configurable environments

## Running and configuring

The normal launch selects real Earth and the Mac Balanced world-render target.
The flight setup and Escape pause menu include **EARTH** and **SCI-FI EARTH**
buttons. Switching worlds restarts the app into fresh flight setup and ends the
current flight. The active world is highlighted; choosing it again does nothing.
The interface remains at the window's native resolution. Settings → Presentation
cycles world resolution through 50%, 67%, 75% and 100%; F9 retains the existing
Standard/High switch. Changes in world resolution are saved to
`~/.nord-flight/graphics.json` (on Windows, under `%APPDATA%/.nord-flight`).
Diagnostic launches do not save graphics preferences.

```sh
# From the repository, or use the executable inside the Mac app bundle.
./target/release/gripen-flight --environment earth --graphics balanced --render-scale 0.5
./target/release/gripen-flight --environment scifi-earth --graphics high
./target/release/gripen-flight --environment legacy --render-scale 1
```

`--environment` also accepts a preset JSON path. `NORD_ENVIRONMENT`,
`NORD_GRAPHICS`, `NORD_RENDER_SCALE` and `NORD_GRAPHICS_CONFIG` provide equivalent
configuration hooks. Planet/preset and installed-pack changes require a restart.
Graphics quality never changes the authoritative terrain or obstacle placement.
Invalid physical parameters, schema versions and over-budget packs fail before
flight. The sci-fi example has different radius, gravitational parameter,
rotation, hydrostatic dry atmosphere, relief scale, snow/tree lines, terrain palette,
vegetation density, tree heights and species mix.
It is an artistic/game-model example, not validated planetary science. Its globe
material also follows the preset palette. Vacuum disables cloud/haze shells, and
zero or retrograde rotation retains a finite geographic north.

Windows packaging includes High and SciFi command launchers. Actual ThinkPad GPU
support, frame rate and installer behavior must be checked on Windows; 32 GB of
system RAM does not establish a GPU performance class.

## Data and preparation

The bundled data comprises western Sweden (10–15° E, 57–60° N), Norwegian
mountains (6–10° E, 60–63° N), and a coarse global ETOPO fallback. The public surface sampler accepts full
body-fixed positions; the local quadtree renderer still covers a tangent region
rather than implementing continuous detailed terrain on every hemisphere. Elevation uses 1,201 samples per degree (about 93 m latitude spacing); land cover
uses 2,401 samples per degree (about 46 m latitude spacing). Longitude spacing is
smaller at these latitudes. They are resampled from GLO-30 and
WorldCover sources, not a claim of 10 m runtime geometry or surveyed bare earth.

| Pack | Runtime sample payload |
|---|---:|
| Western Sweden | 129,744,045 bytes |
| Norway mountains | 103,795,236 bytes |
| Global base | 12,960,000 bytes |

Ground material arrays are shared by every region. BC3 is selected when the
adapter exposes BC texture compression; mipmapped RGBA is the bounded fallback.
The two 512² × 6 arrays occupy about 4 MiB compressed or 16 MiB uncompressed,
including mipmaps. The checked-in source JPEGs are preparation inputs.

```sh
python3 -m venv /tmp/nord-terrain-tools
/tmp/nord-terrain-tools/bin/pip install -r scripts/environment/requirements.txt
/tmp/nord-terrain-tools/bin/python scripts/environment/prepare.py --pack global
/tmp/nord-terrain-tools/bin/python scripts/environment/prepare.py --pack sweden-west
/tmp/nord-terrain-tools/bin/python scripts/environment/prepare.py --pack norway-mountains
/tmp/nord-terrain-tools/bin/python scripts/environment/materials.py
python3 scripts/environment/packs.py list
python3 scripts/environment/packs.py verify sweden-west
```

The importer caches source downloads in the OS temporary directory, emits
little-endian int16 heights and u8 categorical cover, and reconciles shared
borders. Rebuilding with different `--samples` or `--cover-samples` values requires removing the
old generated output first. The default shipped grids fit entirely in the
384 MiB CPU terrain budget, remain pinned and immutable during flight, and need
no query-time disk I/O. Larger future packs must be split or use a paging
implementation; the current loader rejects an over-budget selection.

Use `packs.py install /path/to/pack` or `packs.py remove pack-id` with the game
closed. Installation verifies SHA-256 and stages a temporary directory before
renaming it. Missing regions fall back to the same coarse geography for both
physics and rendering, with a diagnostic warning for detailed scenarios.
The tooling is CLI-based; a graphical region-download browser is not included.

Source URLs, licenses, adaptations and limitations are in
`assets/environment/SOURCES.md` and per-pack/material manifests. Source
mean-sea-level heights become radial offsets from the simulator's reference
sphere; no ellipsoid/geoid survey conversion is implied. Copernicus DSM canopy
and building heights are not completely removed. The source data is historical.

## Code ownership and runtime behavior

- `environment` owns versioned physical/graphics definitions, immutable snapshots,
  geographic conversion, regional/global sampling, surface classification,
  normals, water levels, deterministic vegetation and large-prop placement.
- `terrain` owns quadtree LOD, CPU mesh construction, async staging/publication,
  PBR material arrays and shared vegetation rendering. `terrain/lod.rs`,
  `terrain/mesh.rs`, `terrain/material.rs` and `terrain/vegetation.rs` isolate
  these jobs from the streamer.
- `scenarios` owns geographic anchors and headings. The existing Såtenäs frame
  remains the home airfield. Carrier exercises begin on actual Skagerrak water;
  BVR begins over Norwegian mountain terrain. The off-field diagnostic selects
  a flat grass classification near the home field. Placement is representative,
  not an aviation database or surveyed landing-site guarantee.

`Flight` owns an `Arc<EnvironmentSnapshot>`; background sampling shares the
immutable active snapshot. Switching planets requires a process restart, so
workers cannot publish results from a previous planet. Compatibility height
functions route existing aircraft, contact, AI, missile and map consumers into
the environment service. Raw ground and contact/water-top heights are distinct.
Earth reference constants remain available for legacy regression fixtures;
runtime consumers use active radius, gravitational parameter and rotation.

The legacy terrain generator is retained exclusively for reference launches and
the existing deterministic tests. Normal Earth removes its artificial mountain
and rectangular carrier-sea carve. Airport pavement still needs a localized
civil-engineering grade; this is not a new surveyed airport model.

The renderer uses actual framebuffer height and estimated geometric error for
quadtree selection, with split/merge hysteresis and crack-hiding skirts. The
error is sampled, not a conservative continuous bound. Continuous geomorphing
is not implemented. Original async jobs, bounded uploads, old-coverage retention,
shared anchors and retirement remain. Replacement readiness includes material
bindings and ground texture arrays in addition to meshes.

Ground materials use photographic CC0 detail, macro land-cover color, normal
and roughness maps, mip filtering, and stochastic triangular patch blending.
Variance correction approximates contrast preservation; full histogram-LUT
remapping is not implemented. Steep terrain blends toward granite color and uses additional triplanar rock
projections; flatter surfaces retain the cheaper single-projection path.

Forests use land-cover-constrained, clustered deterministic cells. Pine/spruce
branch whorls and birch crown clusters use opaque meshes. Background jobs combine
them into spatial-cell meshes, with at most two builds and one cell publication
per frame. Each cell shares one transform, and canopy LOD uses hysteresis. This
avoids updating thousands of separate tree entities as the flight origin moves. Distant meshes reduce geometry and shadows; forest color remains
on the terrain. Sparse rocks and settlement blocks share placement with contact
queries. These are generated meshes, not photogrammetric tree assets or baked
impostor atlases. Grass is currently material detail, not individual grass blades.

Near/far water masks preserve the existing wave/shading model. Sea geometry stays
at the configured sea level; fixed world-space lake grids carry their own source
elevations. Shared shore vertices extend wet heights onto dry banks instead of
interpolating toward zero. Water replacements wait for meshes, images and
material bindings before publishing. Mask construction is asynchronous;
interpolation avoids the earlier square mask edges. Coastline precision remains
limited by source and runtime sampling. The wave field and ditching model remain
game estimates, not a calibrated hydrodynamic simulation.

## Validation and performance

See `ENVIRONMENT_TERRAIN_VALIDATION.md` for this pass's actual results and limits.
The reproducible native runner is:

```sh
python3 scripts/environment/validate.py target/release/gripen-flight \
  tests/environment-upgrade/my-run --case forest-external --case forest-cockpit \
  --case coast --case mountain --case airfield --case carrier --seconds 25 --warmup 15
```

It records actual display dimensions, world scale, environment, asset telemetry,
frame percentiles, power state and simulation timing. It suspends only the existing
Nord Flight process during sequential diagnostics and restores it afterward.
Do not run builds concurrently with accepted timing samples. Paused terrain
scripts measure rendering/streaming, not live-flight throughput. GPU intervals
can overlap and must never be added into a fabricated frame budget.

The 60 FPS / p95 ≤18 ms / p99 ≤25 ms goals require measured acceptance. Lower
world rendering resolution is a deliberate graphics choice; it must be reported
when comparing with native-resolution baseline results. Windows acceptance and
a completed 20-minute soak must not be inferred from short Mac captures.

Use `python3 scripts/environment/summarize.py <run-directory>` to extract frame
target checks, process telemetry ranges, and per-scenario entity/asset bounds.
The summary explicitly distinguishes retained CPU assets from total GPU residency.
`capture.py ... --case forest-dense` captures a WorldCover-selected forest region
at approximately 12.4321° E, 59.2125° N; the original default route is mixed
farmland and woodland rather than a dense-canopy stress test.

Standard graphics now uses 2× MSAA plus FXAA. Camera debanding dither is disabled
to avoid magnifying screen-space noise when compositing a reduced-resolution
world. Ground detail fades according to its pixel footprint and distance.

The optional frame-rate limiter is disabled by default: testing showed a
regression when combining timer-based limiting with Vsync on this Mac. It runs
before Bevy updates simulation time when explicitly enabled.
`GraphicsSettings.frame_rate_limit` accepts 0 (unlimited) or 30–240 Hz in a
custom graphics JSON. This is a ceiling, not a guarantee that rendering meets
the chosen rate. The limit is preserved when switching Standard/High.


## Mountain flying start

In flight setup, choose **Atmospheric Flight → Mountain Flying / Norway**, select
an aircraft, and press **Fly**. The unopposed scenario starts near 61.25° N,
7.70° E at 190 m/s, gear up. Starting height is at least 600 m above the local
surface and increased if necessary to clear ridges along the next 8 km by 300 m.
The pilot controls the flight after launch; this is not automatic terrain following.
It uses the Norway pack on both Earth and sci-fi Earth. The geographic starting
point is separate from the BVR scenario, so combat starts keep their existing setup.


## Scene loading

Starting or resetting a flight now shows an opaque loading screen with an animated
spinner and a description of the current stage. The destination aircraft state
is held while terrain coverage, water boundaries, vegetation cells, aircraft
scenes and GPU meshes/materials/textures are prepared. Flight controls are blocked
during loading; the previous pause setting is restored only after readiness and
visibility settle. Loading time is excluded from simulation advancement, so the
aircraft does not fall or consume flight time while waiting. A newer scenario
reset invalidates the previous readiness acknowledgement. Distant terrain can
continue normal view-dependent streaming after launch.


Loading validation: the full suite passed with 887 tests and one ignored test;
the final loading regressions passed separately. A native home-region-to-Norway
transition took 4.42 seconds (455 update frames) with aircraft simulation time
still 0.000 seconds at release. The loading overlay and released scene were
captured under `tests/scene-loading/`. This measurement describes that transition
on the current Mac, not a fixed loading-time guarantee for every scene.

The loading overlay fades in over 2 seconds of wall-clock time. The outgoing
world image remains frozen during that fade; destination rendering starts behind
the opaque overlay, preventing unfinished terrain from showing through. Once the
scene is ready, the overlay fades out over 2 seconds. Aircraft simulation remains
paused throughout both fades and resumes only after the overlay has disappeared.

Flight UI is hidden from the first loading frame through the end of the fade-out.
This includes explicitly visible child widgets and panels created during loading.
Their normal visibility rules resume afterward; intentionally hidden panels stay
hidden. The loading artwork and world-image compositor remain visible.

Flat-ground surface colors now use a geographic classification atlas rather than
interpolated mesh vertex colors inside installed packs. A conservative 3×3
categorical majority removes isolated vegetation/field flecks while preserving
mixed edges, water, snow, and building classes. Palette colors are reconstructed
per pixel with a sharpened bilinear transition confined to class texels. Numeric
material IDs are never interpolated. Large triangles therefore no longer spread
field colors across unrelated ground or invent intermediate material types.

When class texels become smaller than a screen pixel, the shader transitions to
an area-filtered, linear-light color mip pyramid. Two-texel neighboring-data halos
and continuous geographic derivatives avoid degree-boundary sampling jumps.
Near texture contrast is reduced so it does not obscure land-cover boundaries.
The active preset supplies the palette, including sci-fi Earth. Heights, water
masks and collision classifications retain their existing authority.

Shared atlas resolution is selected once at flight startup and capped by the
smaller of half the configured GPU environment budget and the staging allowance.
Near classifications use R8Uint storage; only distant colors need RGBA mipmaps.
GPU publication waits for these textures along with the existing terrain assets.
Rock blending uses the interpolated authoritative slope attribute, rather than
triangle face derivatives, to avoid coloring coarse terrain as individual facets.
Boundary sharpening is applied to accumulated class weights, not independently
to the two texture axes, which reduces square corners in the reconstructed mask.
Geographic UVs are prepared by mesh workers. The fragment shader performs no
spherical coordinate conversion, skips boundary reconstruction in uniform land
cover, and stops high-resolution classification filtering for subpixel texels.
Geographic UVs are stored relative to the home-region origin, subtracting in f64
before conversion to f32. This retains centimetre-scale variation for near-ground
texture derivatives; absolute latitude in f32 can quantize these derivatives and
incorrectly select the finest mip, causing grain and moire.

Validation for this surface-color change: 18 terrain tests and 11 environment
tests passed. Packaged native captures cover Earth at 50 m and 260 m AGL, sci-fi
Earth, and the Norway mountain scenario. The final bundle passed strict signature
verification. Artifacts and executable hashes are under
`tests/surface-color/delivery/`.

The final paired battery-powered paused-render study used a 2880×1800 interface
and 1440×900 world image, 20 seconds warm-up and 30 seconds sampling. The old
build averaged 47.6 FPS (p95 38.20 ms); the delivered build averaged 103.4 FPS
(p95 25.02 ms, p99 27.43 ms). This is one noisy desktop comparison, not proof
of a general speedup. The p95/p99 acceptance targets are still unmet in that
sample. Earlier AC-power and different-display runs are recorded separately;
live-flight timing and long-duration soak acceptance are outside this focused
surface-color validation.
## Nearby 2D ground cover

Flight Setup → Settings → **Ground Cover** cycles Off, Balanced, and Dense.
This control is independent of world-render resolution and the main graphics
tier. Existing graphics files load the new optional `grass_distance` (112 m)
and `grass_density` (0.65) fields automatically. Dense uses 160 m / 0.85;
advanced configuration accepts a maximum range of 192 m and density 0–1.

Grass and broadleaf undergrowth use two crossed, alpha-masked cards per tuft,
batched into 64 m cells with one shared, procedurally generated 256 × 128
mipmapped atlas. They do not cast or receive shadows and have no collision
shapes. There are no imported assets, per-blade entities, alpha blending, or
per-frame camera-facing updates. Palette, seed, and vegetation height come from
the active planet preset, including sci-fi Earth.

Placement uses authoritative land cover, slope, water levels, snow line, airport
pavement exclusions, and shared building/rock footprints. Airport verges have
shorter grass. Stable clustering and varied size, heading, and color avoid
regular placement bands. Cosmetic roots match the triangles of the published
terrain mesh; this does not change the authoritative contact surface.

Balanced has at most nine cell batches and 1,024 tufts per cell: about 3.8 MiB
of active mesh payload at the cap, plus roughly 171 KiB for the shared texture.
Dense has at most 21 batches; the maximum configurable layout has 29. These
are source payload bounds, not measurements of total GPU allocation. Generation
uses the existing background worker with at most two jobs and one pending GPU
publication. Retirement is bounded to four cells per frame. Cache keys include
planet identity, environment revision, density, and the local terrain layout;
obsolete jobs are cancelled and meshes wait for GPU dependencies before showing.

The ground-cover radius contracts with aircraft height and disappears above its
configured range. Distant vegetation remains the existing tree/canopy system.
`tests/grass/validate.py` records sequential on/off release profiles, a moving
streaming sample, and Earth/sci-fi/menu captures. Disposable diagnostics can use
`NORD_GRASS_OFF=1`; normal users should use the in-game control.
