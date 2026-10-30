# Ocean optimization follow-up — 2026-10-30

## Outcome

**The original renderer is retained. No FPS optimization from this investigation is enabled in the application.** All three candidates measured slower in their respective comparisons. The installed executable was restored byte-for-byte to the original SHA-256 `08936a4bc49c818d2e3a1917352e4c03186f07c6f5680307e73b212083a4e535`, with ARM64 UUID `D03B995D-ABD0-3E8C-836A-720835347BC9`, and passed strict deep signature verification. This avoids presenting lower nominal work or reduced storage as a demonstrated speedup.

The source-verified material-recreation cost and the implemented experiment are preserved for further profiling in [persistent_uniforms.patch](experiments/persistent_uniforms.patch). It is **not applied** to the runtime. Against the unchanged `69025a2` runtime sources, use `git apply --unidiff-zero docs/ocean/experiments/persistent_uniforms.patch`, rebuild, and run `tests/ocean-optimization/streaming.py`. The patch application was checked without applying it to the final checkout.

## Implemented and tested experiment: persistent uniform updates

The starting point is commit `69025a2`. The experiment keeps the ocean shader, all 24 spectral components, mesh density, original RGBA8 ripple texture and mipmaps, lighting, foam, and CPU contact model unchanged.

The source review found a specific cost: `src/ocean.rs` marked its material modified on every animated frame. In `vendor/bevy_pbr/src/material.rs`, material preparation explicitly frees/recreates its binding even when only uniform contents change (the source has a TODO for a fast path). Thus a small parameter update was also causing material resource preparation.

The experimental path keeps the material static during animation and extracts a `UniformSnapshot` for the current render frame. During render preparation, after material assets are ready, it writes the same encoded parameters into a persistent `UNIFORM | COPY_DST` buffer. The render-side staging vector is reused. Buffer ID and parameter checks skip redundant writes, including unchanged paused frames. Material preparation still happens when it is needed, such as an opacity-mode change. Newly prepared buffers are detected by ID and receive current parameters before drawing.

The update occurs in the render world, rather than writing ahead from the pipelined main world. All f64 world-phase calculations and the existing shader uniform layout are preserved; this introduces no wall-clock approximation, lower update rate or new control behavior. The old asset-update path remains available through the diagnostic-only `NORD_OCEAN_ASSET_UNIFORMS=1` switch for same-executable comparisons.

The first native check caught that Bevy's derived uniform buffers have `UNIFORM` usage without `COPY_DST`. The persistent path now explicitly allocates the binding with both flags. The original renderer was restored while that correction built. The validation-error log is retained locally; the corrected native view rendered without a fatal GPU error.

## Final validation

Six focused ocean tests passed for the experiment, covering spectrum/derivatives, phase rebasing, mipmaps, animated snapshot updates without material mutation, unchanged paused snapshots, reset and map visibility. A corrected native screenshot was inspected with the original ocean appearance retained. This is visual inspection, not an assertion of identical screenshot pixels.

`tests/ocean-optimization/streaming.py` uses one executable and one bundle for both paths. It performs the approved screenshot plus two 30-second fullscreen samples, each after 20 seconds of warm-up, at 2880×1800 normal quality with AutoVsync. Both prescribe the same moving fleet and animated clock while flight physics is paused. Only the uniform-update path changes. No builds run during sampling. Whole-run counters expose successful material-extension preparations and direct buffer writes; these include warm-up and are not normalized sample-only counts.

| Same-executable path | Sample frames | Window FPS | Raw mean interval | p95 | Whole-run material preparations | Whole-run direct buffer writes |
|---|---:|---:|---:|---:|---:|---:|
| Original asset updates | 1,931 | 64.36 | 15.551 ms | 26.615 ms | 2,912 | 0 |
| Persistent buffer | 1,727 | 57.55 | 17.378 ms | 29.545 ms | 1 | 2,730 |

The implementation removed the targeted material preparation work but did not improve frame rate in this pair. It was removed from the default runtime along with the other rejected candidates. The experiment's built/packaged UUID was `789E8A9D-B132-3DB1-B5A2-666E4C8F33A3`; the final delivered app is the restored original identified above. No GPU-time benefit is claimed.

The effect of repeatedly updating one GPU buffer while prior frames may still use it is a further synchronization hypothesis, not a confirmed cause. A rotating-buffer/dynamic-offset approach could be tested with GPU timing evidence; fewer bind-group preparations alone was not sufficient.

## Rejected experiments

1. **Shader branches:** skipped waves after their existing filters reached zero and skipped fully faded ripple fetches. Nominal unique-vertex wave evaluations fell from 1,155,096 to 544,536, but the corrected repeated app measurements were slower. The branches were removed.
2. **RG8 texture plus precomputed shader coefficients:** halved the mipmapped texture payload from 1,398,100 to 699,050 bytes and moved invariant filter/curvature arithmetic out of the vertex shader. The final paired frame times were also slower. This combined candidate was removed; the result does not isolate which change or hardware condition caused the difference.

An initial six-run shader comparison had a setup difference (unbundled baseline) and is excluded from a gain claim. The corrected comparison used identical app-bundle metadata and exactly shared assets, a 20-second warm-up, and three 30-second samples per version in order A B B A A B:

| Run | Version | Window mean frame time | Window FPS | p95 interval |
|---|---|---:|---:|---:|
| 1 | Baseline | 16.254 ms | 61.52 | 26.815 ms |
| 2 | Branch candidate | 19.378 ms | 51.60 | 28.864 ms |
| 3 | Branch candidate | 23.511 ms | 42.53 | 33.850 ms |
| 4 | Baseline | 17.302 ms | 57.80 | 28.838 ms |
| 5 | Baseline | 18.417 ms | 54.30 | 30.766 ms |
| 6 | Branch candidate | 22.103 ms | 45.24 | 33.702 ms |

Mean across run means: baseline 17.325 ms, branch candidate 21.664 ms. The RG8/coefficient pair measured 22.227 ms / 44.99 FPS for its baseline and 24.765 ms / 40.38 FPS for the candidate; p95 was 33.657 versus 33.735 ms. These are scheduled frame intervals, not GPU timings. They justified rejecting the candidates, not a hardware-level causal claim.

## Trace and measurement limitations

A separately approved five-second Metal System Trace reached its recording limit but failed to finalize within 90 seconds. Attempting to export its table of contents returned `Document Missing Template Error`. It provides no usable GPU attribution. Its unusable approximately 1 GiB output was removed after preserving the error logs. Temporary executable/bundle copies were also removed to recover disk space.

The legacy timing report can include part of a warm-up boundary-crossing frame in its raw mean interval. Window FPS here uses rendered frame count / reported sample duration. Both raw reports and window rates are retained; no individual frame times were silently discarded. VSync, scheduling and other desktop activity affect these measurements. A diagnostic snapshot showed this 8 GiB Mac using about 11.7 GiB of swap during the investigation; that alone does not prove active paging caused a particular frame spike. No claim is made of a locked frame rate, worst-case timing or full-flight performance.

Existing escort glTF hierarchy warnings remain. The GPU trace timeout and its export failure are retained as limitations, not GPU evidence.

## Reproducibility and further candidates

The Python harnesses and this report are tracked; generated captures, raw JSON/logs, rejected-candidate source snapshots and binaries are ignored. `streaming.py` reproduces the persistent-buffer experiment only after the patch has been applied and the app rebuilt. `prepare.py` and `benchmark.py` reproduce the earlier A/B bundle method when supplied a preserved baseline executable; the original baseline SHA-256 was `08936a4bc49c818d2e3a1917352e4c03186f07c6f5680307e73b212083a4e535`.

Further work should investigate spatially culling sectors of the full-circle ocean mesh, per-pixel coordinate-warp cost and the general PBR lighting path. Sector culling adds draw calls and requires conservative bounds during displacement/rebasing. Moving warp evaluation to vertices or simplifying lighting may alter appearance. None is presented as a demonstrated gain. The rejected attempts are evidence that nominally fewer shader operations are insufficient grounds for shipping an optimization on this hardware.
