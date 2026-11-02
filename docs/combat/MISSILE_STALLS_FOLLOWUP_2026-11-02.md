# Remaining missile-scene stalls: warmed CPU and memory investigation

## Measurement changes

The earlier GPU-counter allocation failure was a profiling failure, so this follow-up adds `NORD_PROFILE_CPU_ONLY=1`. It retains CPU spans, render-asset preparation wall timing, upload counts and frame intervals while omitting the Bevy GPU diagnostic recorder and timestamp graph. Spike logs include the preceding frame's CPU-span deltas and wall time. These spans can overlap and include host scheduling; they cannot be summed as exclusive CPU cost.

All compared native trials use the seeded 30-cruise-missile scene, normal-quality fullscreen cockpit, live physics and a 20-second warm-up. A diagnostic freeze keeps cockpit textures at their warm-up state but leaves weapons/physics running. It is never enabled in ordinary flight.

| Trial | Sample | Average FPS | p95 frame, ms | p99 frame, ms |
|---|---:|---:|---:|---:|
| CPU-only, with a five-second OS stack sample | 30 s | 45.41 | 52.54 | 73.95 |
| CPU-only, instrument textures frozen after warm-up | 30 s | 62.62 | 28.17 | 29.28 |
| CPU-only, live instruments, repeated without stack sampling | 30 s | 56.55 | 28.88 | 31.85 |
| Exact ship-frame cache, live instruments | 30 s | 49.44 | 31.95 | 63.61 |

The repeat shows that the worst earlier tail was not consistently reproduced. Freezing displays reduces steady work, but the close p95 values of the frozen and unsampled live trials do not establish display updates as the sole cause of long stalls. The cached trial reduced simulation cost while its overall frame tail worsened; no end-to-end speedup is claimed for that trial.

## Exact carrier transform reuse

The stack sample found repeated deck/ship transform calculations. The new per-thread cache holds only three ship frames. Its value key includes time bits and every damage-motion input: translation, quaternion, linear/angular rates and motion epoch. The same-time reset/damage test compares against the original formulas exactly. Deck queries reuse their computed frame internally. No wheel forces, contact sampling, deck outline, collision narrow phase, physics rate or ship motion is simplified.

The repeated 12,000-query fixture measured 4.371 ms for recomputation versus 1.762 ms with caching, including common reference work in both loops. This is a small kernel measurement, not an FPS claim. In the native trials, average collision-check span cost fell from .387 to .097 ms per call and simulation span cost from 3.670 to 1.459 ms per rendered frame. The trial conditions and host scheduling varied, so these are observed costs rather than a controlled guarantee.

## Memory evidence

This Mac has 8 GiB physical RAM. A separate running Nord Flight session measured 4,788 MiB physical footprint, peak 5,674 MiB, with 2,786 MiB in the footprint tool's swapped/compressed column. Its largest categories were owned graphics memory, Malloc Large and Malloc Small. RSS alone understated its memory usage.

The user authorized ending that flight and reopening the app afterward. Subsequent memory/soak trials use a single game instance. A paused original process would retain its allocations, so a two-instance memory comparison would be misleading.

The single-instance, pre-texture-change soak used a 20 s warm-up and 60 s measured sample. It reported 49.74 FPS, p95 41.382 ms and p99 75.194 ms. Footprints at approximately 15, 48 and 78 s were 4,688 / 4,688 / 4,311 MiB. Across the full process-monitor interval, system-wide VM counters advanced by 239,356 swap-in pages and 300,531 swap-out pages (16 KiB pages), approximately 3.65 / 4.59 GiB. These counters include other processes; they do not assign every stall to Nord Flight. They establish active system swapping during the trial.

Offline inspection of the unchanged embedded PNG/JPEG dimensions found 560 MiB of decoded RGBA images in the carrier GLB, 192.75 MiB across the three Su-27 LODs, and about 17.4 MiB across the bomber LODs. Burke textures add further allocations. Compressed files on disk are much smaller than these decoded buffers.

## Texture retention change

Imported carrier, escort, fighter and bomber assets now request `GltfLoaderSettings.load_materials = RenderAssetUsages::RENDER_WORLD`. The installed Bevy loader passes this setting to embedded images and external image loader settings. Meshes and StandardMaterials remain in the main world for geometry, animation and lighting changes. Only immutable model texture CPU copies are released after renderer extraction.

Texture dimensions, pixel values, samplers, shaders and GPU texture copies are unchanged. Dynamic HUD/map textures, crew images, terrain/planet textures and the Hornet's imported assets retain their existing paths. `NORD_MODEL_CPU_REFERENCE=1` restores the original model texture retention for opt-in comparisons. No on-disk asset or attribution is modified.

The benchmark report now includes retained main-world image/mesh byte totals and the largest CPU images. This is distinct from GPU allocation and process physical footprint. The process monitor records RSS, system-wide VM counters and occasional `footprint` snapshots; those diagnostic queries can affect scheduling and are not sampled hardware frame timings.

## Completed post-change validation

The first post-change run completed its 80-second window, then the newly added memory-report code attempted to inspect a mesh already extracted to the render world and panicked. That was an end-of-report diagnostic bug, not a model texture import failure. The report now skips render-only mesh CPU buffers; a regression covers this distinction. The repeated run completed and wrote its report normally.

| Single-instance soak, live cockpit | Before texture retention change | Final updated build |
|---|---:|---:|
| Warm-up / measured interval | 20 / 60 s | 20 / 60 s |
| Footprint near 15 s, MiB | 4,688 | 3,311 |
| Footprint near 48 s, MiB | 4,688 | 3,338 |
| Footprint near 78 s, MiB | 4,311 | 3,336 |
| Average FPS | 49.74 | 42.22 |
| p95 / p99 frame interval, ms | 41.38 / 75.19 | 44.51 / 103.22 |
| Simulated seconds during measured interval | 59.59 | 57.02 |

The texture retention change reduced the observed footprint by about 1,350 MiB at the middle snapshot and remained near that lower level through the soak. It **did not establish an FPS improvement or eliminate stalls**: the final measured frame tail was worse. The report retains that negative result. No fixed 60 FPS claim is made.

System-wide swap-in/out counters in the final process-monitor interval advanced by 7.28 / 7.08 GiB in logical page units, versus 3.65 / 4.59 GiB in the baseline. These are uncompressed logical page quantities, not measured compressed disk byte counts. They include other applications and show substantial host memory churn despite the smaller game footprint. The steady footprint snapshots and bounded entity/asset counts over this short soak do not establish the absence of a longer-duration leak.

A subsequent same-user process snapshot found aggregate memory-footprint charges of approximately **6.53 GiB for Chrome and its helpers**, **.91 GiB for Creative Cloud processes**, and **2.13 GiB for Codex/ChatGPT processes**. These include nonresident compressed/swapped accounting and are not resident-RAM totals; some shared accounting may overlap. This is evidence of substantial competing memory demand, not proof that a particular Chrome tab caused a particular frame stall. Other applications were not closed. A clean background-app comparison was not performed.

Final main-world retained assets measured approximately 495.8 MiB of image data and 234.2 MiB of mesh buffers. The largest remaining image copies include the Earth/Moon maps and generated runtime images; those paths were left unchanged because some are used by CPU-side region and display generation. This accounting excludes renderer allocations.

All **788 automated tests passed**, zero failed, with the opt-in timing diagnostic excluded from the ordinary suite. Carrier, fighter and bomber texture views were captured in separate native app runs and visually inspected. The Su-27 retained its painted skin, radome and canopy; carrier deck/hull details and bomber markings remained visible. Screenshots are local under `tests/missile-spikes/texture-*.png`; they are model-render checks, not an additional flight-performance sample.

The memory-saving texture setting is retained for its measured allocation reduction and unchanged rendering data, not as a demonstrated FPS fix. The exact ship-frame cache is retained for its deterministic equivalence and measured CPU-cost reduction. Remaining end-to-end stalls are unresolved under the measured host workload; reducing competing application memory demand is the next useful comparison.


Delivery: rebuilt and ad-hoc-signed `Nord Flight.app`; strict deep signature verification passed. Executable SHA-256: `c592aeedb13d30280a7b834e7c89a80decb6b4f1d61c6e82e959d26c0d2f6970`. The user-approved prior flight was ended for the single-instance tests; the app is reopened afterward, rather than claiming that the ended flight state was restored.
