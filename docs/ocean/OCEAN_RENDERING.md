# Gaussian ocean surface

See [the optimization investigation](OPTIMIZATION_2026-10-30.md) for the subsequent rejected candidates and profiling limits. The original runtime was retained after that investigation; the measurements below describe this initial delivery.

The former local water was a smooth StandardMaterial on a 90 km grid with 900 m cells. The new local surface uses a seeded 24-mode Gaussian random field, directional log-spaced wavelengths from 12 to 220 m, and deep-water angular frequency `sqrt(g * |k|)`. Spectrum normalization gives an offshore significant wave height estimate of 1.8 m (`4 * sigma`, linear-sea approximation). This is a visual engineering model, not a calibrated regional sea forecast. Inland water is reduced to one tenth of offshore amplitude; the carrier sea boundary blends smoothly.

## Cost decisions made before integration

- Generate spectrum, concentric mesh, ripple texture and ten mip levels once. No per-frame mesh or image creation/upload, compute dispatch, FFT, reflection pass, screen-space transmission or CPU tessellation.
- A single reusable 48,129-vertex / 96,000-triangle mesh and opaque forward material remains opaque during ordinary flight. It fades only during the existing high-altitude transition and hides in map view.
- Evaluate 24 wave components at vertices, not every fragment. Smoothly suppress waves that the local cells cannot resolve. Two filtered/rotated and spatially warped ripple texture samples supply short-scale normals and breakup; subpixel detail becomes roughness.
- Concentric mesh rings share boundaries and grow outward with no separate tiles or LOD cracks. Camera-following geometry is positioned on a four-metre lattice, with phase anchored to absolute coordinates. CPU phase reduction before f32 upload preserves wave phase through render-origin shifts and long simulation times. Curvature uses a difference-of-squares expression to avoid subtracting two Earth-radius f32 heights.
- Reuse one material, one mesh and one approximately 1.33 MiB mipmapped RGBA8 texture. Only a small uniform block/transform changes with the simulation clock or camera. Paused unchanged inputs do not mark the material modified.
- Do not cast or receive shadows on the ocean. Existing lighting and environment reflection remain in use. No new reflection or refraction render targets.

## Presentation

The shader combines geometric swell/wind waves, analytically differentiated large-wave normals, filtered fine slopes, crest/trough color variation, dielectrically configured reflectance, distance-dependent roughness and sparse curvature/height-based foam. Foam is a procedural crest appearance, not a breaking-wave fluid solver or persistent foam transport model. Fine detail fades before the horizon to avoid crawling highlights.

The CPU sampler returns height, surface normal and vertical surface velocity. Ditching initiation, buoyancy/damping, splash origin, foam placement and water-side fragment support use it. Flight above the possible crest range exits before evaluating waves. Ships retain the approved fixed course and attitude: no new vessel motion, wakes or wave-force dynamics. Geometric wave LOD deliberately removes unresolved distant displacement; nearby contact uses the full large-wave field. Microscopic ripple normals are visual only.

## Validation procedure

`cargo test --locked` includes spectrum variance/bounds, analytic slope/time derivatives against finite differences, CPU/shader phase equivalence across anchors, mesh budget and full mip-chain checks, plus existing aircraft, carrier, ditching and effects regressions.

Approved native views: low surface, identical view six seconds later, carrier approach, high altitude, sunset and night. The paired rendering harness runs at fullscreen normal quality for 10 seconds of warm-up plus 30 seconds of sampling each: old flat-water reference and new animated waves. Both prescribe the same moving fleet and camera while pausing flight integration. These checks measure rendering, not full flight physics. Existing sessions are stopped and resumed in `finally` blocks. Captures and raw JSON reports stay under ignored `tests/ocean/`.

References: [NVIDIA GPU Gems, Effective Water Simulation from Physical Models](https://developer.nvidia.com/gpugems/gpugems/part-i-natural-effects/chapter-1-effective-water-simulation-physical-models) describes separating geometric waves and small normal detail. [Tessendorf, Simulating Ocean Water](https://www-evasion.inrialpes.fr/Membres/Fabrice.Neyret/NaturalScenes/fluids/water/waves/fluids-nuages/waves/Jonathan/articlesCG/simulating-ocean-water-01.pdf) provides the Gaussian spectral foundation; this implementation is a bounded direct sum, not an FFT implementation.

## Measured results

Final paired measurement on this Mac, 2026-10-30: optimized development build, normal quality, fullscreen 2880×1800, AutoVsync, 10-second warm-up and 30-second sample per run. No builds ran during sampling. Both runs prescribe the same moving fleet/camera and advance simulation time for animation while pausing flight integration.

| Case | Frames | Mean frame time | Mean FPS | p50 | p95 | p99 | Frames >50 ms |
|---|---:|---:|---:|---:|---:|---:|---:|
| Original flat water | 2,047 | 14.672 ms | 68.16 | 9.035 ms | 26.713 ms | 35.806 ms | 12 |
| Gaussian ocean | 1,893 | 15.858 ms | 63.06 | 9.384 ms | 26.549 ms | 32.653 ms | 7 |

Added mean frame time in this pair: **1.186 ms** (about 8.1%). This is **0.186 ms above the proposed 1 ms budget**. Median increased 0.349 ms; p95/p99 did not worsen in these samples. Single sequential VSync/scheduling-affected samples cannot resolve all run-to-run variation or establish worst-case cost. The result does not establish a locked 60 FPS, performance during full flight/collision simulation, every water-heavy camera, or energy use. No longer soak or Metal trace was run. Existing game sessions were restored after the approved checks.

Automated evidence: the complete suite passed 667/667 after the shared physics integration. Following the fine-texture refinement, the four focused ocean tests passed again. Native review found a repeating fine-ripple pattern in the first pass; the final version expands the texture to 512×512, uses 64 Gaussian Fourier components, and adds low-frequency coordinate distortion. All six views were recaptured and inspected after that change. The first-pass images remain local under `tests/ocean/first-pass/`.

Native captures reported no fatal renderer or shader compilation errors. Existing escort glTF hierarchy warnings still appear during scene creation. The night capture is almost black under the existing nighttime lighting; no artificial water emission was introduced. These six diagnostic views do not constitute a manual flying session, a full shoreline survey, or a soak test. Shared contact behavior is covered by automated ditching/impact regressions; ships remain kinematic.

The updated app passed strict deep code-sign verification. Its built and packaged ARM64 executables share UUID `D03B995D-ABD0-3E8C-836A-720835347BC9`.
