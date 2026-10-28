# Crew portrait fullscreen performance — 26 October 2026

The compact crew portrait previously rendered at 640 × 640 with 4× MSAA,
regardless of its displayed size. Fullscreen on the local display also renders
the main view at **2880 × 1800**, four times the pixels of the default 1440 × 900
window. Live fullscreen flight reproduced the reported slowdown in both views.

## Change

`src/crew/render.rs` sizes the shared portrait render target from the largest
visible portrait's **physical** UI dimensions, including Retina and UI scaling.
It retains 1.5× linear supersampling and the existing 4× MSAA, rounds up in
64-pixel steps, and bounds the result to 320–640 pixels per side. The default
244-pixel-wide panel uses 384 × 384; the 290-pixel editor preview uses 448 × 448;
larger or higher-density panels retain the full 640 × 640 target.

The target is changed only when its quantized size changes. Hidden portraits do
not resize it. The preceding UI layout supplies the size before the current
frame's camera update, avoiding a viewport/texture mismatch during rendering.
The capture/gallery path retains 640 × 640 output. The developer override
`NORD_CREW_PORTRAIT_REFERENCE=1` also retains the old fixed resolution.

Dog geometry, mesh detail, materials, lights, shadows, expression logic, animation
cadence and the exterior pilot are unchanged. Main-view resolution, graphics
settings, physics, timestep semantics, G-LOC control-loss behavior and the user's
saved settings are unchanged.

The benchmark now obtains simulated elapsed time from the flight state. Previously
`NORD_BENCH_FPS_ONLY=1` disabled the profiling counter that supplied this field,
incorrectly reporting a zero simulation rate despite live flight. This reporting
correction does not affect simulation execution.

## Initial uncapped live comparisons

Apple M1, 8 GB, Metal, standard graphics, 2880 × 1800 fullscreen, optimized
development build. Runs were sequential, with no simultaneous compilation.
Each run used 12 seconds of warmup and a 12-second measurement. VSync was
disabled only for this comparison; CPU/GPU diagnostics were disabled to avoid
Metal counter-buffer allocation failures. All crew features were enabled.

| View | Mean FPS, before → sized | Median frame ms | p95 frame ms |
|---|---:|---:|---:|
| Third person | 46.58 → 51.56 | 19.13 → 17.96 | 36.12 → 25.85 |
| Cockpit | 45.58 → 61.85 | 17.68 → 10.18 | 43.06 → 26.66 |

Raw reports are in `tests/crew-runtime-performance/before-full-{external,cockpit}.json`
and `sized-full-{external,cockpit}.json`. The preserved baseline executable is
`/tmp/gripen-before-current-perf-fix`.

These live runs share initial conditions, but are not identical-trajectory
comparisons: the existing simulator advances a configured step per rendered
frame. Faster rendering therefore travels farther during the same wall time,
changing terrain streaming and camera content. Short-run FPS also depends on
background activity and thermal state; these figures do not establish a steady
60 FPS guarantee across combat, forest, lighting and graphics settings.

The existing opt-in depth prepass worsened the external test (42.05 FPS) and
remains disabled. One utility-QoS worker run reached 54.70 FPS, but that single
comparison does not establish a repeatable scheduling benefit; the existing
worker policy is preserved.

## Additional comparisons and uncertainty

Normal AutoVsync presentation, 12-second warmup and 15-second samples:

- Third person: 49.07 → 51.97 FPS; p95 37.19 → 28.73 ms.
- Sized cockpit: 62.22 FPS, p95 27.74 ms. The reverse-order baseline was an
  outlier at 18.32 FPS with p95 162.53 ms. A subsequent host snapshot showed
  several busy VS Code helpers and system processes. This outlier is retained
  in the raw reports, but is not used to claim a 3.4× improvement.

A paused external comparison using the same updated executable and the
fixed-resolution developer override measured 62.51 FPS at 640² versus 60.60 FPS
with sizing (20-second warmup, 15-second sample). This comparison **does not
establish a frame-rate benefit in the paused scene**. Paused rendering can reuse
the environment filter, whereas the live reports show continuous source captures
and filtering. This distinction and host-load variability limit attribution of
the live gains to portrait sizing alone. The deterministic reduction is from
409,600 to 147,456 portrait pixels at the default panel size (**64% fewer**),
with the same model and MSAA sample count.

The raw reports include both these favorable and unfavorable measurements in
`tests/crew-runtime-performance/`. They support a reduction in portrait rendering
work and better observed live frame times, not a universal FPS guarantee.

## Verification

- All **479 tests passed**, including control-loss opt-in behavior, cockpit
  blackout/recovery, simulation timing, crew animation and mesh checks.
- New coverage checks supersampling at ordinary and high-density panel sizes,
  restoration to the full target when enlarged, and absence of image modification
  events when the panel size remains stable.
- The optimized development executable builds without warnings.
- Native fullscreen cockpit captures (`cockpit-sized.png` and
  `cockpit-reference.png`) were inspected at full view and with magnified portrait
  crops. Face features, eyes, ear silhouettes, suit and cockpit remain intact at
  the displayed size.
- Fullscreen exterior and combat captures (`exterior-sized.png`,
  `combat-sized.png`) were inspected. Crew portrait placement, status and
  expressions remain visible; the inbound-missile ring and warning label clear
  the portrait.
- `Nord Flight.app` was updated atomically, ad-hoc signed, and passed
  `codesign --verify --deep --strict`. Packaged and build executables share
  Mach-O UUID `445D99D3-E968-329C-A7B5-638A7FD8AA88`.
