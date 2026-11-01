# Su-27 / Tu-160 flight and mission energy update

## Scope and evidence

This adds a dedicated Su-27 aerodynamic/propulsion/FBW parameter module to the existing six-degree-of-freedom aircraft integrator, rather than a second kinematic AI simulator. The Tu-160 retains its dedicated rigid-body plant, with revised transonic drag, inlet/ram effects, payload/bay drag and afterburner availability. Both are engineering approximations, not validated reproductions of proprietary aerodynamic databases or flight-control software.

Public envelope anchors: [US government Su-27 aircraft sheet](https://www.govinfo.gov/content/pkg/GOVPUB-D201-PURL-LPS106117/pdf/GOVPUB-D201-PURL-LPS106117.pdf) describes Mach 2+ capability. [Rostec's Tu-160 article](https://www.rostec.ru/media/news/tu-160-belyy-lebed-obnovlyaetsya/) describes maximum speed above 2,000 km/h and payload above 40 tonnes. These broad specifications do not mean either aircraft can hold maximum speed at arbitrary altitude, weight, bank or configuration. Numerical polar, inlet, control, flight-phase and release parameters below are estimates. The requested 24 × 1,500 kg bomber stores are a gameplay loadout, not a sourced operational loadout.

## Physical/control architecture

`src/aircraft/su27.rs` supplies a Su-27-specific polar correction to the existing unsteady separated/vortex flow evaluator. Induced drag uses span 14.7 m, area 62 m² and estimated efficiency 0.79. The old induced and wave terms are removed before adding replacement terms. The wave peak is a smooth Gaussian at Mach 1.07 (amplitude 0.024, width 0.18), transitioning to estimated supersonic wave drag 0.018. Existing aircraft-specific surface geometry, panel-local aerodynamic loads, mass/inertia, control allocation, actuators, damage, hydraulic authority, collision and full rotational coupling continue to apply. Existing low-speed static and post-stall pitch derivatives remain in the integrator. Su-27 control allocation probes aerodynamic derivatives around the actual surface state rather than assuming neutral-surface derivatives remain valid at large deflections.

The twin AL-31-class installed engine estimate uses 149 kN total dry and 245 kN total wet reference thrust. Density lapse exponent is 0.62; ram/inlet recovery varies smoothly with Mach. Spool, fuel flow and separate existing engine health/on-off states remain part of the actual propulsion integration. Both pilot throttle inversion and actual installed thrust call the aircraft-specific engine function. The AI no longer estimates every aircraft's required throttle using a Gripen engine and wing area.

Normal Su-27 FBW requests bounded body rates from estimated load-factor, AoA and roll-rate laws. Positive/negative targets are +9/-3 g, with AoA scheduling from 0.46 toward 0.34 rad as Mach increases. Neutral stabilization, coordinated-turn terms, sideslip damping and reduced high-AoA roll authority feed the existing physical control allocator; no attitude, velocity or position assignment is used to implement FBW. Assist-off retains the existing raw surface-command path. Avionics, hydraulic and surface failures retain existing gates/limits. This is representative simulation behavior, not an operational Su-27 software replica.

All combat AI airframes now select the Su-27 plant, consistent with the imported fighter exterior. Bomber initialization explicitly switches to its separate Tu-160 plant.

## Bomber / mission changes

The bomber throttle limit is now 1.0, allowing the full existing afterburner range. The mission pilot inverts the same piecewise engine curve used by the plant, requesting thrust from actual aerodynamic drag plus acceleration and climb demand, instead of retaining a large proportional speed error. The previous unbounded quadratic Mach drag rise is replaced with a smooth transonic peak, bounded supersonic contribution, high-Mach drag growth, per-store drag and open-bay drag. The ram/lapse curve now includes a bounded Mach-dependent recovery rather than a monotonic thrust reduction. Engine shutdown, fuel depletion, structural/control damage and physical bank/load limits remain.

Naval pilots request Mach 1.45 bomber transit / Mach 1.6 withdrawal, Mach 1.5 fighter escort / Mach 1.65 intercept. Speed requests are constrained by estimated dynamic-pressure ceilings (45 kPa bomber, 65 kPa fighter). Requests do not override the achievable thrust/drag balance. Raids enter at 120 km from the carrier, at 10 km altitude for bombers and 11 km for fighters, with a 300 m/s initial flight state. Strike altitude requests are 10 km during transit and 8 km near the target. These are scenario initial conditions, not boosts applied during flight. Bomber release preparation begins within 40 km and commands Mach 0.9; bay opening and missile release require Mach below 0.95. Once release starts, the bomber retains the subsonic attack configuration until stores are expended. Straight strike corridors and turn-around gates 35 km outside the fleet replace continuous pursuit of the ship centre, preventing a large bomber from becoming trapped in a circle with its nose never aligned for release. The 24-missile inventory, five-second release cadence, physical flight path and next-wave rules remain. Bay motion contributes drag through actual deployment.

The Tu-160 mesh retains its fixed intermediate wing sweep; this milestone does not add sweep animation or claim a validated variable-sweep aerodynamic database. Maximum-speed clean/light certification is outside the current calibration evidence.

## Validation record

Full regression suite: 741 passed, 0 failed, 54.69 s test execution. Includes Su-27 normal turn and stalled-start recovery, raw cobra recovery, engine shutdown/asymmetry, nominal 60/120/240 Hz convergence, transonic polar continuity/nonnegative drag, bomber bank/load and fuel behavior, 24-store release/withdrawal through a 1,400-second physical raid, and existing Hornet/carrier/weapon/collision/reset regressions. The extended raid test permits normal off-scene withdrawal despawning and still rejects physical crashes.

Initial 300-second high-altitude acceleration checks reached Mach 1.74 Su-27 and Mach 1.46 Tu-160 with all 24 bomber stores. This is achieved simulation behavior, not a reference-matched top-speed validation. The final native fixture adds altitude feedback and reports its separate measured results below.


### Final native review

The approved native review completed in 124.4 seconds aggregate, below the 180-second limit, with the user's session restored. Final 300-second altitude-feedback acceleration fixtures achieved:

| Aircraft/configuration | Speed | Mach | Altitude | Final normal load |
| --- | --- | --- | --- | --- |
| Tu-160 with 24 stores | 480.2 m/s | 1.627 | 11,107 m | 0.994 g |
| Su-27 fighter | 505.5 m/s | 1.713 | 12,336 m | 0.992 g |

The fighter finishes roughly 1.3 km above the fixture's 11 km altitude request; this is not a precision altitude-hold validation. The mission pilot remains a reduced-order approximation. Full-raid samples separately show approximately Mach 1.45 bomber transit and Mach 1.50 escort transit, followed by subsonic bomber release passes. Raw screenshot FPS values are not controlled performance measurements and no rendering-speed gain is claimed.

Inspected final bomber/fighter acceleration views and short live fighter/bomber views in the running app. Captures are under `tests/raid-flight-review/`. Complete 24-store mission release/withdrawal and 60/120/240 Hz checks are automated evidence; the brief app views do not replace a user-flown mission review. `./build-mac.sh dev` completed and strict/deep ad-hoc signature verification passed. All aerodynamic derivatives, control gains, inlet/ram curves and mission configuration limits are estimates and still need broader envelope calibration.
