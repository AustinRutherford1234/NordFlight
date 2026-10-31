# Raid flight, tactical picture, cockpit and ordnance update

## Behavior

- Raid starts are **120 km from the carrier**, superseding the earlier miles request. Aircraft spawn at their assigned spherical-Earth altitude, with a horizontal inbound heading. A new four-aircraft raid may spawn immediately after every bomber in the latest raid has expended its weapons or been destroyed. Elapsed time alone cannot bypass this gate. The existing 12-aircraft and 96-airborne-weapon caps remain authoritative; full pools defer spawning/launching.
- Fighters escort their own raid's armed bombers, intercept the player using actual sensor memory, respond to observable missile warnings, avoid other aircraft, and withdraw once the bombers are finished. Bomber bay/release interlocks remain active. Ship attacks require a forward launch opportunity; fighters also require fresh awareness before launching at the player.
- The physical Su-27 plant/actuator allocator is retained. Naval AI requests pass through a governor for bank, load, angle of attack and low airspeed. Pilot commands never assign aircraft velocity or attitude.
- Tu-160 actors now have a dedicated reduced-order airborne plant, rather than Su-27 dynamics: mass/fuel/store changes, large inertias, quaternion attitude, Euler rotational coupling, aerodynamic lift/drag/sideslip damping, engine spool/thrust lapse and fuel flow, actuator rate/authority limits, and estimated FBW load/roll-rate/coordinated-turn response. Damage and shutdown affect the plant. It uses the existing Flight/simulation loop, atmosphere, rotating Earth, environmental wind, and damage state. No new Tu-160 surface animations are enabled.
- The full tactical map and cockpit tactical page combine valid local radar reports, existing BVR friendly reports, and confirmed fleet reports. Stale tracks coast at reported velocity and expire; hidden aircraft do not leak truth positions into sensor mode. Friendly ship positions and known friendly launches are shown. Hostile missiles require reports or local warning conditions. Map designation does not grant a seeker lock.
- **MAP DIFFICULTY: SHARED SENSORS / ALL CONTACTS** appears in flight setup; the same choice is available under BVR difficulty. The existing map reveal control remains available. This aid changes map visibility only. Tactical symbol/label refresh is bounded to 10 Hz and uses 128 reusable symbol groups, sufficient for the bounded mission pools.
- **6** cycles player, enemy and ship-launched airborne missiles. **Backslash → 6** remains the radio sweep and consumes the input before camera handling. Ending the last weapon flight restores the previous camera.
- Naval weapons now use the same retained, textured smoke renderer as AIM-120 weapons, with class-specific nozzle offsets and burn durations. Each pool slot retains at most 64 trail samples. The CPU and optional GPU paths share these rules; distant naval smoke is omitted beyond 35 km from the current camera. Cameras/maps and physical weapons remain active.
- Original class-specific low-poly meshes replace naval capsule placeholders: Standard-class, ESSM-class, air-to-air and anti-ship cruise missile. They include shaped noses, painted bands/seams, fins/strakes or cruise wings, and recessed nozzles. The currently implemented bomber weapons are cruise missiles, not free-fall bombs; this update does not add a new bomb weapon system.
- Hornet HUD controls previously overlapped the entire right radar-bezel column at x=0.425. They now occupy their own smaller row above the middle display. Radar caps retain separate labels and spacing. Both cockpit pickers now apply the inverse of the actual Hornet cabin scale, so visible buttons and click targets agree.

## Evidence and parameter status

The Tu-160 exterior source/attribution remains in `assets/aircraft/tu160/README.md`. Public geometry/mass-envelope context is available from the [Bank of Russia Tu-160 description](https://cbr.ru/cash_circulation/memorable_coins/coins_base/ShowCoins/?cat_num=5109-0108) and [UAC's aircraft overview](https://uacrussia.ru/ru/aircraft/lineup/military/tu-160/). The model is an engineering approximation, not a proprietary Tu-160 aerodynamic database or certified FBW law.

| Parameter | Value and units | Status / uncertainty |
| --- | --- | --- |
| Fixed-pose span | 42.97 m | Measured from converted visual model |
| Effective aerodynamic area | 300 m² | Estimate, ±25%; combines lifting-body/intermediate-sweep contribution |
| Dry aircraft + crew/equipment | 110,400 kg | Representative estimate, ±15% |
| Initial mission fuel | 40,000 kg | Scenario choice, not maximum fuel capacity |
| Cruise-missile store mass | 1,500 kg each | Scenario estimate, ±40% |
| Principal pitch/yaw/roll inertia | 35/50/15 million kg m² | Geometric estimate, ±35% |
| Four-engine reference thrust | 4 × 137.2 kN dry, 4 × 245 kN afterburning | Representative estimates; altitude/Mach lapse and spool response are unvalidated estimates |
| Lift slope / saturation | 4.5 per rad / CL 1.25 | Estimated aggregate polar, ±30% |
| Drag | CD0 .023, induced factor .075, estimated wave/separation increments | Estimate, ±40% |
| Physical actuator travel / rate | ±.45 rad / .28 rad/s | Estimate, ±40%; hydraulic degradation reduces rate |
| Normal FBW load request | .3–2.2 G | Conservative simulation protection, not a sourced operational limit |
| Bomber route bank / roll-rate command | 35° / .35 rad/s | Mission/controller choices |
| Engine spool time constant | 2.8 s | Estimate, ±40% |
| Fuel flow factor | .000015 kg/(N s) | Estimate, ±40% |

An online search found [Lwifff's free generic missile](https://sketchfab.com/3d-models/missile-low-poly-1c5534e678c44f639311c9344bcd3f8d) and [sakigakefuruzawa's free missile/torpedo pack](https://sketchfab.com/3d-models/low-poly-missiles-and-torpedos-99783c90ce904951a3c71e851a527d35). The generic asset has 12.6k triangles and does not cover the four roles. The pack's download API returned HTTP 401; the available tools could not access the user's authenticated browser session. The approved original-model fallback was used. No external missile assets or additional licenses are incorporated. Standard-class model dimensions (4.72 m × .343 m) match the [Navy's public Standard Missile description](https://www.navy.mil/DesktopModules/ArticleCS/Print.aspx?Article=2169011&ModuleId=724&PortalId=1). Other missile dimensions/fittings are representative visual estimates, not exact block replicas.

## Validation

Automated, native visual, and user flight evidence are separate. Final results are recorded below after validation. Scripts/captures stay under `tests/naval-upgrade/`; the capture harness enforces an aggregate sub-10-minute diagnostic budget and restores prior running sessions.

### Final automated results

`cargo test --locked`: **718 passed, 0 failed** (58.60 s test runtime). Coverage includes the existing flight/Hornet-lock/HUD/carrier/camera/ejection/reset regressions plus:

- All visible radar caps pick their own actions through the scaled Hornet cabin; the separate HUD row and sockets clear the existing front controls.
- Shared fleet reports reveal confirmed contacts without leaking current truth positions; stale reports expire. Practice-mode radar contacts appear, all-contact mode is explicit, inactive scenarios clear contacts, and 120 km raids fit the 300 km map.
- Difficulty-map selection survives the launch flow. The crew portrait hides while the map is open so southeast contacts remain visible.
- Key 6 cycles player/fleet/hostile weapons and restores the camera when the final flight ends; existing backslash/radar input-consumption tests pass.
- Every naval ordnance class has finite colored geometry at representative scale, fins, and a recessed nozzle. The common smoke tests, including the optional GPU encoding path, pass with class-specific nozzle offsets.
- Time alone cannot spawn a new raid while an armed bomber survives; completing bomber releases permits immediate spawning, and the aircraft cap still defers launches.
- Heavy-bomber level and banked flight remain bounded over 180 simulated seconds each; equivalent 60/120/240 Hz trajectories pass convergence checks. Shutdown removes thrust; raw commands still pass through physical actuator rate limits.
- A 600-second, four-aircraft transit from 120 km reaches release and withdrawal without pose steering. In the recorded diagnostic run, bomber load peaked at 1.334 G and angular rate at .2244 rad/s. All actors stayed alive and finite; six anti-ship rounds were released. The local state history is `tests/naval-upgrade/raid-flight.json`. This is a repeatable simulation check, not operational flight-data validation.

### Native review and delivery

**17 native captures across 13 distinct views**, including three cockpit angles, shared/reveal maps and the 120 km start, all four weapon classes, a wide smoke view, and live bomber/fighter flight. Aggregate approved app diagnostic runtime was **329.1 seconds**, below the ten-minute limit. Prior sessions were restored by the harness. Final map captures show all four 120 km contacts without the portrait covering their labels. Naval weapon telemetry reports modeled class, allegiance, speed, age, motor time and guidance status, without displaying fictitious propellant values.

Weapon close-ups and map fixtures deliberately freeze simulation; their purpose is geometry/UI/effects inspection. Live aircraft views use full flight physics, with weapon launches disabled in those disposable fixtures to keep the inspected aircraft available. Automatic combat and release behavior are exercised separately in the deterministic tests. User flight review remains outstanding.

The Mac app was rebuilt with `./build-mac.sh dev`; its ad-hoc signature passed `codesign --verify --deep --strict`. No new sustained FPS benchmark or optimization gain is claimed in this update. Performance controls are bounded pools/trails, shared meshes/materials, 10 Hz tactical layout refresh, distance-limited smoke, and a shared 192-triangle weapon exhaust rather than an aircraft-resolution plume.
