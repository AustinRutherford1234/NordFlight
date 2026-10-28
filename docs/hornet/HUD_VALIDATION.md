# Hornet HUD validation

2026-10-28. Design, controls, data sources and fidelity limits: [HUD.md](HUD.md).

## Automated evidence

The final full suite passes **589 tests**, zero failures, in 21.27 seconds on this Mac. The final focused HUD run passes seven tests. Together with the added cockpit-action, keyboard-input and high-resolution crop checks, ten tests were added to the preceding 579-test baseline.

```sh
cargo test -- --test-threads=4
cargo test cockpit::hornet_hud -- --nocapture --test-threads=1
```

Local logs are `tests/hornet-hud/final-regression.log` and `final-focused.log`. The checks cover:

- Compressible CAS calibration at sea level through Mach 2, lower pressure, and continuity across Mach 1.
- Horizontal/vertical projection signs, rearward rejection, and incidence-bracket direction.
- A banked crosswind horizon reprojected back into world space, confirming zero world elevation in both caged and uncaged operation.
- Mode differences, declutter, battery/avionics blanking, default/reset construction and absence of flight-position, velocity, attitude, fuel, flap or throttle mutation during drawing.
- Terrain impact intersection and rejection of an upward gun trajectory with no impact during the actual projectile lifetime.
- Keyboard/cockpit mode, declutter, cage and altitude-source operations, existing stores, unchanged throttle and other-aircraft shortcut behavior.
- 1024 × 768 crop mapping as well as the existing 256 × 192 crop/empty/recovery tests.

The isolated optimized-development CPU raster sample, 120 frames, measured **0.626 ms mean / 0.695 ms p95** at 1024 × 768. This covers raster generation, not complete cockpit processing, GPU work, upload or total frame time. It is not a flight-simulation performance certification.

## Native rendering

```sh
./build-mac.sh dev
python3 scripts/fa18f/capture_hud.py
codesign --verify --deep --strict 'Nord Flight.app'
```

Native Bevy cockpit screenshots were captured and visually inspected for NAV, infrared-missile A/A, radar-missile A/A, A/G, carrier approach, a 30-degree bank, REJ 2 and battery off. The original level-flight A/G case correctly showed NO SOLUTION. The added dive preset produced the visible impact pipper with about 1.1 NM slant range and .48-second flight time. It is a game-gun solution, not real M61 ballistic validation.

Captures live in `tests/hornet-hud/native-*.png` with matching process logs. The two capture passes are recorded in `native-captures.log` and `native-additional.log`. Test-generated transparent raster frames, including caged operation, are in the same directory. Generated images/logs stay local; the test and capture script are committed so they can be regenerated.

The initial native inspection found buttons overlapping lower symbology and daylight reflection washing out strokes. The buttons were moved to the instrument panel, and the Hornet HUD material uses a black reflective base with emitted green strokes. Final captures show the lower HUD unobstructed. Existing optional world-space navigation gates are separate from HUD power and may remain visible when its raster is blanked.

The capture harness injects deterministic aircraft/target states and freezes them for inspection. Native captures and Bevy input/action tests establish rendering and binding behavior; they do **not** constitute manual OS-level piloting, a complete live weapons sortie, or the user's flight review. No real-aircraft certification is claimed.

## Delivered app

`Nord Flight.app` was rebuilt using `./build-mac.sh dev` (optimized development profile). Strict deep code-sign verification passes. Final executable SHA-256:

```
7ceb0b492c899b6f3b90a2b50f2537dc29c2a21b567340c9b5f31b9584bbb3b0
```

The app bundle remains a local generated artifact. Source, documentation and the reproducible capture harness are delivered through GitHub. Existing flight-model, FBW, carrier-force and other-aircraft regression tests remain passing.
