# SR-71A aerodynamic model — November 2, 2026

## Scope and fidelity

The SR-71 now has an aircraft-specific reduced-order aerodynamic and installed-propulsion model inside the existing double-precision, six-degree-of-freedom flight loop. The imported model's rate-limited elevons and rudders generate the control loads; their animation is no longer disconnected from the plant. This is a public-data-informed simulation, not a reproduction of the proprietary Blackbird aerodynamic database, inlet control system, or stability augmentation system.

The primary evidence is NASA/TP-2002-210718, *Stability and Control Estimation Flight Test Results for the SR-71 Aircraft With Externally Mounted Experiments*, Moes and Iliff, June 2002. Only curves labeled **baseline** were used; LASRE and experiment configurations were excluded. [NASA record and full report](https://ntrs.nasa.gov/citations/20020057965). Local source PDF and inspected graph images are under `tests/sr71-aero/` (generated evidence, excluded from Git).

The source's aerodynamic reference area, span, and chord are coefficient normalization quantities. They are deliberately distinct from the imported model's visual dimensions. Its stability derivatives are conventional aircraft body-axis derivatives. The implementation converts conventional `(forward, right, down)` into game `(-Z, +X, -Y)` and converts per-degree derivatives to per-radian values before applying them. This conversion matters especially for yaw, roll, and the inertia product.

## Parameter register

| Quantity | Implemented value / units | Basis | Confidence / uncertainty |
|---|---|---|---|
| Reference area | 1,605 ft² / 149.1094 m² | NASA report nomenclature | High for normalization; not a measured visual planform |
| Reference span / chord | 56.7 ft / 37.7 ft | NASA report nomenclature | High for coefficient normalization |
| Zero-fuel reference mass | 60,728 lb / 27,545.76 kg | NASA Table 2 baseline | High for that test configuration; equipment variation not represented |
| Zero-fuel reference CG | 20.1% MAC | NASA Table 2 | Configuration-specific |
| Conventional Ix / Iy / Iz / Ixz | 220,660 / 954,850 / 1,172,039 / 19,200 slug ft² | NASA Table 2 | Configuration-specific; no instrument error estimate provided here |
| Fuel capacity | 80,000 lb | Representative capacity retained for game | Estimate; ±10% |
| Aggregate fuel CG | 23.5% MAC | Engineering estimate | ±3% MAC; no tank-transfer simulation |
| Fuel inertia additions | fuel mass × (22,31,9) m² in game axes | Engineering estimate | ±30% |
| Wing/engine reference incidence | 1.2° nose-down relative to fuselage | NASA aircraft description | Reference alignment; mesh station registration approximate |
| Elevon travel | Inner +20° / −35°; outer ±35°, with additional 3° up rigging | NASA Table 1 and description | Source-supported; clipping used at stops |
| Elevon / rudder rate | 30°/s / 33°/s | NASA Table 1 | Nominal; existing hydraulic damage reduces authority |
| Rudder travel | ±20° | NASA Table 1 | Nominal |
| Longitudinal derivatives | Mach tables CNA, CMA, CNB, CMB, CNDE, CMDE, CNq, CMq | Approximate readings of NASA baseline Figs. 15–18 | Moderate trend confidence; assume ±20% slope / ±30% intercept and damping |
| Lateral derivatives | CYβ, Clβ, Cnβ, Clp, Cnr, Clda, Cndr, CYdr | Approximate baseline Figs. 36–40; remaining cross terms estimated | ±25–40%; cross terms ±50% |
| Reynolds / skin friction | Sutherland viscosity; compressible turbulent flat-plate correlation | Engineering correlation | No detailed transition or heating coupling |
| Wetted/reference area ratio | 4.2 | Engineering estimate | ±25% |
| Residual parasite coefficient | 0.0042 | Engineering estimate | ±30% |
| Wave drag | Gaussian transonic increment 0.018, plus supersonic increment 0.009 | Engineering estimate | ±35%; not flight-test drag polar |
| Span efficiency | 0.79 nominal, separation/sideslip reductions | Engineering estimate | ±20% |
| Gear / open-door drag | 0.055 × extension + 0.012 × opening | Engineering estimate | ±40%; actual positions, not key-state switch |
| Installed static dry / wet thrust, both engines | 180 / 289.134 kN | Dry estimate; wet approximately two 32,500-lbf engines | Dry ±20%; wet rating is not an installed high-Mach deck |
| Density lapse / Mach augmentation | density ratio^0.62; tabulated dry and wet Mach factors | Engineering estimate | ±30% away from static rating |
| Fuel flow | installed thrust × 23–44 mg/(N s), burner dependent | Engineering estimate | ±25%; no full thermodynamic cycle |
| Separation / vortex constants | Convective lag 0.035–0.6 s, delayed reattachment, bounded vortex transient | Engineering estimate | Qualitative nonlinear behavior; ±50% time constants |

All numerical Mach arrays are visible together at the top of `src/aircraft/sr71/flight.rs`. They are approximate graph readings, not a digitized instrument dataset. Table interpolation is bounded and continuous in value; its derivative is piecewise constant. Above the last tabulated Mach, endpoint values are held. That is a numerical boundary, not a validated extension of the flight envelope.

## Force model

Normal force depends on Mach, incidence relative to the wing reference plane, nonlinear chine/delta contribution, left/right attachment state, and normalized pitch rate. Separation has finite onset and slower recovery rather than an instantaneous lift switch. Roll rate changes the two sides' effective incidence. Large-angle force recovery transitions toward a bounded flat-plate approximation. No random departure impulse is added.

Drag has separately accounted skin friction, residual parasite, transonic/supersonic wave, induced, separated-flow, sideslip, control-deflection, gear-door, and windmilling terms. Induced drag is recomputed once from **total** lift after control contributions; the generic fighter induced correction and generic pitch correction are bypassed for this aircraft. Ground proximity reduces induced drag with bounded strength. This remains a lumped aircraft model, not a panel-method or CFD solver.

Pitch, yaw, and roll moments use the source reference chord/span, Mach derivatives, rate damping, control deflections, and the estimated changing CG. Full tensor Euler integration retains rotational coupling. Existing damage flags affect surface contributions. Aggregate fuel inertia changes with consumption. This does not model six-tank fuel management or component-by-component changing CG registration.

The J58-class installed-thrust approximation uses the existing spool/engine controls and damage paths, density and Mach maps, inlet-angle loss, and animated spike mismatch. Thrust is applied through the twin engine locations at the source incidence, producing physical pitch and asymmetric yaw moments. Fuel shutdown removes powered thrust and retains windmilling drag. There is no new engine thermal cycle, inlet unstart transient, or independent per-engine spool solver in this milestone.

## Controls and initialization

Existing pilot inputs and **T** assistance toggle remain. With assistance on, an SR-specific rate-demand adapter commands the actual rate-limited actuators. It uses lower high-Mach command rates, bounded angular acceleration, rate damping, sideslip correction, and neutral-load trimming. It is an approximation for playability, not production SR-71 SAS logic. Direct mode leaves the plant and actuators active without this adapter.

The controller linearizes around the current commanded surface state, including asymmetric up/down travel. Linearizing around neutral caused a false pitch bias and was corrected during testing. Engine moments are included in allocation rather than canceled by imposing aircraft attitude or angular velocity.

Airborne selection uses a nonlinear steady trim solve for incidence, elevon command, and throttle at the current speed, altitude, and mass. It does not boost speed. Unsustainable conditions can fail to trim; there is no guarantee that selecting an SR-71 at a slow fighter approach condition produces a sustainable flight. The aircraft has no conventional flap or separate speedbrake commands: **F/Y are suppressed for SR-71**, and the generic fighter autopilot is unavailable. Canopy/gear controls remain unchanged.

## Validation boundaries

Headless checks cover coefficient finiteness and dissipative drag, left/right symmetry, zero airflow, interpolation continuity, physical control signs, positive/invertible mass tensors, trim residuals, engine shutdown/asymmetry, actuator rate limits, and full-loop trajectory convergence at 60/120/240 Hz. CSV outputs record the trim matrix and maneuver/release trajectories. These checks establish numerical behavior and internal consistency, not agreement with a complete operational flight envelope.

No live Nord Flight flight review or whole-app FPS benchmark was performed for this aero pass. A coefficient microbenchmark measures only this model's evaluation cost in the optimized test profile; it excludes the shared solver, controls, collision, rendering, and asset loading. User flight review remains necessary. Detailed stall, high-altitude inlet behavior, takeoff/landing distances, thermal limitations, and full Mach/altitude performance validation remain estimated or unvalidated.

## Measured results from this delivery

- `cargo test --locked`: **831 passed, 0 failed, 1 ignored**, 28.93 seconds for the test executable. The ignored test was not counted as validation.
- Seven new aerodynamic/flight tests accompany the six existing SR-71 rig/menu tests.
- **15/15 trim points solved** at masses 35.55, 47.55, and 57.55 tonnes. Cases span 1,000–25,000 m and 120–950 m/s. These are selected combinations, not a claim that every speed works at every altitude. Maximum combined normalized trim residual was **9.99×10⁻⁵**; force residuals are normalized by weight and pitch moment by qSc. [Trim CSV](sr71-aero-evidence/trim.csv).
- A 60-second Mach-3 maneuver/release sequence, beginning at 23,000 m, finished at 23,384.81 / 23,385.06 / 23,385.16 m for 60/120/240 Hz. Final Mach was 3.01524 / 3.01522 / 3.01521. Rates remained bounded, aircraft integrity remained 1.0, and position differences versus 240 Hz were below the 100-m acceptance bound. This validates nominal step convergence, not all failure/contact scenarios. [Trajectory CSV](sr71-aero-evidence/trajectory.csv).
- Neutral controls for 60 seconds at the same initial condition produced **+208.59 m altitude drift**, Mach **3.02482**, final rate-vector magnitude below 0.001 rad/s, and no structural damage. This is an explicitly recorded limitation; no altitude-hold promise is made.
- A 100,000-iteration coefficient-only microbenchmark measured **273 ns/evaluation** on this Mac in the optimized Cargo test profile. This excludes controller allocation, rigid-body integration, collisions, graphics, and normal application scheduling. It is not a frame-rate benchmark.

The full-loop tests uncovered an SR-71 collider-cache index error, corrected by adding its hull set. They also uncovered pitch allocation bias from asymmetric elevon travel and missing engine moment compensation. Both were repaired before the recorded final trajectories. No attitude snap was used to hide the resulting flight errors.

## Reproduce

```sh
cargo test --locked sr71 -- --nocapture
cargo test --locked
./build-mac.sh dev
```

The first command regenerates `tests/sr71-aero/trim.csv` and `trajectory.csv`. Diagnostic output states the coefficient microbenchmark context explicitly. Normal controls select the SR-71 through F6 in a non-carrier scenario. Live flight acceptance is pending; this delivery did not interrupt a running flight.

## App delivery

`./build-mac.sh dev` completed successfully. The existing `Nord Flight.app` bundle was rebuilt and ad-hoc signed; `codesign --verify --deep --strict` passed. Installed executable SHA-256: `85599254e11c67f22c9a92e66888c4f4f67bf933a6c471e6d55a78d6e58b0a98`. This verifies the bundle's build/signature, not live flight behavior.
