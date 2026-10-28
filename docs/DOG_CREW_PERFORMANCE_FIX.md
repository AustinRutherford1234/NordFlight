# Dog crew performance regression fix — 25 October 2026

The dog portrait introduced a second active 3D camera. The existing environment-map renderer permits cached/sliced sky-light filtering only with a single filter encoder, because independent encoders may otherwise submit in an unsafe order. The portrait unintentionally counted as another encoder and ran the global generator nodes even though it renders only local cockpit content. This forced complete sky-environment filtering twice per frame.

In the clean paused baseline, 385 sampled frames produced **768 cache bypasses and 768 full filter publications**. The corrected run produced **1,647 cache hits and zero bypasses/publications** during the sample. Disabling only the portrait in an earlier diagnostic run restored a median frame time of 8.17 ms, corroborating the cause.

## Change

- Add the extracted `SkipEnvironmentMapGeneration` camera marker in the bundled Bevy PBR renderer.
- Mark the local crew portrait camera. Both global downsampling and filtering nodes return before encoding work on that view.
- Exclude only opted-out views from the generator's encoder count. Genuine additional generation encoders retain the original multi-view fallback and ordering protections.
- Avoid flagging unchanged crew UI nodes, portrait-camera activation and exterior pilot transforms as modified every frame.

The detailed models, geometry, materials, lighting, **640 × 640 portrait target, 4× MSAA, and portrait animation cadence are unchanged**. No flight-force or simulation timing changes were made.

## Matched measurements

Apple M1 / Metal, macOS, 1440 × 900, optimized development build, standard graphics. VSync was disabled only for benchmarking. The original executable was retained as `/tmp/gripen-crew-before-performance`. Runs were sequential without concurrent compilation. Paused runs used 20 seconds of warmup plus a 15-second sample; live runs used 12 seconds of warmup plus a 12-second sample.

| Scene | Mean uncapped FPS, before → fixed | Median frame ms, before → fixed | p95 frame ms, before → fixed |
|---|---:|---:|---:|
| Paused F-22 | 25.7 → 109.7 | 35.95 → 8.43 | 50.11 → 11.43 |
| Live F-22 (same start) | 27.3 → 85.4 | 36.32 → 8.47 | 39.15 → 26.06 |

The paused scene is the primary identical-state comparison. Live runs share initial conditions but evolve differently: the existing simulation advances a configured step per rendered frame, so uncapped faster rendering advances farther in simulated time. Live results corroborate responsiveness but are not an identical-trajectory physics comparison. Normal game VSync settings were preserved.

Raw reports:

- `tests/crew-performance-final-before.json`
- `tests/crew-performance-final-fixed.json`
- `tests/crew-performance-live-before.json`
- `tests/crew-performance-live-fixed.json`
- Initial ablations: `tests/crew-performance-disabled.json` and `tests/crew-performance-no-portrait.json`

Diagnostic environment switches: `NORD_CREW_DISABLE=1` omits the crew plugin; `NORD_CREW_NO_PORTRAIT=1` leaves crew state/models/UI present but disables the offscreen camera. These are developer ablations, not production defaults.

## Checks

- Nine crew tests passed, including all 30 breed/equipment mesh combinations and a new regression test proving that unchanged portrait layout no longer marks UI nodes changed each frame.
- Ten renderer cache/slicing tests passed. The encoder test verifies that a portrait opt-out preserves single-encoder reuse, ordinary second encoders still trigger the fallback, and removing the opt-out restores ordinary behavior.
- Native GPU portrait inspection confirmed that the complete model remains visible with the optimization.
- Mac app rebuilt and ad-hoc signature verified. Windows runtime was not measured.

## Normal presentation check

A final live-flight run with the ordinary AutoVsync setting averaged **60.0 FPS**, with median **16.67 ms** and p95 **17.00 ms** over 12.01 seconds after 12 seconds of warmup. Full crew rendering was enabled. Raw report: `tests/crew-performance-vsync-fixed.json`.
