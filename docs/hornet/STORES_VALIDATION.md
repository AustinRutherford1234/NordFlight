# Hornet store and faster-actuator validation

2026-10-28. Configuration, controls, reference provenance and estimated parameters are in [STORES.md](STORES.md).

## Automated results

The final full suite passes **602 tests**, zero failures, in 15.26 seconds on this Mac. This includes the preceding aircraft, carrier, collision, FBW, combat, crew, HUD and scenario-reset regressions. Thirteen focused checks were added to the preceding 589-test suite, and existing AIM-9X aircraft-compatibility assertions were extended to include the Hornet.

```sh
cargo test -- --test-threads=4
cargo test aircraft::hornet::fbw -- --nocapture --test-threads=1
```

Local evidence: `tests/hornet-stores/final-regression.log`. A separate **16/16 FBW** run is recorded in `fbw-response.log`. With faster actuators, the bounded high-load case peaked at about **6.28 g**, and the jammed-surface case at **5.30 g**. The gust and engine-out recovery cases settled near **.991 and 1.000 g**. These are results of the estimated game plant/controller, not Super Hornet flight-test measurements.

New coverage verifies:

- Two AIM-9X and six AIM-120 launches use their matching rack coordinates, consume all eight missiles and reject further launches.
- Hornet IR launches enable the existing AIM-9X GNC; wingtip releases have no generic downward rack kick.
- Tank capacity, dry/fuel mass, CG shift, positive inertia, remaining-fuel discard and reduced drag after release.
- External fuel feeds running engines even with zero internal fuel; internal fuel is consumed only after external fuel is exhausted.
- The opposing ejection impulse preserves total linear momentum in a zero-rate release case.
- A loaded, trimmed aircraft remains intact in five seconds of flight, then remains intact for five seconds after tank release.
- Dropped tanks stop on the carrier deck; released records are bounded and separate from installed mass.
- F6 tank-loadout selection, tank-only versus emergency-all jettison inventories, and reset restoration/clearing of the chosen tank loadout.
- Wingtip store folding and launch inhibition with folded wings.
- Finite tank geometry, sufficient body volume for the stated capacity, and faster actuator response with retained rate, hydraulic and jam limits.

## Native app inspection

```sh
./build-mac.sh dev
python3 scripts/fa18f/capture_stores.py
codesign --verify --deep --strict 'Nord Flight.app'
```

Native screenshots were captured and visually inspected for the loaded configuration, folded wingtips with their missiles/rails, tank separation at **0, .25 and .75 seconds**, and AIM-9X/AIM-120 release. The loaded screenshot was also inspected as a close crop. The corrected missile harness resumes the simulation before requesting fire; the first attempt was correctly inhibited while paused and is not counted as successful launch evidence.

The loaded and separation sequence shows three tanks attached at their stations, then departing beneath the aircraft, while missiles remain mounted. Missile captures show the released model apart from the aircraft and the corresponding ammunition decrease. Source-state diagnostics report:

| Diagnostic case | Aircraft mass | External fuel |
| --- | --- | --- |
| Three full tanks and all missiles | 24,621.534 kg | 4,360.794 kg |
| Immediately after all tanks released | 19,870.740 kg | 0 kg |
| Loaded, one AIM-9X released | 24,537.164 kg | 4,360.794 kg |
| Loaded, one AIM-120 released | 24,469.534 kg | 4,360.794 kg |

The tank-release mass difference is **4750.794 kg**; missile-release differences are **84.37 and 152 kg**. These correspond to the implemented inventory and mass model.

The fold inspection uses the imported rig's existing pivots, transformed into the aircraft frame; both the missile body and its rail follow the fold. Native images and logs stay local under `tests/hornet-stores/`; the capture script and headless checks are committed for reproduction.

These deterministic native captures establish rendering and release-state behavior. They are not a manual flight sortie, certified store-separation analysis, or the user's hands-on flight review.

## Local app delivery

The optimized development app is rebuilt in `Nord Flight.app`. The strict deep signature check passes. See the final executable hash below. The generated app and images remain local; source, documentation and capture tools are committed and pushed.

Final executable SHA-256:

```
cc0e7fe1d6cfcc1ba5dffd3a5e814bff4a882716927ec527a7bb8654ed1281f0
```
