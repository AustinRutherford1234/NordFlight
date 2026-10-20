# Near-terrain rendering performance

Scope: Gripen flight over land and forests, including a larger/fullscreen-sized
render target. No aircraft dynamics, collision geometry, AI rate, integration,
tree population, material resolution, shadow settings or render resolution are
reduced by this work.

## Findings

Initial large-window workloads rendered **2880 × 1800**, four times the pixel
count of the earlier 1440 × 900 tests. Those exploratory runs emitted Bevy's
current-monitor selection warning. The final harness selects the primary
monitor explicitly at window creation; each report records its actual framebuffer
dimensions. The warning-bearing experiments are kept separate from acceptance.

An initial forest test coordinate was actually inside Lake Vänern after shoreline
initialization. The corrected diagnostic uses x = -8000 m, z = -1000 m, verifies
`forest=true`, and places the aircraft 50 m above the actual ground. Earlier lake
route experiments in this evidence directory are excluded from the accepted
forest comparison.

The focused Metal capture is in
`tests/terrain-performance/metal-small/aggregate.json`. It identifies terrain
fragment shading, sky-light filtering and the outer directional-shadow cascade
as important GPU costs. Event intervals on different GPU channels overlap and
must not be added to claim frame wall time. CPU physics is separately measured;
the moving render-only path deliberately pauses authoritative simulation.

## Implementation

An experimental option partitions forest geometry into tighter spatial batches. Original
positions, normals, vertex colours, UVs, triangles and tile anchors are retained
exactly; the renderer can reject more offscreen geometry in camera and shadow
views. Streaming retains the complete old coverage until replacements are ready,
and new batches receive their camera-relative transforms before becoming visible.
It reduced potential submitted vertices, but did not earn a reliable FPS gain
and increased draw submissions. **Fine batching is disabled by default.** The
shipping forest geometry uses the original batch sizes.

The diffuse environment filter assigns one GPU workgroup to each output texel
and distributes the original 1024 seeded samples across 64 lanes. The specular
filter uses eight sample lanes per texel, with eight texels per workgroup.
Both retain every sample, input texture, output resolution and filtering cycle.
Only the association of floating-point additions changes; this is presentation
math, not authoritative simulation math. Each uses 1 KiB of workgroup storage;
neither retains a new persistent sample cache. Small final mip levels mask
padded texels before writing.

The renderer also stops submitting global haze/cloud shells while their shader
opacity is identically zero. It reopens visibility two metres before the zero
threshold to cover CPU/GPU float rounding at Earth radius. Visibility is checked
before the material-cache early return, so paused views and map returns remain
current. This does not hide local cumulus, distance fog, or the visible sky.

The forward-cockpit capture identified a much larger transparent-fragment cost
than the side-looking test pose. The PBR shaders now avoid evaluating a point
light with zero RGB intensity or zero range attenuation, and evaluate a spot
light's exact cone weight before its expensive BRDF. Only an exactly zero weight
returns early; no light range, brightness, falloff or enabled contribution changes.
After decals, zero-alpha `Blend` fragments are discarded before lighting. Their
original blend result contributes no colour, alpha or depth. Premultiplied and
additive modes are excluded, as are depth/deferred prepasses.

Developer reference switches:

- `NORD_TERRAIN_LARGE_BATCHES=1`: original forest batching.
- `NORD_TERRAIN_BATCH_SIZE`: experimental batch size in metres (128–2048).
- `NORD_ENV_SERIAL_IRRADIANCE=1`: original serial-per-texel environment filters.
- `NORD_EARTH_DRAW_ZERO_ALPHA=1`: original submission of zero-opacity shells.
- `NORD_REFERENCE_LIGHTS=1`: original PBR evaluation without zero-contribution skips.
- `NORD_BENCH_FULLSCREEN=1`: large-render diagnostic window.

The ordinary controls and shipping VSync setting are unchanged. A depth-prepass
experiment was slower and was removed.

## Evidence and reproduction

The starting installed executable is preserved at
`tests/terrain-performance/baseline-release`; `starting-hashes.json` records
starting source hashes. Same-executable reference switches isolate rendering
changes without rebuilding between runs.

```sh
python3 tests/terrain-performance/run.py target/debug/gripen-flight tests/terrain-performance/my-comparison
```

The runner uses sequential ABBA order, a ten-second warmup and twenty-second
sample, checks the forest location, records the actual framebuffer size, and
does not run a compiler concurrently. Pass instrumentation is disabled for final
FPS runs (`NORD_BENCH_FPS_ONLY=1`); `--profile` enables separate, instrumented
studies. Its `--cockpit` option selects the forward cockpit;
`--windowed` selects the default 1440 × 900 render target.

The original batching and spatially partitioned stationary forest screenshots
at `tests/terrain-performance/verified-forest/{reference,candidate}.png` are
**pixel-identical at 2880 × 1800**. All 305 application regressions passed before
the additional GPU-filter experiment, including exact triangle-attribute
preservation and terrain first-frame-transform coverage.

## Final validation — October 18, 2026

The final release built successfully in 3m 48s and was installed in
`Nord Flight.app`, stripped and ad-hoc signed. Strict signature verification
passes. The original installed executable remains preserved as
`tests/terrain-performance/baseline-release`.

**307 application regressions pass**, including a new ECS regression for cached
atmosphere opacity across small descents. The visibility decision uses the eye
position actually bound to the shader while its material cache remains valid;
it must not hide a shell whose cached opacity is still positive.

Native reference/candidate captures are in
`tests/terrain-performance/validated-visuals/`. They cover forests, the forward
cockpit, and Earth flight at 12 km and 60 km, all at 2880 × 1800. The forest pair
is pixel-identical. In the cockpit, the canopy and instrument-panel comparison
regions are pixel-identical; the sky/window regions differ by at most one
8-bit channel level. Full-frame differences also include changing FPS text and
animated effects, so their maximum pixel error is not an equivalence metric.
No visible lighting or geometry regression was found in the inspected images.

The final timing runs use the installed release executable, 20-second warmups
and 20-second samples, with no concurrent compilation. Same-executable reference
switches restore the original lighting evaluation and shell submission. These
are controlled rendering comparisons, not a claim that live combat or physics
throughput was benchmarked. Source hashes confirm the authoritative aircraft,
weapons, collisions, orbital propagation and audio files were unchanged.

The laptop was on battery. Creative Cloud and macOS post-restart analysis
services consumed substantial, varying CPU time during some samples; each
summary records the busiest processes and power state. Thermal state was not
measured. These conditions limit causal FPS claims, especially comparisons
between early and late runs. The post-restart development-build runs are
excluded from accepted performance evidence. Sixty FPS in the large framebuffer
is **not achieved**; no resolution or quality setting was reduced to meet it.

### Final measured rendering results

| Workload | Reference FPS | Candidate FPS | Reference p95 | Candidate p95 |
|---|---:|---:|---:|---:|
| Forward cockpit, 2880 × 1800, two runs each | 28.95 / 29.38 | 30.75 / 32.55 | 51.29 / 35.42 ms | 51.51 / 34.57 ms |
| External forest, 2880 × 1800, one pair | 39.80 | 39.88 | 34.43 ms | 34.61 ms |
| Forward cockpit, 1440 × 900, one pair | 59.85 | 59.80 | 17.54 ms | 17.18 ms |

The large cockpit averages across the two runs are 29.17 versus 31.65 FPS
(8.5% observed increase), but varying background load prevents attributing that
entire difference to the changes. Tail latency is not consistently improved:
the two reference cockpit samples contain 64 frames above 50 ms in total,
versus 72 in the two candidate samples. External forest FPS is effectively
unchanged. The smaller framebuffer remains near the VSync limit in both paths;
its candidate sample includes one frame above 50 ms. This is a bounded rendering
optimization, **not a complete fix for the fullscreen near-terrain slowdown**.

The summaries in `accepted-cockpit/`, `accepted-external/` and
`accepted-windowed/` contain individual percentiles, stutter counts, resolution,
power state and background processes. The packaged release was also launched
for another native cockpit image pair in `packaged-visuals/`.

### Remaining costs and retained evidence

The exploratory forward-cockpit Metal capture measured a median 14.12 ms
transparent-fragment active interval and 7.34 ms opaque-fragment interval.
Terrain shading, large shadow cascades and environment filtering remain material
costs. These overlapping GPU-channel intervals are diagnostic evidence, not
additive frame budgets or final-candidate pass measurements. Earlier filter
experiments reduced diffuse active intervals from about 3.88 to 2.16 ms and
specular intervals from about 4.76 to 3.70 ms while retaining samples and outputs;
those savings did not translate into a demonstrated external FPS increase.

Further work should isolate cockpit transparency and forest/shadow submission
under stable power/background conditions, with matched GPU captures before
changing either. The finer-batch and depth-prepass experiments are not shipping
defaults because they failed to demonstrate an overall benefit. No claim of a
Bevy hardware ceiling follows from these runs.

`tests/terrain-performance/delivery.json` records the installed executable hash,
signature result, tests, source manifest and evidence paths. Temporary development
executables and superseded experimental raw traces were retired; aggregate
results and retirement records remain. The evidence directory is approximately
311 MiB and Cargo's target cache approximately 8.5 GiB, below the existing 12 GiB
build-cache policy. The original release backup is retained.
