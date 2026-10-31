# Tactical map navigation and naval weapon tracking

Date: 2026-10-31

## Controls and presentation

With the aircraft tactical map open, W/up moves the view north, S/down south, A/left west, and D/right east. Panning uses real elapsed time, works while paused, scales with the map span (0.4 spans per second), and normalizes diagonal input. Opposite directions cancel. Flight pitch/yaw/roll input is neutral while this map is open; closing it restores normal key behavior. Right-drag and wheel zoom remain available. The on-map instructions include both keyboard sets.

Already-visible friendly and hostile weapon contacts have exactly two label lines: missile name and actual guidance state (datalink, active radar, IR, inertial navigation, memory, no track, separation or decoy tracking). The state line is mixed 12% toward white from the name's friendly/hostile color, preserving affiliation while adding a subtle distinction. Missile labels omit IDs, range/altitude and diagnostic clutter. Detailed guidance reasons, confidence/signal and flight telemetry remain in the existing missile-camera interface and internal state. Revealing an enemy weapon's mode is an intentional gameplay inspection aid; it does not reveal previously undetected weapon positions. Full-map labels reserve enough height for both lines. The underlying tactical picture remains refreshed at the existing bounded rate.

## Naval physics changes

The old naval solver steered toward live target position even after losing support, and the rendering adapter labeled all weapons as active radar. The replacement evaluates sensors separately and feeds measured or predicted navigation states into the existing acceleration/turn-limited weapon solver:

- Fresh, confirmed ship radar reports drive midcourse datalink updates. Radar/power loss and aged reports remove those updates. Enemy fighter updates use its recent player-contact memory.
- Standard-class and ESSM-class simulation weapons switch to estimated active terminal seekers. Range, forward field of regard, finite gimbal slew, received return and terrain/Earth visibility gate acquisition. Air-target radar processing retains Doppler notch behavior; maritime seekers use grazing-angle/sea-clutter attenuation so slow or stationary ships are not filtered out as airborne ground clutter. The same maritime return calculation is used for chaff competition.
- Anti-ship weapons follow a launch-time inertial solution and sea-skimming profile before terminal seeker activation. They do not follow unseen changes in ship position through a perfect target-information feed.
- A missed measurement extrapolates the last measured position and velocity. Confidence decays; expired memory removes steering. The motor, inertia, gravity, drag, physical contacts and lifetime continue normally. Reacquisition restores measured steering.
- Radar seekers evaluate geometric, age, signature and Doppler-dependent chaff competition through the existing deterministic countermeasure model. Capture changes the navigation estimate and flight path. A cloud losing visibility or expiring permits reacquisition; tracking a cloud does not directly damage the intended aircraft.
- A disappearing target no longer deletes a weapon immediately. Target truth is used for sensor evaluation and swept physical/proximity checks, not steering through a lost track.
- The same runtime navigation state drives map and missile-camera telemetry. Existing aircraft-missile tracking remains in use; the common mode enumeration adds INERTIAL NAV for naval cruise navigation.

## Parameter register: all naval values below are simulation estimates

These are representative gameplay/engineering values, not reproductions of specific Standard/ESSM/anti-ship seeker variants. They are deliberately separate from public descriptions of weapon roles. Existing propulsion/force limits and damage models are retained.

| Parameter | Value | Units / confidence |
| --- | --- | --- |
| Seeker measurement cadence | 20 | Hz; estimated |
| Terminal activation range: Standard / ESSM / anti-ship / AAM | 18 / 12 / 16 / 18 | km; estimated |
| Seeker maximum measurement range | 1.15 × terminal activation range | estimated |
| Forward field of regard | 60 | degrees half-angle; estimated |
| Acquisition cone around gimbal | 10 | degrees half-angle; estimated |
| Maximum gimbal slew | 80 | degrees/s; estimated |
| Maritime grazing/clutter return factor | 0.45 + 0.55 × clamp(abs(LOS dot local up) / sin(10 degrees), 0, 1) | estimated; no airborne Doppler notch |
| Measurement acceptance threshold | 0.18 | normalized return; existing generic model |
| Measurement position filter gain | 0.65 | per 20 Hz accepted update; estimated |
| Memory hard limit | 6 | seconds; estimated |
| Confidence decay after missed scan | 0.18 | per second; estimated |
| Ship/fighter supporting-report freshness | 3 | seconds; estimated |
| Ship launch vertical departure | 0.6 | seconds; retained |
| Raider countermeasure inventory | 12 flare/chaff pairs | per aircraft; estimated |
| Raider warning range and cooldown | 6.5 km naval / 5 km player missile; 6 s | estimated, requires visibility |
| Ship chaff inventory / warning range / cooldown | 16 bundles / 6 km / 8 s | estimated; requires a confirmed incoming missile track |
| Countermeasure decision cadence | 5 | Hz; estimated |
| Automatic naval countermeasure deployment ceiling | 128 | clouds; existing player dispensing remains separately limited by its finite inventory |
| Per-weapon evaluated-cloud history cap | 64 | IDs; no repeated chance rolls for the same cloud |

Supplies remain part of the mission state across player loss/respawn. Each new raider receives its own finite allocation. Clouds share the existing physical drag, gravity, lifetime and visual effects. Friendly chaff can affect any naval radar seeker for which the cloud has suitable geometry; ownership does not grant immunity.

## Public role references

The representative active-terminal SAM choice is consistent with the public [DOT&E SM-6 description](https://www.dote.osd.mil/Portals/97/pub/reports/FY2022/navy/2022sm-6.pdf), which distinguishes autonomous active terminal guidance from ship-supported illumination. [Raytheon describes ESSM Block 2](https://www.rtx.com/news/news-center/2025/10/01/rtxs-raytheon-delivers-500th-essm-block-2-to-u-s-navy) as having active and semi-active radar modes. These references support the choice of broad guidance roles; they do not source the activation ranges, sensor limits, countermeasure probabilities or other numerical parameters in this implementation. Semi-active illuminator scheduling is not added in this change.

## Limits

This is reduced-order tracking, not a proprietary electronic-warfare or seeker simulation. There is no new RF jammer, detailed communications waveform/interference model, infrared naval missile variant, or multi-object seeker imagery model. Countermeasure susceptibility is inherited from the existing seeded competition estimate. Naval body/turn physics remain the existing bounded model, rather than a newly validated six-degree-of-freedom missile aerodynamic database.

## Validation

- Final full regression run: `cargo test -- --test-threads=4`, 730 passed, 0 failed, 41.09 seconds test execution.
- New tests exercise both keyboard sets without cursor dependence, closed-map/editor input gating, aircraft-input isolation, concise labels for all eight guidance modes and sensor-based weapon visibility.
- Tracking tests cover support loss, prediction without following a changed hidden target, confidence expiration, reacquisition, terminal gimbal loss/recovery, anti-ship inertial navigation, seeded chaff capture/expiry/reacquisition, stationary-ship maritime returns, and target loss without deleting the flying weapon.
- Countermeasure tests cover finite raider supplies, cooldowns, confirmed ship-threat requirements and finite ship supplies. Existing moving-aircraft SAM intercept, anti-ship hull impact, CIWS, mission reset/respawn, Hornet flight/radar/HUD, carrier, collision and other-aircraft regressions also passed.
- Final native review: two actual-simulation snapshots at 5 and 15 simulation seconds, captured through `scripts/fa18f/capture_naval_upgrade.py --modes map-guidance map-guidance-hostile`. Inspected `tests/naval-upgrade/map-guidance.png` and `tests/naval-upgrade/map-guidance-hostile.png`. The later view contains 27 sensed/friendly contacts, with fleet datalink/active-radar states and hostile anti-ship active-radar states. Concise two-shade labels are readable without label overlap in these reviewed scenes.
- The final capture pair took 37.0 seconds. This follow-up used 74.8 seconds of native review overall; the shared approval-budget file totals 403.9 seconds including the earlier 329.1-second review, within the approved 600-second limit. The harness restored suspended flights. These are scripted snapshots after simulation warm-up, not a user-flown mission review or a frame-time benchmark.
- Rebuilt the existing Mac app with `./build-mac.sh dev`; strict/deep ad-hoc signature verification passed.
