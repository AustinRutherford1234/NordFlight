# SR-71 integrated J58-class propulsion model

Date: November 3, 2026. This implementation replaces the SR-71's density/Mach thrust multiplier with two independent dynamic propulsion systems. It is a reduced-order engineering simulation, not a validated J58 component deck. The existing rigid-body flight loop remains the aircraft integrator. The terrain/environment changes in this checkout are retained.

## What the aircraft now simulates

Each side has its own shaft speed and energy, fuel valves, main ignition, afterburner ignition, consumable ignition inventory, compressor bleed, inlet spike, bypass/shock state, nozzle area, health and faults. The flight loop receives the resulting force and attachment moment, rather than an imposed yaw or speed change. Engine master, throttle and afterburner input range remain compatible with the existing controls. **3** retains the existing engine toggle, subject to battery/APU controls. It is no longer an instantaneous restart.

Airborne scenarios initialize a warm, running pair at the trim operating point. On the ground, a stopped engine uses an abstract starter-cart connection when the aircraft is stopped, battery/APU are available, and the engine is commanded on. The cart has finite power/torque and is not a new visual object or cockpit workflow. A windmilling airborne engine can relight only within the estimated ignition envelope and with ignition inventory available. These are gameplay-accessible approximations, not a checklist trainer.

No solid thermal states, airframe heating, temperature-dependent structural damage, or engine life model are added. Gas stagnation temperatures and fuel-energy balances are necessary to compute compressor work and exhaust momentum, and are included. The working fluid is an approximate air/combustion-gas pair; non-Earth atmospheric chemistry is not validated.

## Component calculation and force accounting

The quasi-steady gas path is evaluated inside a dynamic shaft/controller model:

1. Freestream static pressure/temperature become stagnation conditions. Inlet recovery depends on Mach, distortion and the shock state.
2. Corrected shaft speed and estimated corrected-flow tables determine compressor flow. Supersonic captured flow is limited by inlet geometry and axial airflow.
3. Compressor specific head scales with corrected speed squared. The pressure ratio follows the isentropic temperature relation and compressor efficiency. The front/rear work split allows interstage bleed to bypass the rear compressor, combustor and turbine.
4. Main fuel adds chemical energy to the core stream. Turbine expansion subtracts shaft work. Bleed rejoins the exhaust, afterburner fuel adds energy, and estimated secondary ejector air mixes downstream. Air and hot-gas heat capacities are used consistently across mixing boundaries.
5. Choking, pressure ratio, finite throat/exit area, exit Mach and pressure determine exhaust momentum and pressure thrust. A bounded flow-matching solve reduces compressor flow when an actual throat restricts it. Nozzle matching residuals remain visible in diagnostics.

The control-volume force uses exhaust momentum plus pressure thrust minus captured inlet momentum. In body-axis form, the transverse captured-air momentum is also applied. Spillage, unstart and unpowered windmilling drag act at the intake; powered thrust acts at the exhaust attachment. Lever arms are measured from the current mass-model CG. Negative net powered-cycle thrust contributes drag instead of being silently discarded. Historical inlet/core/ejector thrust percentages are **not** added as extra forces: the integrated momentum calculation already includes the propulsion stream. Applying extra percentages would double-count it. The momentum/pressure-force basis follows [NASA's thrust equation](https://www1.grc.nasa.gov/beginners-guide-to-aeronautics/thrust-force/).

Shaft dynamics use `J dω/dt = τ_turbine − τ_compressor − τ_accessories + τ_starter + τ_windmill`. Rotor kinetic energy and discrete torque work are exported, including work associated with numerical speed bounds. Windmilling can draw energy from the airstream and carries aerodynamic drag; an engine off at rest cannot create powered thrust. Component integration is bounded to 1/480 s inside the existing 1/240 s maximum SR-71 aircraft step. Independent component tests also integrate directly at 240/480/960 Hz.

Fuel consumption uses each engine's actual metered main/afterburner flows and is deducted once in the propulsion adapter. The shared generic fuel-burn and spool-lag paths are skipped for this aircraft. Infinite fuel preserves stored fuel while retaining a diagnostic consumption rate. The last partial-fuel step scales powered loads while retaining passive inlet drag.

## Inlets, controls and failures

The translating spike and bypass actuators have finite rates and hydraulic authority. Supersonic shock stability uses an explicitly estimated margin built from spike mismatch, distortion, flow demand and bypass position. Sustained adverse conditions cause an unstart: pressure recovery and thrust fall while inlet drag rises. Recovery opens bypass and moves the spike forward, then returns toward the normal schedule after the recovery conditions persist. The two sides can unstart and recover independently.

A shaft-speed governor controls main fuel using estimated feedforward compressor/turbine work, proportional correction and a bounded integral. Integral accumulation is restricted during large speed errors and the early start regime. Metering uses observed flow as well as cold demand, preventing a nozzle-restricted start from being fed as if it had unrestricted airflow. Duct pressure losses scale with flow rather than applying a full-load loss at cranking speed. Afterburner fuel, compressor bleed and nozzle response have separate dynamics. The nozzle follows required flow area unless jammed or without authority.

The model includes deterministic flameout/surge conditions, restart cooldown, failed ignition, fuel-pump loss/degradation, spike/bypass/bleed/nozzle jams and seized engines. Damage to either engine affects its component map and force independently. Estimated surge margin, persistent fuel/air-ratio excursions and low inlet pressure drive failures; there are no unexplained random engine failures. Low-speed/reverse taxi flow does not itself extinguish a running engine.

Plumes and sound read actual running/afterburner state. Each rendered spike reads its own inlet state, and the existing nozzle animation reads actual normalized throat area. `Flight.spool` remains a compatibility **visual power proxy**, not mechanical RPM. Actual RPM is available per engine in diagnostic snapshots.

## Evidence and uncertain parameters

Public sources anchor architecture and a few reference values; they do not supply the off-design maps or controller thresholds used here.

| Parameter family | Basis and confidence | Simulation estimate/uncertainty |
|---|---|---|
| Independent single-shaft core, interstage bleed | [Smithsonian J58 record](https://www.si.edu/object/nasm_A19920006000); architecture anchor | A log-pressure split approximates the front stages; not individual stage CFD |
| Nominal compressor pressure ratio | Smithsonian: 8.8:1 | Full off-design head/flow curves are authored estimates |
| Shaft operating anchors | [Peter Law propulsion presentation](https://www.enginehistory.org/Convention/2013/SR-71Propul/SR-71Propul.shtml): 4,000/7,400 rpm | Governor dynamics and rotor inertia are estimates; ±50% sensitivity range |
| Afterburning rating comparison | [USAF museum J58 fact sheet](https://www.nationalmuseum.af.mil/Visit/Museum-Exhibits/Fact-Sheets/Display/Article/195665/USAFmuseum/pratt-whitney-j58-turbojet/): 32,500 lbf | Initial model sanity tolerance ±10%, rather than certification accuracy |
| Mixed-compression inlet operation | [J. Thomas Anderson inlet presentation](https://www.enginehistory.org/Convention/2014/SR-71Inlts/SR-71Inlts.shtml) | Recovery ±0.08 absolute; estimated stability thresholds/timing ±50% |
| Corrected flow and bleed/ejector splits | Authored component map, architecture informed by the references | Flow ±20%; bleed/secondary splits ±50% relative |
| Efficiencies and pressure losses | Engineering estimates | Efficiency ±0.05 absolute; no measured efficiency map |
| Inlet/attachment/nozzle geometry | Existing rig registration and estimated effective flow geometry | Capture area ±15%; effective nozzle areas ±25%; attachment registration requires further validation |
| Ignition inventory and failure envelope | Representative architecture and authored limits | 16-shot inventory is an estimate here, ±4-shot sensitivity; not an operational guarantee |
| Gas-cycle temperature caps | Approximate historical gas-temperature anchors and energy model | ±100 K sensitivity; not a material thermal limit or life estimate |

These ranges are suggested engineering sensitivity ranges, not measured statistical confidence intervals. `propulsion/config.rs` owns tunable parameter groups and exports them with source/confidence/units notes. The register labels legacy simple-engine values separately; they no longer drive runtime thrust or fuel flow. No proprietary compressor map, operability deck or operational engine manual has been reconstructed.

## Diagnostics and reproduction

Production files are under `src/aircraft/sr71/propulsion/`: `config.rs`, `gaspath.rs`, `inlet.rs`, `state.rs`, `study.rs`, `tests.rs`. `sr71/engine.rs` owns the twin-engine/flight adapter. `trim.rs` uses the same healthy equilibrium load calculation, including inlet momentum and attachment moments.

Snapshots include both full engine states: station pressures/temperatures/mass flows, corrected quantities, shaft powers, gross/ram/pressure/net forces, nozzle matching, gas/mass/shaft-work residuals, valve/actuator states, ignition inventory and fault counters. Fixed-size state is retained per aircraft; runtime stepping performs no file I/O, report serialization or growing-history allocation. Flow matching uses a bounded number of iterations. Study tracing and performance sample allocation are offline only.

```sh
cargo test --locked
python3 scripts/sr71/test_engines.py
./build-mac.sh dev
"Nord Flight.app/Contents/MacOS/gripen-flight" --study-sr71-engines --sr71-output=tests/sr71-engine-study
"Nord Flight.app/Contents/MacOS/gripen-flight" --study-sr71 --sr71-output=tests/sr71-engine-coupled
python3 scripts/sr71/analyze_engine_study.py tests/sr71-engine-study
python3 scripts/sr71/analyze_aero_study.py tests/sr71-engine-coupled
```

The lightweight Python command compiles the **same production component modules and tests** against cached Cargo dependencies, bypassing the full Bevy link. It does not copy the engine equations. If the build-cache housekeeping has cleared dependencies, run Cargo tests first.

The engine study writes 150 steady points, 10 transient scenarios at three independent rates (30 runs), trace CSV, final-state JSON, parameter register and paired-engine timing samples. Steady points explicitly flag whether shaft/nozzle matching is feasible. High-Mach/low-altitude corners outside the aircraft operating envelope are stress queries, not validated trim points. The coupled study separately exercises the flight model. Neither headless study is a renderer/FPS benchmark or live handling review.

## Validation results

The final full suite passed **879 tests**, with **0 failures and 1 pre-existing ignored test**, in 31.64 seconds test execution. This includes 47 SR-71-named component, flight, spatial, rig and selection tests. The existing project regressions and the retained terrain/environment tests are included.

The rebuilt executable completed **150 steady queries** and **30 independent engine transient runs**: ground start, failed ignition, throttle step, cruise, inlet unstart/recovery, nozzle jam, bleed jam, pump failure, fuel exhaustion and windmill restart, each at 240/480/960 Hz. The analyzer passed. Worst fuel/impulse/peak-thrust convergence difference across these runs was **0.0665%**. All healthy cases stayed running without unsolicited surge/flameout. The forced inlet event produced an unstart and surge, then recovered. The nozzle-jam case flamed out; pump loss and fuel exhaustion produced zero powered thrust. A jammed bleed valve alone did not force a failure at this tested point; it changed the cycle rather than triggering a scripted outcome.

At sea-level static conditions, one engine produces **86.27 kN dry** and **135.82 kN afterburning**, with corresponding main fuel **2.283 kg/s** and combined wet fuel **7.492 kg/s**. Wet thrust is **6.05% below** the 32,500 lbf reference, inside the preselected ±10% initial sanity tolerance. This is a rating comparison, not validation of the full thrust/fuel-flow map. A 60-second ground-start case ends running at about **4,070 rpm**, having used one ignition shot. Initial warm-airborne states are explicit scenario conveniences, not simulated completed start sequences.

**Four of 150 stress-grid points are flagged infeasible** for steady shaft/nozzle matching: sea-level Mach 2 at idle demand, and sea-level Mach 3.2 at dry maximum/partial/full afterburner. They are not accepted trim solutions or evidence of real aircraft performance. The authored map should not be used to claim a sustained high-Mach sea-level envelope. Positive nozzle matching residuals represent unused open-nozzle capacity because the modeled core jet can occupy a smaller effective area; negative residuals represent capacity shortfall. This is still a simplified nozzle/core matching treatment.

The coupled executable study completed **21 bounded full-flight/failure runs** and **12 independent airframe runs**; its analyzer passed. Across **1,251 snapshots**, maximum engine air-mass residual was **0 kg/s**, gas-energy residual **3.58e-7 W**, and discrete shaft-work residual **1.24e-8 J**. Maximum total aircraft force-budget closure error was **9.31e-10 N**. These small residuals establish arithmetic consistency within the model, not the correctness of its estimated parameters.

Representative coupled runs at 120 Hz, up to 60 simulated seconds:

| Case | Altitude change, m | Final Mach | Integrity | Aircraft lost |
|---|---:|---:|---:|---|
| Neutral trimmed flight | +46.15 | 3.0305 | 1.000 | No |
| Control doublet | +234.13 | 3.0179 | 1.000 | No |
| Left engine out | -110.70 | 2.8821 | 1.000 | No |
| Hydraulic loss | -8,124.47 | 3.3804 | 0.929 | No |
| Jammed elevon | -10,727.64 | 3.2605 | 0.000 | Yes |
| Detached elevons | -7,664.50 | 3.0533 | 0.402 | No |
| Fuel exhaustion | +249.44 | 2.6098 | 1.000 | No |

The hydraulic failure now also removes inlet/nozzle actuation authority; its severe descent is reported rather than hidden. Stabilization is not altitude hold, so neutral altitude drift remains. Injected fuel exhaustion also removes fuel mass immediately in the study; it is not an endurance result. Matching 60/120/240 Hz full-flight caller results share the same internal cap and demonstrate frame partition consistency; independent component and airframe timestep studies provide separate convergence evidence.

On this **Apple M1 / 8 GiB Mac**, 6,000 warmed headless timing samples in the shipped **dev/opt-level-1** executable measured the pair of engines at **15.39 μs mean, 16.00 μs p95 and 16.63 μs p99 per 240 Hz aircraft tick** (four component steps). At normal 60 Hz rendering, four such aircraft ticks correspond to approximately **0.062 ms mean engine computation per frame**. That is an arithmetic conversion of this isolated workload, not a measured full-game frame time. No renderer benchmark or live flight was run.

The existing `Nord Flight.app` was rebuilt with the terrain changes retained, and ad-hoc signing plus strict/deep signature verification passed. Executable SHA-256: `07f210098a4f2551f2575087f1905bbfd7cbedfb8ef774ce060f4ef1afb99331`. Compact evidence is archived in [sr71-engine-evidence](sr71-engine-evidence/); full traces and snapshots remain locally in `tests/sr71-engine-study/` and `tests/sr71-engine-coupled/`.

Remaining fidelity limits include measured off-design compressor/turbine maps, detailed variable-guide-vane scheduling, resolved compressor surge oscillations, full inlet shock/plenum gas storage, exact secondary ejector geometry, rotor gyroscopic/reaction coupling, fuel plumbing/JP-7 properties, physical starter-cart hardware, and real transient/flight calibration. Gas stations are quasi-steady; only shaft, metering, actuator and operability states carry dynamics. No solid thermal effects are included. Automated evidence is complete for this implementation; live visual/handling review and real-aircraft validation remain separate.
