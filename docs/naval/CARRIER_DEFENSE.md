# Carrier Defense — implementation and parameter notes

Updated 2026-10-31. This is a representative, public-reference game model, not an implementation of operational Aegis software, classified radar performance, or certified ship hydrostatics.

## Playing

In the start screen choose **DEFENSE / DECK** or **DEFENSE / AIRBORNE**, then Easy, Normal, or Hard. The mission forces F/A-18F selection. The existing 25-knot carrier formation and DDG 125 / DDG 128 escort identities remain. Bombers explicitly prioritize the carrier; fighters can attack the closest surviving fleet ship or engage the player.

- Backslash opens/closes radio. **6** requests a radar sweep; Escape closes radio without opening Pause.
- Radio reports contact ID, player-relative true bearing, nautical-mile slant range, feet above sea level, and classification when identified. Confirmation takes two scans; classification takes four. Undetected enemies are not included. Reports are snapshots, not permanent target truth.
- **Tab** cycles confirmed ship contacts. The HUD adds numeric true heading and a steering marker for the selected ship track. This is not a missile lock: aim the aircraft and satisfy the existing AIM-9X/AIM-120 acquisition requirements.
- **7** cycles carrier, port destroyer, starboard destroyer, then restores the previous player view. Mouse orbit/zoom work. Flight and combat continue; no autopilot or protection is granted. Existing camera bindings outside the mission remain.
- **U**, **V**, and Backspace retain cannon / countermeasure controls through the existing combat subsystem.
- Radio **1** services the aircraft only when safely stopped on the usable carrier deck at idle, with no catapult/launch/arrest attachment. Raise the hook to release the wire, service, then use the existing stopped-aircraft catapult repositioning command to prepare another launch.
- After crash/ejection, radio **2** respawns on a usable carrier. The battle clock, enemies, damage, and ship magazines persist. Radio **3** ends the run. Scenario restart creates a fresh battle.

## Architecture and bounds

`src/combat/naval.rs` owns mission state, stable contact identities, raid scheduling, ship scans, radio controls, and servicing. Submodules implement weapons, compartment damage, static/shared procedural meshes, presentation, and asynchronous radio speech. Existing Flight integration and the normal combat tick remain active.

Aircraft use reusable bounded slots, but naval weapons target stable IDs. A slot cannot be recycled while a live player missile still refers to it. Player missiles reuse inactive slots during endless play. Aircraft and missile target types are distinct. The combined player/naval airborne weapon limit is 96; the aircraft limit is 12. A pending raid waits for four available slots. Burst groups, impacts, tracks, visual meshes, speech requests, and speech cache are bounded.

Radar scans run at 1 Hz; engagement decisions at 4 Hz; damage control at 10 Hz; weapons at the current bounded simulation step. Low-cost distance/horizon checks precede terrain and mesh queries. Contacts expire. UI text refreshes at 10 Hz; decorative meshes/materials are shared and initialized once. Speech generation and playback occur on a worker with two pending requests, latest-report cancellation and a 16-file temporary cache; it never waits in the simulation. Speech uses macOS `say`, PCM telephone-band coloration and `afplay`. Mute/zero master level suppress speech; missing voice tools leave text usable. No network speech service is used.

Damage displacement, list, trim, angular velocity, reduced forward speed, and sinking are published through the existing Flight carrier state. Rendering, aircraft/weapon hull sweeps, deck support, catapult and hook loads use the same damaged ship frame. Nominal straight-course helpers remain for existing scenarios and legacy callers.

## Estimated initial parameter register

All numbers below are **engineering/gameplay estimates**, unless identified as existing project settings. They are tunable and do not establish real weapon capability.

| Parameter | Initial value / units | Basis and limits |
|---|---|---|
| Fleet speed / escort spacing | 12.86 m/s / ±500 m | Existing project formation |
| Initial raid | 2 fighters + 2 bombers, ~42–46.5 km from fleet | Game pacing; seeded heading/formation variation |
| Raid interval | Easy 300 s; Normal 240 s; Hard 180 s | Simulation time; defers at aircraft limit |
| Radar nominal range scale | 140 km × signature^(1/4) × authority^(1/2) | Estimated detection gate; geometric horizon and terrain/ship occlusion also required |
| Radar signature proxies | Fighter 3, bomber 12, missile 0.1 | Relative sensor proxies, not measured RCS validation |
| Track confirmation / classification / coast | 2 scans / 4 scans / 8 s | Game estimate; missile support tracks coast at most 3 s |
| Destroyer magazines | 24 Standard-class + 32 ESSM-class rounds | Representative finite mission allocation, not real loadout disclosure |
| Carrier magazine | 32 short-range ESSM-class rounds | Representative self-defense allocation |
| CIWS ammunition | 3,100 rounds per simulated mount | Mission supply estimate |
| Aircraft engagement boundary | 22 km | Preserve a useful player interception role; missile threats take priority |
| Standard-class maximum launch range | 55 km | Estimated game limit; scan, clearance and ammunition gates still apply |
| ESSM-class maximum launch range | 18 km | Estimated game limit |
| Standard-class propulsion / turn | 1,050 m/s speed target, 110 m/s² powered acceleration, 14 s burn, 30 g maneuver cap | Reduced point-mass estimate; drag and radial gravity integrated |
| ESSM-class propulsion / turn | 950 m/s, 160 m/s², 9 s burn, 40 g | Same model, estimated parameters |
| Anti-ship missile | 290 m/s, 45 m/s², 6 g, 180 s life | Representative subsonic threat; terrain-aware low-altitude guidance and physical hull collision |
| Anti-ship cruise / terminal altitude | 18 m / 5 m above supporting terrain/sea | Estimated; not the ocean's animated crest height |
| Fighter stores | 1 anti-ship + 4 radar air-to-air rounds | Mission loadout estimate; bombers have 2 anti-ship rounds |
| CIWS | 1,100 m/s projectile speed, 2 rad/s traverse/elevation, 30-round burst each 0.5 s | Bounded ballistic groups, gravity/intercept lead, estimated 2.5 mrad burst footprint; no hits from cosmetic tracers |
| CIWS engagement range | Missiles 2.5 km; aircraft 1.8 km | Terrain, ship obstruction, depression/elevation and slew limits apply |
| Ship damage compartments | 8 longitudinal/port-starboard compartments | Reduced-order fire, flooding, power/radar/launcher/propulsion and magazine model |
| Damage control | Automatic suppression/dewatering, power-dependent | No player damage-control station; stiffness/buoyancy parameters are estimates |
| Ship mounts | Ship-local positions in weapons module, metres | Heights checked against exported mesh collision skin; representative added mounts, not a certified real-ship installation |

Missiles have bounded acceleration, powered/coasting flight, gravity compensation through maneuver authority, lifetime, support loss, swept impact/proximity checks and finite magazines. This is not a full six-DOF missile/seeker database. CIWS burst footprints approximate multiple projectiles rather than spawning every round. Ship compartments approximate progressive damage; flooding is not a CFD solver. Bomber AI currently uses the existing generic aircraft physics plant and a distinct original representative bomber silhouette, without claiming a named real bomber's flight model.

## Sources and asset provenance

- U.S. Navy, [Aegis Weapon System](https://www.navy.mil/Resources/Fact-Files/Display-FactFiles/Article/2166739/aegis-weapon-system/): integrated ship sensors, command/weapon control and Mk 41/Standard Missile roles.
- U.S. Navy, [Standard Missile](https://www.navy.mil/Resources/Fact-Files/Display-FactFiles/Article/2169011/standard-missile/): representative fleet air-defense role and launch-system association.
- NAVSEA, [Close-in Weapons Systems](https://www.navsea.navy.mil/Media/News/Article-View/Article/4313344/close-in-weapons-systems-the-last-line-of-defense/): terminal protective role and coordination with Aegis.
- Existing ship asset attribution remains in `assets/ships/burke/README.md` and `assets/ships/nimitz/`. Original authors are not endorsing this simulation.
- Free-model search found generic fighters, transports, low-detail/stealth bombers, and catalog assets unsuitable for the chosen representative twin-engine maritime bomber. No additional third-party aircraft asset was imported. Bomber geometry is original procedural project geometry; fighters reuse the existing Su-27 loft/wing/fin geometry. Existing ship licenses remain unchanged.

Validation and actual timing results are recorded separately in VALIDATION.md. Automated checks and scripted screenshots do not substitute for the user's flight review.
