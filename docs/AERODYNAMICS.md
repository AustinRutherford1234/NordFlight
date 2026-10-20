# Nord Flight aerodynamic model

The jet uses a six-degree-of-freedom rigid-body model and a hybrid aerodynamic model: a neutral swept delta-wing/canard airframe polar plus incremental forces from nine moving panels. It runs at 120 Hz independently of rendering. The model is intended to make a flyable, tunable game; its coefficients are not a Saab flight-test database.

## Frames and signs

Body +X points right, +Y points up, and −Z points forward. `Flight.orientation` maps body vectors into the rotating planet frame. `Flight.velocity` is velocity in that planet frame. Aerodynamic velocity is velocity relative to the local wind.

For each physics step:

\[
\mathbf v_b=Q^{-1}(\mathbf v-\mathbf w),\qquad
\alpha=\operatorname{atan2}(-v_{b,y},-v_{b,z}),\qquad
\beta=\arcsin(v_{b,x}/|\mathbf v_b|).
\]

Dynamic pressure is \(q=\rho V^2/2\). Moments about body +X, +Y, and +Z are pitch, yaw, and roll respectively; the pilot's positive roll input is converted to a negative body-Z rate request because forward is −Z.

## Neutral airframe

`src/aerodynamics.rs` evaluates lift, drag, side force, and natural moment coefficients. Reference area is 30 m², span is 8.4 m, and mean chord is 3.5 m. These are aerodynamic reference dimensions; the procedural exterior is an approximate visual model.

The attached-flow lift curve combines a finite swept-wing slope, a neutral lift offset, delta-wing vortex lift, and pitch-rate lift. A smooth subsonic-to-supersonic slope transition avoids a singularity at Mach 1. The high-incidence branch uses a bounded flat-plate curve, blended with an attached-flow state.

The attached state has separate separation and reattachment time scales. Its threshold includes hysteresis, so reducing angle of attack after a stall does not instantly restore full authority. Incidence has a convective first-order lag. A small bounded transient vortex term develops during changing incidence and relaxes once incidence becomes steady. Its constants are game assumptions, not a fit to a particular dynamic-stall experiment.

Drag includes Reynolds-dependent turbulent skin friction, a base parasite term, gear drag, induced drag, a smooth transonic wave-drag rise, high-incidence separation drag, and sideslip drag. Skin friction uses a Sutherland viscosity relation. Ground effect reduces induced drag rather than adding arbitrary lift.

\[
C_{D,i}=\frac{C_L^2}{\pi AR e}\,G(h),\qquad
AR=\frac{b^2}{S}.
\]

The final induced-drag calculation uses **total lift**, including the moving panels. Its difference from the neutral-airframe induced drag is applied once as a wind-axis drag correction. This avoids charging for neutral-wing induced drag twice and makes lift-producing flap deflections carry an additional drag cost.

Natural pitch/yaw/roll moments include static stability, nondimensional angular-rate damping, a Mach-tuck term, and separated-flow pitch break. A filtered sideslip state contributes bounded unsteady lateral-directional effects. Those derivatives are openly tunable approximations.

## Individual moving surfaces

`src/control_surfaces.rs` contains fixed panel geometry and the deflection mapping used by both rendering and physics:

| Surface | Mixing | Principal effect |
| --- | --- | --- |
| Two canards | Common pitch | Lift ahead of the reference origin; pitch torque |
| Two inner elevons | Pitch, differential roll, flap droop | Lift and pitch/roll torque |
| Two outer elevons | Pitch, differential roll, reduced flap droop | Lift and roll/pitch torque |
| Rudder | Yaw | Side force, yaw torque, coupled roll torque |
| Two airbrakes | Airbrake command | Drag and pitch moment from their lever arms |

Each surface evaluates its own local airflow including rigid-body rotation:

\[
\mathbf v_i=\mathbf v_b+\boldsymbol\omega\times\mathbf r_i,\qquad
q_i=\frac12\rho|\mathbf v_i|^2.
\]

For a lifting panel, deflection changes effective incidence. A bounded local polar blends attached finite-wing lift with separated flat-plate lift. Mach reduces panel effectiveness. The code subtracts the neutral panel lift because neutral lifting surfaces are already represented in the whole-airframe polar:

\[
\Delta C_{L,i}=k_i\left[f_i(\alpha_i-\eta_i\delta_i)-f_i(\alpha_i)\right].
\]

The lift direction is the panel normal projected perpendicular to local airflow. Drag opposes that airflow. Deflection profile drag and a quadratic incremental-lift drag term are included. Airbrake drag follows its exposed-angle sine squared. Panel force and moment are:

\[
\mathbf F_i=q_i S_i\left(\Delta C_{L,i}\hat{\mathbf l}_i-C_{D,i}\hat{\mathbf v}_i\right),\qquad
\mathbf M_i=\mathbf r_i\times\mathbf F_i.
\]

Force and moment sums feed translational acceleration and rigid-body Euler dynamics. Rudder side force therefore produces roll as well as yaw when its aerodynamic center lies above the reference origin. No direct angular-velocity increment is substituted for these aerodynamic moments.

Flap and airbrake coefficients are disabled in the neutral-airframe evaluation when flying the jet: their forces come from the moving panels. The standalone neutral polar still accepts those parameters for isolated coefficient tests.

## Actuators and fly-by-wire

Servo motion combines first-order response with a hard rate limit and normalized travel limits. Above 25 kPa, a bounded aerodynamic-load factor slows the normalized actuator rate. This is a simplified loaded-servo model; hydraulic pressures, detailed hinge moments, and actuator electrical dynamics are not simulated.

With fly-by-wire enabled, pilot commands request body angular rates. The controller combines desired angular acceleration, natural moment compensation, and flap trim compensation. It computes the full three-axis moment Jacobian from centered differences of the panel model. Damped least squares allocates torque across coupled pitch/yaw/roll commands:

\[
\mathbf u=(J^T J+\lambda I)^{-1}J^T\mathbf M_{\rm requested}.
\]

The implementation normalizes by the largest Jacobian column before regularizing and clips normalized requests to actuator travel. This remains finite near zero dynamic pressure, where aerodynamic control authority vanishes. The controller is a local approximation around neutral actuator command; it is not a nonlinear optimal control allocator.

In manual flight, fly-by-wire also scales pitch demand near predicted high incidence and positive/negative G limits. Disabling it with **T** exposes raw surface commands, including the possibility of stall and departure. Approach assistance remains a separate guidance loop, with the same physical actuator and panel model beneath it.

## Atmosphere and propulsion

The atmosphere integrates hydrostatic ideal-gas pressure through seven lapse-rate layers to approximately 86 km. Geometric altitude is converted to geopotential altitude for layer selection. Temperature, pressure, density, and speed of sound are continuous at layer boundaries. Above this range, a smooth game transition reaches vacuum at 120 km; it is explicitly not a thermosphere or real satellite-drag model.

Jet thrust uses spool response, dry and afterburner ranges, density lapse, Mach correction, and fuel consumption. Exhaust visualization follows spool and thrust; its low-power blue plume remains visible below afterburner. Plume length and brightness are visual effects, not a nozzle-flow solution.

The October 14 performance tuning keeps nominal dry thrust at 50.4 kN and increases the wet increment from 30.1 to 60.0 kN. The transonic Gaussian wave-drag amplitude decreases from 0.085 to 0.028; the downstream supersonic drag term remains 0.022. Above Mach 1.4 a smooth intake-recovery factor joins to a 25% loss at Mach 2.4. These are game-model choices targeting Mach 2 at 60,000 ft, not measured RM12 performance. No velocity clamp is used. `src/propulsion.rs` solves level-flight lift and pitch trim using the same whole-airframe and moving-panel loads, including total induced drag. `--study-performance` checks the terminal thrust/drag balance and runs controlled six-degree-of-freedom flights with altitude feedback and infinite fuel to keep the reference mass fixed.

The spacecraft has main-engine pressure correction and RCS torque/fuel consumption. SAS can hold rates or align prograde, retrograde, radial out, and normal. Orbital elements and the map trajectory use inertial velocity \(\mathbf v_I=\mathbf v+\boldsymbol\Omega\times\mathbf r\), rather than rotating-frame velocity.

## Verification and limits

`cargo test --locked` checks force/moment signs, differential roll, rudder side force, flap lift, airbrake drag, energy-removing drag, local stall behavior, actuator limits, coupled allocation, transient relaxation, atmosphere continuity, orbit geometry, takeoff, landing, and visual/physical deflection agreement.

`--study-flight` writes `tests/aerodynamic-study.json`: angle/Mach/flap polars, pitch/yaw/roll maneuvers with and without FBW, repeated landings, and CPU-only physics throughput. `--verify-flight` separately checks approach landing and a one-hour orbital coast. App-native screenshots validate appearance; render benchmarks validate frame timing on the current host. Neither screenshots nor numerical tests validate the model against real aircraft data.

Known simplifications include fixed aerodynamic centers, diagonal inertia scaled with mass, no fuel-dependent CG migration, no aeroelasticity, no detailed wing/canard wake interaction, no inlet distortion database, no ground-effect CFD, and no calibrated transonic pressure distribution. Per-surface separation is a bounded local polar, while time-dependent stall memory belongs to the whole-airframe state. These boundaries should remain explicit when extending or tuning the game.

## References

- [NASA induced drag coefficient](https://www1.grc.nasa.gov/beginners-guide-to-aeronautics/induced-drag-coefficient/) supports the reference induced-drag relationship.
- [U.S. Standard Atmosphere, 1976](https://ntrs.nasa.gov/citations/19770009539) supplies the lower-atmosphere layer framework; the game's upper vacuum transition is an additional approximation.
- [NASA dynamic stall effects on high-performance aircraft](https://ntrs.nasa.gov/search.jsp?R=19910012793) provides context for transient lift, drag, and pitching-moment effects. The game does not implement that paper's experimental model.
- [NASA lateral-directional derivatives due to changing sideslip](https://ntrs.nasa.gov/api/citations/19750022117/downloads/19750022117.pdf) provides context for unsteady swept/delta-wing lateral dynamics. The coefficients here are not fitted to its data.
