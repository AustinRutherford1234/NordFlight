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
branch whorls and birch crown clusters share opaque meshes and materials for
Bevy batching. Distant meshes reduce geometry and shadows; forest color remains
on the terrain. Sparse rocks and settlement blocks share placement with contact
queries. These are generated meshes, not photogrammetric tree assets or baked
impostor atlases. Grass is currently material detail, not individual grass blades.

Near/far water masks preserve the existing wave/shading model while placing
lakes at their actual source elevations. Mask construction is asynchronous;
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
