# F/A-18F HUD

Implemented 2026-10-28. This replaces the Hornet's shared cockpit combiner rendering with an original Super Hornet-style display. Other aircraft retain their existing displays. It is a representative simulation display, not a reproduction of a specific classified/proprietary operational flight program.

## Operation

Use the cockpit camera (C). HUD functions also have labelled clickable cockpit controls. The existing weapon-selection, firing, radar and countermeasure controls remain in effect.

| Input | Function |
| --- | --- |
| F7 | Cycle NAV → A/A → A/G |
| Alt+F7 | Cycle full / REJ 1 / REJ 2 declutter |
| Alt+End | Cage / uncage velocity marker |
| Alt+Delete | Select barometric / radar altitude, with automatic B fallback when radar altitude is invalid |
| Alt+Page Up / Page Down | HUD brightness |
| Cockpit HUD MODE | Cycle modes |
| Cockpit HUD REJ / CAGE / B/R ALT | Corresponding display controls |

HUD speed is calibrated airspeed in **knots**, altitude in **feet**, vertical velocity in **feet/minute**, distance in **nautical miles**, closure in **knots**, and fuel in **pounds**. Heading is explicitly true (`T`); there is no magnetic-field model. The rest of the application's generic telemetry retains its existing units.

## Display and data

Common symbols include a wrapping heading tape, boxed speed and altitude, altitude-source flag, world-attitude pitch ladder with dashed negative rungs, aircraft waterline, velocity vector, energy caret, Mach, normal acceleration, incidence, vertical speed, terrain/deck clearance, fuel and display mode. The limited velocity marker is crossed and flashes. Caging keeps its lateral location at the centre and adds a ghost at the actual location where visible. The NAV bank pointer flashes when its indicated range is exceeded.

CAS comes from ideal-gas pitot/static pressure, using the isentropic pressure ratio below Mach 1 and Rayleigh pitot relation above it, inverted against sea-level standard conditions. It is not a density-scaled TAS shortcut. Barometric altitude uses the existing simulation MSL altitude; adjustable QNH and independent baro-sensor errors are not implemented.

NAV points to the home runway or the moving carrier landing lane and shows bearing, range, heading cue and an in-view destination diamond. This is a default steer-to point, not a newly implemented INS, TACAN or route editor. Gear-down airborne symbology includes an incidence E bracket and FAST / ON SPD / SLOW indication. These cues display information; they do not command the FBW, flight path, flare or throttle.

A/A shows selected missile family, remaining ammunition, acquisition field, target designation, line-of-sight direction, range, closure, acquisition/lock state, readiness and the existing gun lead solution when available. The displayed LOCK/RDY requires the combat system's cue check and lock timer. It is not a new radar or a simulated missile probability-of-kill/no-escape-zone calculation.

A/G supplies **gun CCIP**, using the current game's actual muzzle velocity, inherited aircraft velocity, radial gravity, terrain intersection and 2.5-second projectile lifetime. The numerical intersection is refined within the crossing step. It shows a pipper, connecting fall line, slant range and time of flight, or a limited/no-solution flag. The current cannon uses 4250 m/s: this existing gameplay value has deliberately not been relabelled as real M61 ballistics. Bombs, rockets, guided air-to-surface weapons, automatic release and bomb CCRP require their corresponding physical weapon implementations. This display does not invent those solutions.

HUD mode selection changes presentation; it does not change the existing game's firing authorization. ARM/SAFE reflects availability of the current combat scenario rather than a new clickable master-arm circuit. The aircraft flight model and FBW are unchanged by this work.

Battery loss, very low avionics health, and a destroyed aircraft blank the display. Radar-altitude validity uses estimated 5000-foot, 60-degree bank and 45-degree pitch bounds; the clearance samples the actual local terrain or carrier deck. Fuel, gear and carrier-hook cautions are based on current aircraft state. Display brightness, placement, scaling, declutter levels and alert thresholds are game-specific estimates.

## Rendering

The Hornet combiner has a 1024 × 768 RGBA texture, transparent background, smoothed original strokes, emission and adjustable alpha brightness. The image is updated at up to 30 Hz in cockpit view through the existing bounded texture-upload path. Empty-border cropping preserves plane position and UV alignment at both the original and new resolutions. It remains attached to the physical cockpit and disappears in external/map/follow cameras. At the existing design eyepoint, attitude, velocity and targeting directions use the actual combiner-plane ray intersection. Optical collimation for arbitrary translated head positions is not simulated.

Validation results and native capture instructions are in [HUD_VALIDATION.md](HUD_VALIDATION.md).

## References and distinction from validation

The developer-authored [VRS Superbug navigation documentation](https://forums-new.vrsimulations.com/support/index.php/Navigation_Systems) and [approach tutorial](https://forums.vrsimulations.com/support/index.php/Navigation_Tutorial_Flight) provide publicly accessible F/A-18E simulation references for visual conventions. These informed the three modes, heading/velocity/attitude presentation, caging, declutter, altitude fallback and the 8.1-degree approach cue. They are simulation documentation, not Navy certification data or an F/A-18F software specification. No VRS assets or code were copied.

[NASA's isentropic-flow equations](https://www.grc.nasa.gov/www/k-12/airplane/isentrop.html) support the subsonic pressure relationship. [Just Flight's Super Hornet manual](https://downloads.justflight.com/products/JFF003245/FA-18%20Super%20Hornet%20Manual.pdf) is another public visual reference. Exact symbology placement, display timings, readiness text and game navigation/weapon bindings are our implementation choices. Automated checks establish simulation consistency, not real-aircraft avionics fidelity.
