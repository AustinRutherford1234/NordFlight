# Tu-160 bomber and Hornet rudder update — 2026-10-31

## Delivered behavior

Carrier Defense bomber-role aircraft use SIpriv's CC BY 4.0 **Lowpoly TU-160** supplied by the user. Fighter-role aircraft retain the Su-27. The original file, attribution, reproducible Blender converter, parameter/geometry inspection, and three runtime GLBs are included. The temporary B-1B fallback was removed before delivery.

The Tu-160 is normalized to 54.1 m length. The fixed source airborne pose gives approximately 43.0 m span. Landing gear, wings, control surfaces, and internal assemblies are static; no source animation clips or animation players ship in the runtime LODs. Four original bay panels rotate about longitudinal hinges. Bay doors take two simulation seconds to open/close, open inside 28 km while carrying weapons, and close after withdrawal or ammunition exhaustion. Bomber launches require at least 98% door opening, and release origins are under the corresponding forward/aft bays. These timings, positions, and mission release behavior are simulation estimates, not operational Tu-160 procedures. Existing mission flight dynamics and anti-ship weapons are retained.

The F/A-18F repair extracts the full swept trailing-edge rudders instead of only small fin-tip strips. Each rotates about its swept, outward-canted hinge using the existing actuator output. Vertical fin structure remains fixed. Control laws and aerodynamic authority were not changed.

## Rendering cost controls

- One shared scene asset per LOD, reused by all bomber instances; eight source materials retained.
- Fixed source pieces batched into one hull with material primitives; only four doors retain transforms.
- LOD triangle counts: 12,271 / 6,175 / 2,274. Runtime GLBs total about 1.65 MB.
- LOD transitions at 350 m and 1,800 m with 10% hysteresis. Door shapes remain full resolution at every distance.
- Bomber pool bounded by the existing 12-aircraft mission limit. Scene registration scans only newly added names. Hidden LOD doors are not animated.
- `NORD_BOMBER_REFERENCE=1` is a diagnostic-only switch to the previous procedural bomber for same-executable comparison; it is not a product setting.

## Verification

Automated and scripted evidence is separate from the user's flight review:

- `cargo test --locked`: **705 passed**, zero failures, including signed rudder transforms, raw actuator response, LOD hysteresis, bay-opening/release interlock, station alignment, empty-bay closure, and existing naval/flight/carrier regressions.
- F/A-18F Blender rig validation passed surface areas, finite geometry, normalized hinges, gear/door and closed-mesh checks.
- Tu-160 Blender validation checks all three LODs: four doors, no retained animation state, finite vertices, 54.1 m close-model length, bounded LOD dimensional error, and downward quarter-step hinge sweeps. All pass. Far-LOD length is 53.913 m (0.35% simplification error).
- Native captures: neutral/left/right Hornet rudders; front, rear, top, and belly closed/half/open Tu-160 views. Initial inspection caught an active source landing-gear action blending into the static pose. The converter now clears that action before baking the airborne pose; final underside captures show gear enclosed.
- Native diagnostic harness pauses and restores an existing Nord Flight session. Captures are retained locally under `tests/bomber-rudder-review/`.

Performance results are recorded below after same-scene fullscreen checks. These are rendering comparisons with mission physics paused, not busy-wave combat or user flight validation.

### Measured rendering results

Each sample used a 10-second warm-up and 30-second measurement, normal quality, the same executable/camera/mission state, and paused physics. Existing flights were suspended and restored. No Cargo builds ran during measurement.

| Sample | Model | Resolution | Mean FPS | p95 frame ms |
| --- | --- | --- | ---: | ---: |
| Alternating 0 | Old procedural | 2880 × 1800 | 63.5 | 25.86 |
| Alternating 1 | Tu-160 | 2880 × 1800 | 58.6 | 17.53 |
| Alternating 2 | Tu-160 | 2880 × 1800 | 37.2 | 106.89 |
| Alternating 3 | Old procedural | 2880 × 1556 | 31.8 | 95.25 |
| Confirmation 0 | Old procedural | 2880 × 1556 | 28.5 | 112.60 |
| Confirmation 1 | Tu-160 | 2880 × 1556 | 34.7 | 91.15 |

The initial matched pair gave 63.5 FPS for the old model and 58.6 FPS for the Tu-160. Subsequent runs showed substantial frame-time variation in both variants; the fourth alternating sample also ended with a different window size. The confirmation pair matched resolution but both variants still stuttered. These measurements **do not establish a reliable model-specific penalty or improvement**, and stable 60 FPS was **not demonstrated** across the session. Do not infer an optimization gain from the later Tu-160 sample outperforming the baseline. The three bounded LODs are retained without reducing the close model further on inconclusive evidence.

The initial GPU-counter-enabled attempt failed in Metal with `Cannot allocate sample buffer` / `Parent device is lost`. The successful comparisons used the existing `NORD_BENCH_FPS_ONLY` path; no GPU pass-cost claims are made. Machine-readable samples are in `tu160-validation.json`; local raw captures/logs remain under `tests/bomber-performance/`.

The existing Mac app was rebuilt with `./build-mac.sh dev` and its ad-hoc signature verified. A currently running flight retains its loaded executable/assets until relaunch. User flight review remains separate from the scripted evidence above.
