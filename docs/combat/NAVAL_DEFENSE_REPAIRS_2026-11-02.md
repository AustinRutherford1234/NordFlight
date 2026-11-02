# Burke launch repair, sea-skimming cruise and CIWS presentation

## Burke SAMs

Both installed Burke launch locations are at ship-local (0, 7.8, -40) m. A diagnostic target at approximately 14 km range and 500 m altitude was detected by both destroyers, but both rejected engagement because a direct hatch-to-target ray intersected their own superstructure. The upward hatch-exit sweep was clear. The carrier's launcher passed the direct-ray check, which masked the escort failure in the previous fleet-level interception test.

VLS eligibility now checks a vertical exit to 60 m above the launch mount, followed by the outgoing clearance ray. It retains ship collision checks on both segments, including other friendly ships, and the ordinary missile solver remains active after launch. This is an estimated loft-clearance test rather than a trajectory planner. The missile still leaves the physical hatch upward and pitches toward the measured track through its finite-force guidance; it is not teleported to the clearance point. Existing cooldown, ammunition, commitment, reachability and radar requirements remain. Spent SAMs in ballistic flight no longer reserve the two-shot target commitment; the physical rounds continue until impact or age expiry, while the fleet may re-engage. A regression verifies that two still-alive ballistic rounds do not prevent a fresh shot.

The new regression disables the carrier's weapons and the other escort, then verifies launch and interception separately for each Burke. It explicitly reproduces the old obstructed direct ray and confirms the corrected vertical route. An additional native preset shows each escort firing independently.

## Cruise altitude channel

Previously the cruise weapon aimed at a distant point whose altitude was reduced to 18 / 5 m. That does not command an early descent or local radar-altimeter altitude hold: a missile released at 10 km can remain high for most of its route.

The new force-command altitude channel uses the missile's current local height and vertical speed, with a 1.5-second terrain/water look-ahead. A bounded descent-rate command begins immediately after high-altitude release. It settles onto a 12 m clearance in cruise, smoothly reducing toward 5 m between 8 and 3 km of estimated target range. Over modeled sea, the clearance includes the current/look-ahead wave height; elsewhere the channel follows the existing terrain height. The modeled sea region is unchanged. Descent speed is bounded at 60 m/s, climb at 25 m/s, and vertical acceleration request at -12 to +25 m/s², within the existing 6-g total maneuver limit. The controller compensates gravity and planetary curvature, without snapping altitude or velocity.

Lateral navigation continues toward the inertial/seeker track. No new live target truth is passed into the controller. Propulsion, drag, terminal seeker requirements, finite endurance, proximity checks and ship collision remain enabled. Five-minute air-to-air/SAM cleanup and longer cruise lifetime are unchanged.

Public evidence: [NAVAIR's Harpoon description](https://www.navair.navy.mil/harpoon) describes low-level sea-skimming cruise and terminal options. It does not establish the game's clearance, gains or descent schedule. All numerical altitude-control settings here are engineering/gameplay estimates, not a claim to model a specific operational missile block.

## Phalanx-style CIWS

[Raytheon describes Phalanx](https://www.rtx.com/en/raytheon/what-we-do/sea/phalanx-close-in-weapon-system) as a radar-guided rapid-fire system with 3,000–4,500 rounds/minute. [The Royal Australian Navy lists 4,500 rounds/minute](https://www.navy.gov.au/capabilities/weapons/phalanx). This representative later-model installation uses **4,500 rounds/minute = 75 rounds/second**.

The previous model emitted 30-round packets every .5 s: 60 rounds/second with widely separated visible traces. It now emits three-round ballistic packets every .04 s, advancing the schedule without accumulating render-frame rounding. A two-second cadence fixture at 60 / 120 / 240 Hz requires 150 rounds consumed at each rate. Packet capacity is preallocated and bounded at 256; packets still expire after three seconds or collision. All three mounts firing continuously need at most approximately 225 active packets. The existing per-ship aggregate ammunition allocation is unchanged; it is not represented as a sourced magazine capacity.

Each packet still travels ballistically, including gravity, moving-ship muzzle velocity, swept ship obstruction and swept target intersection. This is a burst-group model, not independent six-degree-of-freedom bullets. Visual tracers show three spaced round streaks per packet; they do not add damage without the packet's computed intersection.

Presentation changes include a six-barrel cluster spinning at the equivalent 12.5 revolutions/second, retaining bands, side mounting detail, pulsing emissive muzzle flash, and warm tracer bodies with brighter short cores. Meshes/materials are shared and flashes are preallocated; no point lights, per-shot entity spawning or new particle simulation is added. These choices retain a bounded rendering workload in the previously investigated missile-heavy scene. No FPS improvement is claimed from source inspection.

## Verification and limits

The ordinary regression suite retains radar support loss, guidance, finite motor energy, ship damage, other aircraft, carrier operations and reset checks. New targeted tests cover isolated Burke launches/intercepts, CIWS cadence across frame partitions, and a high-altitude release that descends to sea-skimming height at 60 / 120 / 240 Hz. The existing 149 km cruise strike test remains active to catch range/endurance regressions.

The approved native review captures port and starboard Burke launches, the low-altitude cruise state and the close CIWS view. These scripted snapshots are separate from live manual combat and sustained performance testing. The harness pauses and resumes prior running sessions and is bounded to three minutes.


## Recorded automated results

The full regression run passed **791 tests** with zero failures and one opt-in diagnostic ignored (31.98 s). After the spent-round commitment repair, the focused naval suite passed **48 tests**, zero failures and one ignored diagnostic (6.32 s). The final full-suite run passed **792 tests**, zero failures and one ignored timing diagnostic, in **39.06 s**.

The sea-skimming altitude-channel fixture recorded approximately 12.412–12.476 m over the final 30 s of its four-minute high-release trial, at each of 60 / 120 / 240 Hz. It holds tangential speed only in this isolated controller fixture; the integrated long-range strike uses normal propulsion and drag. The integrated 149 km release struck the moving carrier at 540.33 s, with missile height approximately 5.54 m and carrier hull reduced to .901. These are simulation regression points, not real-missile performance measurements.

Native diagnostics printed one launched round and 55 remaining SAMs for each isolated Burke, approximately 12.525 m height and 290.16 m/s for the low-level cruise snapshot, and nine three-round CIWS packets with 3,073 of the initial 3,100 aggregate rounds remaining. The corrected muzzle-flash effect was recaptured after resolving a collision with the existing missile-exhaust visual tag. Tracer streak length was shortened to leave visible gaps rather than form a continuous laser-like line.


## Native review and delivery

The approved native checks consumed **97.44 seconds** in total, including CIWS recaptures. The restoration handler resumed the prior user session after each invocation. Both Burke launch views, the sea-skimming view and the final close CIWS view were visually inspected. The final image shows separate tracer segments, the muzzle flash at the barrel cluster, and the added mount geometry. Static captures verify geometry/presentation at selected simulation states; the rotor animation code follows simulation time, but these still images alone do not constitute a continuous-motion or user flight review.

The final `Nord Flight.app` was rebuilt with `./build-mac.sh dev` and passed `codesign --verify --deep --strict`. Executable SHA-256: `5f95d56e37267d58097fe7644f718cb22864bf1b68ba42c738589cec1d4bde98`. The running user session was preserved and still uses its original executable; quit and reopen the app to load this build. This milestone did not include a new rendering benchmark or claim to resolve the previously documented system-memory stalls.
