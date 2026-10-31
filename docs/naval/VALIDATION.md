# Carrier Defense validation — 2026-10-31

## Automated and build evidence

- `cargo test --locked`: **701 passed, zero failed**. Log: `/tmp/naval-all-tests-final.log`.
- New tests exercise deterministic two-fighter/two-bomber raids, pending-wave capacity, confirmation/classification/coasting, horizon and disabled sensors, player-relative reports, true-bearing agreement with the actual Flight/HUD geographic frame, safe-deck servicing, reset and persistent battle respawn, camera restoration, swept missile fuzes, physical anti-ship hull strikes, SAM interception of a moving aircraft, ballistic CIWS interception, ship superstructure obstruction, finite magazines/pools, compartment flooding/fire/magazine damage, sinking after mission end, and aging/reuse of destroyed aircraft slots.
- Existing aircraft, Hornet physics/FBW/HUD locks, carrier arrest/catapult, ejection/canopy reset, collision, and rendering/mesh regressions also pass.
- `./build-mac.sh dev` rebuilt the existing Mac application. ARM64 executable UUID: **3097116F-CDD5-383D-B796-6D823FA05D46**. Ad-hoc signature passes `codesign --verify --deep --strict`.
- Sources and exact machine-readable results are recorded in `validation-results.json`.

## Native inspection

The approved diagnostic window was used for repeated six-view inspection and fullscreen comparisons. Total interruption was approximately **597.4 seconds**, including the final budget-capped probe. Each harness restored the previous running session.

Inspected the Hornet HUD/report, carrier, both escort cameras, ship fire/smoke, SAM flight presentation and scripted sinking. Review findings led to correcting the HUD capture orientation, clearing legacy combat overlays from ship views, keeping the camera above the sea as its ship sinks, softening smoke, and correcting the true-north calculation. Final refreshed HUD/port/sinking captures are under `tests/naval-review/`; earlier carrier/starboard/damage captures establish those views and model presentation but predate the geographic-bearing correction.

The final HUD screenshot reports a selected contact at **203° true**, matching the HUD steering direction relative to its approximately 184° heading. Unit tests compare headings in all quadrants, including across north. Radar guidance does not grant missile lock.

The screenshot states are scripted and mostly paused. Damage and sinking states deliberately inject damage. The physical hit/interception claims come from simulation tests, not screenshots. Speech synthesis generated a valid 22,050-Hz mono PCM WAV; manual radio intelligibility and a user-flown combat sortie remain review items.

Native logs contain GLB texture-coordinate and hierarchy warnings associated with the existing imported scenes. No fatal ECS/render error or panic was observed in the completed reviews. These warnings were not treated as proof of a new subsystem failure or silently classified as fixed.

## Fullscreen performance

Apple M1, normal quality, Metal, **2880 × 1556**. Both completed comparison samples use the same port-escort orbit camera, nominal ship motion and player start. Baseline disables the battle; active sample runs the scripted raid with full flight/collision simulation. Existing user session was suspended. No compilation or headless stress run overlapped the samples.

| Sample | Recorded duration | Mean frame | P95 frame | P99 frame | Mean FPS | Simulation / real time |
|---|---:|---:|---:|---:|---:|---:|
| Fleet baseline | 30.00 s | 16.93 ms | 26.37 ms | 37.94 ms | 59.05 | 1.000 |
| Live defense battle | 60.02 s | 17.86 ms | 29.23 ms | 42.29 ms | 56.01 | 1.000 |

These are descriptive workload samples, not proof of an optimization gain or a controlled isolation of each subsystem. Engagements change the scene and simulation work over time. The active run reached **19 simultaneous naval rounds**, with **21 allocated reusable round slots**. Warmed counts stayed in a narrow range: 12,990–12,993 entities, 109 images and 2,137–2,140 meshes. Aircraft were killed during the sample; it is not a sustained 12-aircraft saturation measurement.

RSS was approximately 300–322 MiB during/at the end of the battle sample. RSS is not GPU allocation or total physical footprint, and a one-minute sample cannot establish absence of a leak. The final wreck-effect lifecycle correction was made after this paired capture; its executable passed all 701 tests and launched in a final native probe. Subsequent corrections align the defensive readout with the selected preset, require bombers to prioritize the carrier, and retire projectiles when a run ends. All 701 tests pass after those changes. These paired FPS figures therefore remain provisional measurements of the earlier capture build, not final-build FPS certification. That probe was terminated at the approved time limit before producing another complete performance report. The paired capture's executable UUID was **F9CB9412-99A3-3D83-9AA0-06C931E1864E**.

**Steady 60 FPS is not established.** Frame-time spikes remain. The implementation bounds work and storage; it does not claim to eliminate the existing renderer's performance limits.

## Multi-wave simulation

`NORD_NAVAL_STUDY_PRESET=1 NORD_NAVAL_STUDY_SECONDS=660 target/debug/gripen-flight --study-naval` completed **660 simulated seconds at 120 Hz**, covering three Normal-difficulty raids. This used executable **E267A9F6-13ED-39AF-B88C-49AA8BF9608C**, including corrected bomber carrier priority and wreck lifecycle logic. A subsequent end-of-mission cleanup retires projectiles instead of leaving them frozen; it does not change this surviving-carrier run. The full 701-test suite passes after that final cleanup.

- Carrier survived at full hull health; player remained alive on the deck.
- Five ship aircraft kills and 18 missile interceptions occurred.
- Maximum active aircraft records: eight; maximum simultaneous naval rounds: 12.
- Finite magazines were consumed: carrier 7 short-range rounds remaining; escorts 10/16 Standard-class and 32/32 short-range rounds remaining.
- No raid was fully defeated by the unattended fleet during this run. Surviving attackers keep those raid records active; there is no false success award.
- All aircraft positions remained finite. The capacity/reuse tests separately exercise the 12-aircraft and 96-round limits.
- Mean whole-simulation step time: **2.275 ms**; P95: **1.787 ms**. These are wall-time measurements, include rare slow steps, and were taken with the user's app resumed. They are not an isolated dedicated-core benchmark and are not rendered-frame timings. The mean exceeds the earlier project's 2-ms-per-rendered-frame physics ambition; that target is not claimed as met for multi-aircraft naval combat.

## Practical limits and acceptance

Missile aerodynamics/seeker behavior, radar classification, CIWS burst footprints, damage-control rates, hydrostatics and weapon installations are documented estimates. The representative bomber has original geometry and uses the existing generic AI physics plant. Naval fighter missiles use reduced point-mass tracking rather than the legacy dogfight missile's complete decoy-discrimination path; countermeasure controls/presentation remain available, without claiming identical effectiveness against this new threat model.

The automated evidence and scripted reviews support delivery for testing. They do not establish exhaustive naval damage validation, operational Aegis fidelity, steady 60 FPS, or completion of the user's flight review.
