# Missile guidance and seekers

The tables and close-range acquisition rules below describe the original IR and
radar weapons. Separate BVR scenarios add an LR-AAM profile using this same GNC
implementation, with a longer boost/sustain motor, lofting and active acquisition
near 18 km. BVR Search/TWS/STT tracking and support rules are described in
[BVR operations](BVR.md); selecting another target in TWS does not by itself
remove support from maintained tracks.

Both player and enemy weapons use `src/combat/guidance.rs`. This is a demanding, tunable game simulation. It does not reproduce classified seeker algorithms or calibrated AIM-9/AIM-120 performance. The infrared weapon now supports all-aspect acquisition. The radar weapon uses aircraft updates followed by autonomous active terminal homing, consistent with the public [USAF AMRAAM description](https://www.af.mil/About-Us/Fact-Sheets/Display/Article/104576/aim-120-amraam/) and [USAF description of midcourse datalink updates](https://www.af.mil/News/Article-Display/Article/467412/f-35a-conducts-first-live-fire-with-amraam/).

## Target acquisition and navigation

The launch aircraft requires target range, nose angle, signal strength, terrain visibility and lock dwell. Weak returns acquire more slowly. The player radar must remain powered, healthy and pointed within 55 degrees at the selected target to supply midcourse updates. Selecting another target removes that missile's support. An enemy launcher supplies the same geometric support until it turns away, is destroyed or loses sufficient health. Missiles activate their own radar when the **predicted** target range falls below 6.5 km, after the separation interval. They can still acquire autonomously following a datalink loss if their prediction puts the target inside the seeker's view.

Each missile carries an estimated target position and velocity. An alpha-beta filter updates these from available sensor measurements. During track loss, the navigation state propagates its last velocity; it does not read the target's new position or velocity. Infrared confidence decays over roughly one second and radar confidence over roughly two seconds. Low confidence disables guidance; a subsequent valid return can rebuild the track. Seeker pointing uses the predicted target position rather than invisible target truth. Gimbals have finite travel and slew rate:

| Parameter | Infrared | Active radar |
|---|---:|---:|
| Gimbal travel | 45 degrees | 55 degrees |
| Gimbal slew | 22 degrees/s | 40 degrees/s |
| Tracking view around gimbal axis | 6 degrees | 6 degrees |
| Minimum normalized signal | 0.18 | 0.18 |
| Confidence decay | 1/s | 0.5/s |

## Infrared signal and flares

After launch, IR missiles can fall back to radar datalink when their own seeker
cannot detect the aircraft because of glare, weak signal, seeker range or gimbal
limits. Infrared tracking is preferred and resumes automatically when a valid IR
measurement returns. The missile does not gain an onboard radar seeker.

Fallback uses the launching aircraft's existing radar-support availability,
20 km support range, 55-degree forward sector, terrain masking and Doppler-notch
checks. In BVR, the launcher must maintain the appropriate radar track under the
existing TWS/STT rules. Radar power, launcher destruction/damage and loss of the
required target track can remove support. When neither IR nor radar is available,
the missile propagates its last estimate, loses confidence and eventually becomes
ballistic. It does not read unseen target motion. Existing IR launch acquisition
rules and missile propulsion/energy limits are unchanged.

Telemetry shows **IR / RADAR DATALINK** during fallback. An IR missile following a
flare does not receive radar updates for that decoy; existing probabilistic
diversion and reacquisition behavior remains in effect.

Aircraft heat depends on engine operation, spool, engine health and viewing aspect. All-aspect IR acquisition accepts head-on, beam and rear approaches, with no hard rear-aspect gate in player launch cues, AI weapon selection or missile seekers. The directional engine factor is `0.65 + 0.35 × max(rear cosine, 0)²`, so exhaust-facing targets remain brighter. Afterburner adds a strong heat term above 80% spool. Engine-off aircraft retain a small residual signature. Received signal falls with squared range; weak signals, sun glare, terrain and seeker pointing limits still prevent acquisition. This is a gameplay signature model, not a calibrated spectral detector.

Sun interference applies when the sun is above the local horizon. A target within 3 degrees of the sun is masked; the glare factor recovers smoothly between 3 and 12 degrees. This is a normalized radiometric proxy rather than a spectral detector simulation. The [CSIR infrared simulation study](https://researchspace.csir.co.za/bitstreams/76b0e69c-b04f-4e95-8268-e8649b7ebbc7/download) identifies aircraft/flare radiance, background, atmosphere and seeker dynamics as separate simulation concerns. The numerical thresholds here are game assumptions.

Flares eject, slow down, fall under gravity and cool exponentially. A burst needs at least 0.08 seconds to develop. Diversion depends on flare-to-aircraft radiance, distance, separation angle, seeker view and burst age. Afterburner increases aircraft radiance and reduces the flare's relative salience. There is a seeded, repeatable draw **once per eligible burst per missile**, rather than a fresh chance every physics frame. IR track continuity now reduces attraction for straight-flight targets; transverse motion increases the opportunity for a flare to pull the seeker away. The IR diversion probability is capped at 65%, with a lower straight-flight coefficient. Radar/chaff competition is unchanged.

Selecting a flare is temporary. After 0.75 seconds the IR seeker rechecks competition: it can return to a detectable aircraft still close to the flare's angular track and not moving across the LOS, or when aircraft radiance overtakes the cooling flare. Aircraft reacquisition requires sufficient signal and being inside the seeker field/gimbal limits; it does not bypass terrain, sun, aspect or pointing checks. A break turn can separate aircraft and flare tracks while forcing the missile to spend time and energy correcting. The incoming-IR warning remains active while a missile is still closing, even during temporary flare tracking. Flares alone are insufficient in the controlled rear-aspect diagnostic; this is not a universal hit-rate claim.

Enemy pilots also dispense finite, matching countermeasures when a player missile closes. Their bursts use the same sensor competition model. Difficulty changes the pilots' dispensing cadence, rather than making the player's flare effectiveness an arbitrary difficulty percentage.

## Radar notching and chaff

Radar return falls with fourth-power range and is reduced in a look-down clutter notch. The relevant velocity is the **target's ground-relative velocity projected onto the radar-to-target line of sight**, not missile-target closure speed. A target moving perpendicular to that line can overlap the ground-clutter Doppler return. Look-up geometry does not apply this clutter notch. Public background is described in [Radartutorial's airborne pulse-Doppler explanation](https://www.radartutorial.eu/20.airborne/en/ab10.en.html).

The model retains 4% of return below a 12 m/s radial speed, increasing to full return at 45 m/s. These widths and floor are assumptions. Notching can break the launch aircraft's updates or the missile's active track, depending on which radar observes it. Track memory can bridge a short notch. Chaff competes with an active radar seeker; its return decays and its velocity slows rapidly. Chaff near the ground-clutter velocity can itself be filtered out. Chaff does not distract infrared seekers. Terrain LOS checks mask both sensor types.

## Guidance, controller and energy

Long-range radar missiles now use a smooth midcourse loft instead of the previous
small altitude bias with an abrupt 25 km cutoff. The path requests thinner-air
cruise, then descends toward the intercept. Requested altitude rises with remaining
estimated range, up to 14 km above the estimated target altitude and a 26 km
ceiling. A bounded flight-path-angle demand and vertical-speed feedback command
the actual fin controller; altitude is never assigned directly. Horizontal
intercept guidance continues during the climb. Between 40 and 18 km the vertical
path command smoothly gives way to ordinary terminal proportional navigation.
Short-range radar and IR missiles retain their existing direct guidance.

The loft uses the missile's filtered navigation state, including during track
memory. It cannot read an unseen target's new altitude. Decoy tracking uses direct
guidance. Gravity compensation, actuator lag, density-dependent aerodynamic
authority, propulsion and induced drag still apply. This is a game trajectory
heuristic, not a globally optimal trajectory solver or a real missile performance
model. Low-altitude, long-distance tail chases can still exhaust their energy.
The missile-camera telemetry shows the path altitude request and loft blend.

The guidance law uses filtered line-of-sight rate and closing velocity:

`a_command = N * closing_speed * (LOS_rate cross velocity_direction)`, with `N = 3.8`.

Gravity compensation and a short launch capture term are added before limiting the command to available aerodynamic authority. Public proportional-navigation background is provided in [NASA's intercept guidance report](https://ntrs.nasa.gov/api/citations/20170001430/downloads/20170001430.pdf).

The missile then runs an acceleration controller with feed-forward, proportional/integral feedback, integral anti-windup, finite fin travel, 4 normalized travel units/s slew and a 65 ms actuator lag. Available acceleration depends on dynamic pressure, mass and normal-force coefficient, with a 28 g infrared / 24 g radar structural cap. High altitude and low speed can leave the controller unable to produce its requested turn. Achieved acceleration feeds back into the controller. Body-rate and nose alignment lag drive the rendered missile attitude and thrust direction.

This is a reduced-order three-dimensional flight-control model. The fin vector represents combined pitch/yaw effort. It does **not** integrate independent roll attitude, aerodynamic hinge moments or a full rigid-body six-degree-of-freedom missile plant.

| Parameter | Infrared | Radar |
|---|---:|---:|
| Launch mass | 85 kg | 150 kg |
| Propellant | 20 kg | 35 kg |
| Thrust during burn | 14 kN | 23 kN |
| Motor duration | 4.5 s | 6 s |
| Reference area | 0.020 m2 | 0.029 m2 |

There is a 0.18 second separation delay before ignition. Propellant consumption decreases mass throughout the burn. Drag includes base drag, a transonic increment and a lift-induced term proportional to squared normal-force coefficient. Turns therefore consume energy; guidance cannot produce free lateral acceleration indefinitely. After burnout, coasting speed falls under drag while gravity exchanges kinetic and potential energy. Specific mechanical energy is recorded as `v^2/2 - mu/r`. Proximity fuses, finite lifetime and terrain impact remain in the combat layer.

## Diagnostics

Press **6** to cycle airborne missiles and inspect seeker mode, signal, speed, achieved/available g, remaining propellant and track-loss reason in the separate missile telemetry panel. **7** restores the previous aircraft camera. **V** dispenses a paired flare/chaff burst and **Backspace** dispenses chaff only. These effects apply to both sides.

`--study-missiles` records dry/afterburner/sun and supported/unsupported/notching radar engagements to `tests/missile-gnc-study.json`. `--study-air-combat` uses the real player aircraft and all opponents to check the integrated combat/damage path. Unit tests cover sensor geometry, memory without target-truth leakage, handoff, non-perfect countermeasures, actuator rates, altitude-dependent authority, propellant exhaustion and turning/coast energy. These checks establish numerical behavior under the documented assumptions, not real weapon fidelity.

The missile study also records 24 matched trajectory trials: previous versus new
guidance at 40/70/100 km, 3/11 km launch altitude, and approaching/retreating
280 m/s targets. Both profiles use the same 320 m/s launch, motor, atmosphere,
controller and continuously available radar support. In the 100 km head-on case
at 11 km, the new trajectory peaks at 23.2 km and intercepts at 1,344 m/s versus
974 m/s previously, with integrated drag energy loss falling from 1.402 to
0.981 MJ/kg. At 3 km the corresponding speeds are 635 versus 351 m/s. All ten
previously successful geometries still intercept; the two low-altitude long
tail chases still miss, though their closest approaches improve. These are
controlled numerical comparisons, not guarantees against maneuvering targets.

The missile study also runs 100 seeded trials each for straight/no flare, straight/three flares, a 3 g break alone, and three flares plus that break. The diagnostic starts with a rear-aspect IR missile 1.8 km away at 400 m/s, a 230 m/s aircraft at 3 km altitude and 65% throttle, a one-second defensive delay, and 0.65 seconds between bursts. Current hits are 100/100, 100/100, 100/100 and 25/100 respectively. Straight-flight flares temporarily divert 89 missiles, all of which return to hit. These are conditional results for this geometry, not universal probabilities or real seeker performance. A regression compares combined defense at 120 Hz and 240 Hz.
