# SR-71A Blackbird: static integration and flight-design report

Date: November 2, 2026

## Animation update

The static integration has been superseded by the articulated model and actuator pass. See [SR71_ANIMATION_PASS_2026-11-02.md](SR71_ANIMATION_PASS_2026-11-02.md) for mechanisms, standalone Blender review, validation and remaining limits. Historical static-preview descriptions below are retained as the original milestone record.

## Current asset update

The procedural first pass has been replaced by the user-supplied **manilov.ap Sr71** model under CC BY 4.0. The installed static GLB has 31,413 triangles, 24 embedded textures and no animations. It is uniformly scaled to 32.73 m long (17.03 m span), with game-forward alignment and tyre-bottom registration. Source and attribution are in `assets/aircraft/sr71/`; reproducible conversion is in `scripts/sr71/prepare.py`. Its fixed gear remains deployed. No dedicated physics or cockpit was added. The sections below retain the initial integration analysis and future flight-design plan; original procedural geometry described there is superseded by this imported asset.

## 1. Initial decision and scope

The first delivery adds an original, code-generated static SR-71A visual preview to the existing aircraft selector. It is not a calibrated Blackbird simulation. Select **F6 → SR-71A BLACKBIRD (STATIC PREVIEW)** in a non-carrier flight. The existing carrier restriction remains Hornet-only. Use external views to inspect this milestone; there is no bespoke SR-71 cockpit yet.

This is an unarmed reconnaissance aircraft. No gun or missile capacity is assigned. No SR-71 control surfaces, intake spikes, wheels, doors, nozzles or cockpit mechanisms are animated. The complete static assembly follows the aircraft's rigid-body pose. Fixed gear is deliberately visible even when the shared simulation gear command changes; this visual/physical mismatch is a known preview limitation, not acceptable for the later flight milestone.

Free Sketchfab candidates were found, including Tyler Nichols' CC Attribution model, but a downloadable file was not obtained in this session. No third-party geometry was copied. The original prototype therefore introduces no third-party model redistribution requirement. Replacing it with a verified downloadable, properly attributed model remains an option. The label “preview” should remain until visual review and flight acceptance are complete.

## 2. What is integrated

The existing aircraft enum gains an appended `Sr71` variant and `sr71` slug, preserving preceding variant order. The selector, parsing and saved aircraft path use the same existing aircraft list. The static renderer is isolated in `src/aircraft/visual/sr71.rs`, attached to the existing aircraft root and visibility/update path. It does not create a second simulation or per-frame geometry generator.

Geometry includes a tapered fuselage; broad nose chines blending into swept delta wings; two separated nacelles; dark intake annuli and pointed inlet cones; aft exhaust rings; inward-canted fins; two canopy sections; and fixed struts with paired nose and triple main tyres. Dark matte skin, subdued metallic rims and reflective canopy materials distinguish major surfaces. These are representative authored shapes, not a scan or station-by-station reconstruction. Close-up panel lines, markings, cockpit frames and exact wheel geometry need later reference matching.

The mesh is constructed once with 32-sided axial sections, 20×8 tyre tessellation and simple closed wing/fin plates. No high-resolution textures, runtime subdivision or SR-71 effect emitters are introduced. It uses existing immutable geometry handling. This bounds complexity by construction but does not establish an FPS result: performance must be measured before making that claim.

## 3. Evidence and uncertainty

[S1] NASA, *SR-71 Blackbird*, FS-2008-6-030-DFRC:
https://www.nasa.gov/wp-content/uploads/2021/09/495839main_FS-030_SR-71.pdf

[S2] National Museum of the USAF, *Lockheed SR-71A*:
https://www.nationalmuseum.af.mil/Visit/Museum-Exhibits/Fact-Sheets/Display/Article/198054/AFmuseum/lockheed-sr-71a/

[S3] NASA-TM-4791, Cox and Jackson, *Evaluation of High-Speed Civil Transport Handling Qualities Criteria with Supersonic Flight Data* (1997):
https://ntrs.nasa.gov/citations/19970014738

[S4] Smithsonian National Air and Space Museum, *Lockheed SR-71 Blackbird*:
https://airandspace.si.edu/collection-objects/lockheed-sr-71-blackbird/nasm_A19920072000

Asset-search lead, not installed: https://sketchfab.com/3d-models/sr-71-blackbird-40794825d2f64a6ca945990c8ff1e90f

S1 describes a 32.73 m aircraft with 16.94 m span, Mach 3.2 cruise design and flight up to 85,000 ft; it identifies elevons and all-moving fins. S2 lists two 32,500 lbf afterburning J58 engines. S4 supplies an independent museum reference for appearance and configuration. S3 evaluates high-speed handling using actual SR-71 flight data, making it a better control-validation starting point than fighter control gains.

**Source trap:** S1's printed pounds-to-kilograms conversions for gross/fuel weight are inconsistent. Convert original pounds independently and cross-check configurations before using mass data. Do not silently copy those kilogram values. Do not infer a complete thrust map or aerodynamic database from a single headline specification.

## 4. Current parameter register

| Parameter | Preview value | Basis / confidence |
|---|---:|---|
| Length | 32.73 m | S1, high |
| Span | 16.94 m | S1, high |
| Total static wet thrust | 289,134 N | Two × 32,500 lbf from S2, rounded conversion; not installed net thrust at altitude |
| Wing reference area | 167.2 m² | Provisional literature-style estimate; verify definition before calibration |
| Empty mass | 30,600 kg | Engineering placeholder pending configuration-specific evidence |
| Fuel capacity | 20,000 kg | Deliberately provisional gameplay allocation, not full historical capacity |
| Dry thrust | 180,000 N total | Uncalibrated estimate |
| Inertia diagonal | (1.8, 2.3, 0.45) million kg·m² | Body-axis engineering estimates; no validated mass distribution |
| Load limit | 2.5 g | Conservative preview setting, not a certified envelope |
| AoA parameter | 0.35 rad | Shared model tuning placeholder |
| Mach limit | 3.2 | Design target from S1, not demonstrated preview performance |
| Weapons | None | Reconnaissance configuration |

The shared engine lapse, aerodynamics, ground contact and assist remain provisional. Increasing the Mach limit alone does not make the aircraft sustain Mach 3.2. There is no dedicated intake, thermal, fuel-transfer, autopilot or stability-augmentation model in this delivery.

## 5. What should make it fun

The Blackbird should reward planning, acceleration and smooth handling. It should feel progressively more capable as it climbs into its intended regime, with speed producing a sense of scale and consequences. It should not behave like a large Hornet or become entertaining through arbitrary instability.

A suggested core sortie is: take off, climb through a guided acceleration corridor, reach a cruise gate, photograph reconnaissance targets, manage fuel and heat, then descend and recover. Optional research sorties can score precise speed/altitude tracking. A beginner start near the cruise regime avoids forcing a long ground departure on every attempt. Full departures remain available for players who enjoy procedure.

Keep goals readable: remaining fuel/time, predicted turn footprint, temperature margin and distance to the next gate. Reward smoothness and efficient routes rather than maximum instantaneous g. A photo pass should require framing and sufficient dwell, without secretly granting missile targeting capabilities. Avoid punitive random failures; every failure should have a visible cause and an achievable recovery path.

## 6. Dedicated rigid-body and aerodynamic model

Use the current flight integrator, SI units and aircraft-frame convention. Add a dedicated SR-71 evaluator, preferably under `src/aircraft/sr71/`, without altering Hornet or Flanker coefficients. Build mass and inertia from fuselage, wing, nacelles, crew and fuel groups. Update the centre of gravity with fuel burn/transfer; apply local aerodynamic and propulsion forces with their actual moment arms.

A first reduced-order model can use fuselage/chine, left/right delta, elevon and fin elements. At each element compute local air velocity including angular rates and wind. Evaluate lift, drag and sideforce as continuous functions of Mach, incidence, sideslip, Reynolds approximation and control deflection. Include trim drag, induced drag, transonic wave drag and compressibility effects. Decide explicitly which forces are aggregate and which are distributed to prevent double counting.

Write the balance as F = ΣF_aero + ΣF_prop + F_gravity + F_contact and M = Σ(r × F) + ΣM_section. Use the full rotational equation Iω̇ = M − ω × (Iω), with changing-mass treatment documented. Verify signs before tuning: a positive pitch command must produce the intended angular acceleration, and damping must oppose angular rate.

Above the calibration envelope, smoothly bound coefficients rather than clipping velocity or overwriting attitude. Do not transplant high-AoA fighter vortex/departure logic as verified Blackbird behavior. A conservative, recoverable estimated off-design response is preferable until supporting data is found.

## 7. Propulsion and inlet model

Treat each J58-class propulsion installation as engine, inlet, bypass and nozzle contributions. Use published performance curves where available; otherwise expose uncertain maps explicitly. The implementation needs spool lag, fuel flow, altitude/Mach dependence and net thrust after ram drag. Do not add pressure-recovery thrust to an already installed-net-thrust map.

Model inlet state with a stable reduced-order recovery variable, commanded spike position, actuator lag and a bounded distortion measure. Unstarts should require defined off-design conditions and produce physically applied asymmetric force and yaw moments. Provide clear symptoms and a recovery sequence. Do not make a random-number timer simulate intake physics. A beginner option may widen operating margins while retaining the same basic equations.

Build headless propulsion tests for shutdown, restart, afterburner transitions, asymmetric unstart and fuel exhaustion. A fixed spike mesh can remain in the static milestone; its later animation should read physical state rather than independently infer throttle.

## 8. Stability augmentation and keyboard handling

The eventual controller should be described as representative stability augmentation, not a claim that the real SR-71 uses modern fighter FBW. Separate pilot intent, controller, actuator and airframe. Keep a raw/direct test path. Start with rate damping and slow trim; add gain schedules against dynamic pressure, Mach and mass after trim solutions exist.

Keyboard input needs smooth command ramps and bounded release behavior. At high dynamic pressure, demand modest rates and accelerations; reduce control gain before actuator saturation causes oscillation. Filter sensor noise without hiding a destabilizing phase lag. Use anti-windup and bumpless transitions. Trim should change slowly enough to avoid fighting maneuvers but settle after release, with deadbands and limits documented.

Validate at 60/120/240 Hz with consistent controller scheduling. Reproduce high-speed reversals, abrupt release, long neutral cruise, asymmetric thrust and sensor/actuator failures. Do not tune only a single easy flight condition or use a hidden attitude snap to pass cruise tests.

## 9. Thermal and fuel management

Use a handful of thermal nodes: nose, chines/leading edges, nacelles, main skin and fuel. Track heat capacity and exchange with aerodynamic heating, internal sources, radiation and cooling. A useful starting estimate is recovery temperature T_r = T∞[1 + r(γ−1)M²/2], followed by finite heat-transfer dynamics; recovery temperature is not instant skin temperature. Calibrate coefficients instead of treating this estimate as a complete thermal solution.

Fuel quantity affects range and mass; distribution affects trim. Later transfer logic should preserve mass and energy accounting. Avoid rendering dramatic red-hot skin as the default cruise appearance. Introduce warnings and progressive consequences only after temperatures and exposure times are validated. Thermal simulation can run more slowly than rigid-body dynamics, with interpolated outputs.

## 10. Ground handling, cockpit and presentation

Before full flight acceptance, replace fixed gear with reference-based wheel contacts and matched geometry. Model braking, steering, tyre slip and suspension consistently. Do not permit carrier attachment or imply the SR-71 is carrier-qualified. Add an appropriate runway start and determine approach/rotation speeds from trim and contact tests rather than scaling fighter spawn speeds.

A later cockpit should prioritize readable Mach, indicated/equivalent airspeed, altitude, vertical speed, attitude, fuel, engines and navigation. Instruments and optional teaching overlays should explain margins without filling the view with fighter missile symbology. Distinguish authentic instrument references from gameplay aids. The rear crew station can first be visual, followed by optional navigation assistance.

Sound should communicate spool, afterburner and unstart state through the existing sound interfaces. External heat haze and restrained exhaust can follow later. Animation should be a separate milestone: elevons, all-moving fins, gear/doors, intake spikes and nozzles must share their transforms/state with physics and collision. Do not animate parts purely to appear busy.

## 11. Optimization plan

Establish same-scene baselines before adding detailed textures and cockpit displays. Measure CPU simulation, GPU frame time, allocation rate and resident memory separately. Keep geometry static, reuse materials and instance repeated wheels/rings. Add distance-based detail only after checking silhouette continuity and camera transitions. Avoid high-frequency material/mesh replacement.

A future physics model can evaluate rigid-body/actuators at the solver rate, intake dynamics at a bounded intermediate rate and thermal/fuel-management at lower rates. Keep deterministic stepping and test convergence. Fixed pools and histories prevent long flights from growing memory. Reserve performance claims until an approved short comparison demonstrates them.

## 12. Delivery roadmap and acceptance

1. **Static preview (this delivery):** selectable original mesh, stable identity, unarmed allocation, successful compilation/tests and signed app. Runtime appearance remains subject to visual review.
2. **Reference/mesh pass:** obtain top/side/front references; verify dimensions, nacelle spacing, chine continuity, canopy and gear; decide whether to retain original geometry or import a licensed model. Review silhouette before spending time on tiny fittings.
3. **Physics foundation:** mass properties, trim and engine maps. Demonstrate sustainable cruise, climb and approach; record source quality and preselected tolerances. Keep unsupported regions labeled estimated.
4. **Handling:** tune augmentation and trim across weights/altitudes, measure oscillation decay and response, prove neutral stability and frame-rate independence. Give players recovery practice.
5. **Systems/animation:** thermal/fuel/inlet dynamics, matched contacts and animations, cockpit and audio. Add failure practice only after normal flight is reliable.
6. **Mission loop:** acceleration/recon/research sorties, optional assisted starts, score clarity and accessibility. User flight review decides whether it feels enjoyable.

Minimum numerical checks include finite zero-speed output, positive inertia, force/moment symmetry, continuous tables, thrust/fuel shutoff, conservation checks and trim residuals. Flight comparisons should cover multiple weights and altitudes, with tolerances set before tuning. Include high-speed neutral-release and input-reversal tests to avoid repeating oscillatory-control failures. Test selector/reset/save compatibility, unarmed behavior, collision, camera views and other aircraft regressions. Automated checks, screenshots and interactive flight acceptance are separate evidence categories.

## 13. Current validation record

See `SR71_VALIDATION.md` beside this report for current build/test results and outstanding checks. No historical cruise-performance claim follows from this static model integration.
