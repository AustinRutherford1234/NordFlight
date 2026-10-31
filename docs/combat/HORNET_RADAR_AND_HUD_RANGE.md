# Hornet radar and HUD launch-zone correction

Date: 2026-10-31

## Behavior

The non-BVR Hornet previously searched only 5/10/20/40 km, gated radar missile releases at 20 km, evaluated lock accumulation with a separate short-range radar signal, and launched the short-range missile profile. This combination prevented useful long-range acquisition and launch outside the dedicated BVR scenario.

Hornet local search now uses the existing BVR scales of 40/80/160/240 km (160 km default). The scope projection and tactical display use that same selected scale. These are simulation search settings, not sourced APG-79 detection specifications. Detection still depends on scan direction, elevation, terrain visibility/horizon, target return and Doppler notch, radar power and damage. Other aircraft retain their local search scales.

A Hornet AIM-120 release requires a fresh own-aircraft radar track in TWS/STT, pointing, signal, launch-envelope and existing airframe/weapon interlocks. A fleet report by itself cannot supply missile lock. Tab can select a contact detected by either the fleet or the local radar in naval missions. Missile launch and midcourse guidance use the existing long-range BVR profile; powered, fresh radar support is required before autonomous seeker operation.

## HUD

In A/A mode, the HUD shares the radar display's measured-track launch-zone calculation:

- RNG: extrapolated measured slant range, nautical miles.
- RMAX: estimated maximum launch range at the current launch altitude, velocity and target closure.
- NEZ: existing model's conservative inner-zone estimate, 48% of RMAX.
- Vertical scale: target-range chevron, maximum-range tick and bracket for the estimated NEZ.
- IN NEZ EST: target inside the estimated inner zone; COAST EST for an aging track.

The existing RMAX formula is `clamp(30000 + altitude_m * 3 + speed_mps * 40 + closure_mps * 35, 28000, 115000)` metres. This is an engineering/gameplay estimate, shared with BVR, not a validated missile performance or guaranteed-kill calculation. No new claim of operational APG-79 or AIM-120 fidelity is made. Radar-off/failed, missing and expired tracks suppress the range envelope; the HUD shows RNG -- NM rather than inventing a measurement. Existing seeker/lock/readiness indications remain authoritative.

## Automated validation

`cargo test -- --test-threads=4`: 720 passed, 0 failed, 39.40 seconds test execution. This covers existing flight, carrier, naval, HUD, ordnance, cockpit and other-aircraft regressions. Dedicated additions check 60 km acquisition, fresh-track launch interlocks, long-range profile selection and datalink guidance, range-scale exclusion, power/damage, coasting/expiration and closure-dependent RMAX/NEZ. Inventory tests now acquire a real radar track before forcing their test-only lock state for station-release assertions.

## Native review

The separate native capture harness is `scripts/fa18f/capture_hornet_radar.py`, with an aggregate 180-second authorization limit, timeout and automatic restoration of suspended flights. Disposable fixed-contact fixtures exercise the actual scan/signal/track code and signal-weighted lock accumulation, then pause for repeatable cockpit and instrument captures. These fixtures are scripted inspection, not a user-flown intercept or an end-to-end missile kill validation.

Native checks completed in 43.7 seconds total, within the approved three-minute limit. All three fixed contacts were detected through the real TWS scan path:

| Contact slant range | Radar/weapon result | HUD result |
| --- | --- | --- |
| 60 km / 32.4 NM | Fresh track, full signal-weighted lock | RNG 32.4 NM, RMAX 53.4 NM, NEZ 25.6 NM |
| 20 km / 10.8 NM | Fresh track, full lock | IN NEZ EST, target chevron inside bracket |
| 120 km / 64.8 NM | Fresh radar track, launch inhibited OUT OF RANGE | RNG 64.8 NM, target chevron beyond RMAX |

Inspected the long-range cockpit view, all three HUD instrument images, and the long-range radar instrument image. Readouts and bracket positions agree with the logged envelope; the scope shows TWS 160 KM. Native fixtures pause for capture, so RESUME TO FIRE / INHIBIT is expected for otherwise eligible launches. Captures/logs are under `tests/hornet-radar-review/`; the checked-in summary preserves results without adding generated image files to Git.

`./build-mac.sh dev` completed and replaced the existing `Nord Flight.app`; `codesign --verify --deep --strict 'Nord Flight.app'` passed. Scripted evidence does not replace the user's final interactive flight review.
