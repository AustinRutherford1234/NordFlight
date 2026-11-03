# SR-71 distributed aerodynamics, landing effects and structural response

Date: November 3, 2026. This follows the first aerodynamic implementation and the earlier refinement/debugging pass. It is a real-time reduced-order airframe model. It is not CFD, a proprietary aerodynamic database, finite-element stress certification, or validated flutter analysis.

The subsequent [integrated engine implementation](SR71_ENGINE_MODEL_2026-11-03.md) supersedes this report's engine-deferral status. The results below remain the historical airframe-pass evidence.

## Scope

Implemented in this pass: local distributed aerodynamic forces, a multidimensional coefficient database, unsteady and post-stall states, supporting-surface landing effects, two-way aeroelastic feedback, wing loading, bending, stress/strain estimates, and diagnostic/validation extensions.

The user's clarification was **wing** loading, bending, stress and strain. No new wind/gust field is added. Existing atmosphere and air-relative velocity remain in use. Engine-specific development is deferred; the engine maps/cycle implementation are unchanged. No airframe heating, temperature-dependent material model, or thermal stress is added. Atmospheric temperature still enters the existing density/speed-of-sound and viscosity calculations.

## Spatial force model

`elements.rs` defines a fixed, symmetric 30-element discretization:

| Region | Elements | Role |
|---|---:|---|
| Wings | 12, six per side | Spanwise normal/drag loads, rotational airflow, bending/torsion feedback |
| Chines | 6 | Forebody normal force, separation/vortex/wake histories |
| Fuselage | 4 | Distributed normal, drag and side force |
| Nacelles | 4 | Airframe drag, normal/side force; no new engine/inlet cycle |
| Fins | 4, two per side | Local side force, bending/torsion response |

Six additional physical control load points correspond to the imported elevon/fin pivots. Gear forces use the same mount/wheel/retraction geometry as the existing gear mechanism. Element positions and coefficient weights export with the parameters.

For each element, aircraft-relative airflow includes translation, rigid-body rotation about the estimated CG, and elastic point velocity:

\[
\mathbf v_i=\mathbf v_{air}+\boldsymbol\omega\times(\mathbf r_i-\mathbf r_{CG})+\mathbf v_{elastic,i}.
\]

It is transformed into the local deformed frame before computing incidence, sideslip, local Mach, dynamic pressure and Reynolds corrections. Forces transform back into aircraft coordinates; point moments use the actual deformed lever arm. There is no new random wing-drop impulse.

The element normal-force, drag and side-force weights each sum to one. They are calibrated pressure/area weights referenced to the NASA coefficient area; they are not independently measured strip pressure distributions. Left/right damage affects the appropriate element loads. Structural health is combined with the shared damage ledger without counting the same degradation twice.

**Whole-aircraft calibration remains explicit.** The source supplies whole-aircraft derivatives rather than a pressure field. Static pressure couples anchor the integrated moments to those derivative trends. Linear reference-flow contributions are removed from the rate-derivative correction before adding the existing whole-aircraft damping target, avoiding straightforward duplication of geometric rate damping. Actual nonlinear, local-flow and elastic increments remain active. Induced drag is evaluated once from total lift, including control increments; the old shared induced correction is disabled for this path. Its load is assigned to generating elements in proportion to their resolved lifting load and transferred into the structural point loads. The control-related induced increment is removed from hinge-load bookkeeping and transferred to those same lifting elements. A dedicated force/moment test checks that this transfer is counted once, including the unresolved pressure couples. The displayed induced/control drag split follows this separation.

Positions follow the normalized mesh and estimated aerodynamic centers. Exact fuselage-station/CAD registration of the reference CG and local pressure centers remains unvalidated. The unresolved pressure couples are real terms in the simulation's reduced-order model; summing point-force lever arms alone is not the entire aircraft moment. Their diagnostic channels are exported separately.

## Coefficient database

`database.rs` contains a bounded trilinear database with **11 Mach × 25 incidence × 7 sideslip points = 1,925 vertices**. Incidence spans −180° to +180°, and sideslip spans −45° to +45°. The lookup includes normal/lift, profile/separated/wave drag, side force, static pitch/yaw/roll moments, and equilibrium attachment. A separate seven-point deflection curve combines with incidence and Mach dependence for nonlinear control effectiveness.

The grid is built once and retained. Runtime lookups interpolate eight neighboring vertices without allocating. Values are continuous across cell boundaries, while their derivatives are piecewise continuous. Endpoint holding is diagnosed when local flow goes beyond the table's Mach/sideslip domain. Reynolds dependence is evaluated analytically with local chord, density, speed and Sutherland viscosity; it is not presented as another measured data dimension.

The vertices are **authored estimates**, constrained by the earlier NASA baseline derivative readings and supplemented with bounded separated-flow approximations. Creating a larger table does not create new experimental evidence. `coefficient_database.csv` marks every exported row `authored_estimate`. The source's reported Mach flight-test coverage remains a separate diagnostic range from the wider numerical table.

At large incidence, a normal-pressure/flat-plate contribution replaces reliance on a small-angle lift law. Vortex, separation, side-force and control-authority estimates remain bounded. No accuracy claim is made for the full ±180° range simply because it is finite.

## Unsteady and post-stall flow

Each of the 30 elements owns independent states for:

- Fast and slow circulatory incidence response with convective time scales.
- Pressure/incidence lag and a bounded dynamic-stall delay.
- Separation and slower reattachment, including hysteresis.
- Vortex strength, decay and a convection-age state with an estimated pitching-moment shift.
- Bounded noncirculatory/added-mass response.
- Wake memory and estimated chine-to-wing downwash coupling.
- Deterministic, smoothly periodic separated-flow buffet tied to local speed/chord.

The lag updates use exponential responses and shortest-angle differences, including the incidence derivative. Controls use the local frozen flow/attachment state for their load calculations. Jacobian probes do not advance the flow or structure.

This is an estimated state-space model informed by the roles of circulatory, separation and vortex states. It is not a validated SR-71 Beddoes–Leishman parameter fit. It does not resolve vortex cores, shock-boundary-layer interaction, a three-dimensional wake lattice, or independently measured departure behavior.

## Landing aerodynamics

The aircraft queries its supporting terrain/deck plane. Each element has its own clearance against that plane; roll, pitch, elastic displacement and surface slope therefore affect left/right and fore/aft ground proximity differently. The contact plane is a local approximation, not detailed flow over every terrain triangle.

Near the supporting surface, the model includes estimated lift amplification, reduced induced drag, an incidence-dependent pitching-moment contribution, pressure-response lag, and a bounded sink-rate contribution. The effect fades with height and Mach. Gear drag follows actual extension and local gear-point airflow, while door drag follows the actual opening state. Gear and door moments use their respective lever arms.

The aerodynamic ground effect does not replace tyre/suspension contacts. Completed rigid-body specific acceleration and angular acceleration, including contact loads, are fed back into the structural calculation on the next bounded step. Thus parked-wing weight and touchdown acceleration can produce wing inertial bending even when aerodynamic lift is small.

## Aeroelasticity, loading, stress and strain

`elastic.rs` models flapwise bending, in-plane bending and torsion for the two wings and the two fin assemblies. Mode displacement and velocity alter aerodynamic position, frame orientation and local airflow. Fixed-size point-load arrays transfer the actual aerodynamic and control loads into the structural model.

For the wings, bending and torsion stiffness derive from the configured effective section/material quantities:

\[
I=Zc,\qquad k_b=\frac{3EI}{L^3},\qquad k_t=\frac{GJ}{L},\qquad
f_n=\frac{1}{2\pi}\sqrt{\frac{k}{m_{modal}}}.
\]

The structural oscillator reuses the project's exact constant-load damped-oscillator solution. This stabilizes each modal update; it does not by itself prove the complete aerodynamic/structural coupling stable. Coupling is bounded to a configurable maximum step, currently 1/240 s. Genuinely finer internal-step experiments are separate from caller-frame partition tests.

At **six stations per wing**, outboard point loads produce axial load, flapwise and in-plane shear, flapwise and in-plane bending moments, and torsion. Aerodynamic and inertial root bending resultants are also retained separately. Four representative section corners combine axial and both bending stresses. Tension and compression limits are checked separately; shear includes transverse and torsional contributions. Effective strain and an equivalent stress are computed as:

\[
\sigma=\frac{N}{A}\pm\frac{M_f}{Z_f}\pm\frac{M_e}{Z_e},\quad
\epsilon=\frac{\sigma}{E},\quad \gamma=\frac{\tau}{G},\quad
\sigma_{eq}=\sqrt{\sigma^2+3\tau^2}.
\]

The record additionally contains modal bending curvature and its associated outer-fiber strain. The modal and section-resultant strain estimates are different reductions of a distributed structure and need not match point-for-point. Sections taper through named estimated properties; the modal stiffness is an equivalent reduced beam, not a tapered finite-element reconstruction.

Two loading measures are distinguished: estimated aircraft weight/planform area, and each wing's aerodynamic force/half-planform area. The currently configured lifting planform is an estimate distinct from the NASA coefficient reference area.

Progressive overload damage reduces stiffness and wing aerodynamic health. Fin overload uses estimated displacement/twist limits. The existing aircraft-level load/Mach failure path remains a separate envelope approximation. No crack propagation, local skin buckling, detailed joints, fatigue life, plasticity, true material stress tensor, or validated structural attachment strength is claimed. Material constants, section moduli, modal masses and damping are engineering estimates; thermal dependence is deliberately excluded.

## Code organization and performance boundaries

The existing flight loop remains the only rigid-body integrator. New modules are `airframe_config.rs`, `database.rs`, `elements.rs`, `plant.rs`, `unsteady.rs`, `landing.rs`, `elastic.rs`, and `spatial_tests.rs`. `plant.rs` assembles loads and interfaces with the shared loop. The prior `aero.rs` remains the reference-polar authoring method used to populate estimated table vertices, not a second runtime force path.

Named parameter groups, geometry, database axes and provenance export together. Elements, flow states, structural states and load buffers use fixed-size arrays. The coefficient grid is allocated once. The normal runtime does not serialize JSON or retain a growing history. Detailed snapshots are sampled at 1 Hz during offline studies; compact traces remain 10 Hz, and forced-airframe CSV sampling is 50 Hz. Every physics step still updates the latest diagnostic state.

Diagnostics now include local incidence/attachment/forces/positions, ground pressure, unresolved force/moment contributions, modal displacement/velocity/energy, section stress/strain, root resultants, wing loading, structural damage, and coefficient-domain warnings. The old coefficient-only microbenchmark exercises the reference-polar authoring function; it must not be quoted as the cost of the distributed runtime model.

## Reproduce and inspect

```sh
cargo test --locked
./build-mac.sh dev
target/debug/gripen-flight --study-sr71 --sr71-output=tests/sr71-distributed-airframe
python3 scripts/sr71/analyze_aero_study.py tests/sr71-distributed-airframe
```

The study has no renderer and does not interrupt a live session. In addition to 21 bounded flight/failure cases, it runs 12 independent airframe cases: free decay, an incidence doublet, a prescribed flare/ground-effect sweep, and asymmetric loading, each at 240/480/960 Hz. These forced-input cases bypass the stabilization adapter. A separate full-loop test exercises raw controls with assistance disabled.

The normal simulation now substeps this aircraft at a maximum 1/240 s. Consequently, identical results from 60/120/240 Hz callers demonstrate frame-partition consistency, not three independent structural integration rates. The finer-step airframe studies provide the separate convergence evidence. Force/moment budget closure establishes bookkeeping, not independent aerodynamic validation.

## Sources and tool suitability

- [NASA/TP-2002-210718](https://ntrs.nasa.gov/citations/20020057965): baseline whole-aircraft derivative trends and existing reference quantities. Its flight-derived derivatives already contain flexibility effects; the added estimated modal feedback is not an independently validated reconstruction. Joint calibration against appropriate dynamic-pressure data remains necessary.
- [NASA: Ground effects related to landing of airplanes with low-aspect-ratio wings](https://ntrs.nasa.gov/citations/19660028848): qualitative motivation for including lift, drag and moment effects; no SR-71 ground-effect numbers were extracted from this record.
- [NREL unsteady aerodynamics module report](https://www.nrel.gov/docs/fy20osti/66347.pdf): background on reduced-order unsteady model components; no wind-turbine coefficient set is transplanted into this aircraft.
- [RASAero official description](https://www.rasaero.com/home.htm): its rocket-oriented scope makes it a candidate for later missile analysis, not the primary database for this wing/chine/twin-nacelle aircraft. RASAero was researched but not run or used to generate these coefficients.

Open questions remain in local pressure distribution, exact reference-frame/CAD registration, high-angle-of-attack and transonic calibration, real stiffness/mass distributions, approach/takeoff performance, and live handling. The model is deeper and more diagnosable; it is not complete real-aircraft validation.

## Final validation and measured results

- Full suite: **853 passed, 0 failed, 1 ignored**, 33.15 seconds test execution. **35** passing tests match the SR-71 filter, including existing rig/menu/reference-polar checks and 15 new spatial/structural checks.
- Final executable study: **21** bounded full-flight/failure cases and **12** independent forced-airframe cases; analyzer **passed**. Detailed export contains **1251** snapshots.
- All **15 nominal trim points** solved with the spatial/equilibrium-elastic plant. Maximum combined normalized residual: **3.062e-06**. Six adverse trim cases returned explicit failures.
- Full-flight 60/120/240 Hz caller results agree because they share the 240 Hz coupled cap. This is partition consistency, not independent-rate evidence.
- Independent 240/480/960 Hz forced-input comparison: largest checked relative peak difference **0.930%**, below 2%. Metrics include force, bending, root stress and root strain.
- Maximum airborne force-budget closure error: **9.331e-10 N**; maximum Euler moment-budget closure error: **7.523e-09 N m**. These check arithmetic/interface consistency, not aerodynamic truth.
- Mean measured headless study-call cost ranged **32.20–163.08 μs**. Calls at 60 Hz contain four coupled substeps; 120 Hz contains two; 240 Hz contains one. Timing includes periodic file serialization, excludes rendering, and is not a game FPS benchmark.

Independent 240 Hz airframe cases:

| Forced-input case | Peak wing bend, m | Peak root stress, MPa | Peak root strain | Final elastic energy, J |
|---|---:|---:|---:|---:|
| free_decay | 0.09901 | 0.000 | 0.000000 | 1.047e-25 |
| alpha_doublet | 0.12200 | 259.532 | 0.002359 | 2670 |
| landing_flare | 0.08963 | 194.948 | 0.001772 | 2552 |
| asymmetric_load | 0.10797 | 230.281 | 0.002093 | 3097 |

These are simulation estimates under prescribed inputs, not measured aircraft benchmarks. Free-decay energy decreases to numerical near-zero with no airflow. The forced-incidence deflection is a consequence of the configured effective stiffness/mass values, not an observed SR-71 deflection.

Full-flight outcomes at 120 Hz, up to 60 simulated seconds:

| Case | Altitude change, m | Final Mach | Integrity | Aircraft lost |
|---|---:|---:|---:|---|
| neutral | 46.37 | 3.0306 | 1.000 | False |
| doublet | 236.37 | 3.0201 | 1.000 | False |
| left_engine_out | -124.91 | 2.8523 | 1.000 | False |
| hydraulic_loss | -827.92 | 3.0665 | 1.000 | False |
| jammed_elevon | -10808.52 | 2.9970 | 0.000 | True |
| detached_elevons | -7754.58 | 2.9502 | 0.330 | False |
| fuel_exhaustion | 186.85 | 2.5145 | 1.000 | False |

The jammed-elevon case loses the aircraft in this simulation; this is reported, not hidden or counted as a numerical failure. Detached elevons produce significant integrity loss. Fuel exhaustion is an instantaneous stress injection that also changes mass, not an endurance run. Neutral flight still drifts: stabilization is not altitude hold.

The reduced modal model includes rigid-body inertial forcing and two-way airflow feedback. It does not solve a full flexible-body mass matrix or exact modal/rigid-body angular-momentum exchange. No energy-conservation or flutter certification is implied by the free-decay check.

Compact evidence is archived in [sr71-distributed-evidence](sr71-distributed-evidence/). Full snapshots and 50 Hz forced-case traces remain locally under `tests/sr71-distributed-airframe/` and are reproducible with the commands above. Baseline element forces and actual structural element forces are exported separately; the latter include the control-related induced-drag transfer. Their distinction prevents pressure/drag bookkeeping from being mistaken for local resolved CFD pressure.

The existing `Nord Flight.app` was rebuilt with `./build-mac.sh dev` and ad-hoc signed. Strict/deep signature verification passed. No live flight or renderer benchmark was run for this pass, and no live session was interrupted. Live handling review remains outstanding.

Executable SHA-256: `720c7286b22d7c070eac0690f4ccb6dcb05d9b702ce47009788b364a05196d87`.
