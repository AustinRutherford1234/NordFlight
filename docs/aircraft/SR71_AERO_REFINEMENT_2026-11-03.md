# SR-71 aero refinement, organization and debugging

Date: November 3, 2026. This supersedes the implementation/validation details of the first aero pass where explicitly stated below; the earlier report remains the historical parameter-source record.

## What changed physically

The normal-force circulatory term now uses the incidence-history state. Previously, that state only affected separation while ordinary lift reacted immediately to incidence. Sideslip force now uses the sideslip-history state as well. Incidence history follows the shortest angular difference across the ±π boundary. This avoids feeding a spurious nearly full revolution into the lag state. The existing vortex/separation estimates remain engineering approximations.

Steady trim now includes the required curvature acceleration at the requested speed and height. A fast aircraft following a spherical Earth needs less aerodynamic support than a stationary aircraft at that height. The neutral-load adapter uses the same principle, local gravitational acceleration, body incidence, and bank. This is stabilization, not an altitude controller. It has no altitude-error term and does not snap attitude or velocity. Rotating-Earth and solar terms remain in the shared integrator; the trim approximation does not solve every one of those small terms.

The trim solver now returns a structured result: solution, iteration count, residual vector, and failure reason. The bounded Newton step uses backtracking and accepts a candidate only if it reduces residual norm. Invalid mass/altitude/speed, insufficient speed, nonfinite or singular Jacobian, failure to find a descending step, and iteration exhaustion are distinguishable. Airborne initialization retains the report instead of silently losing information when trim is unavailable. Solver tolerance tightened from 1e-4 to 1e-5.

Detached control surfaces previously left a false incremental moment because the nominal outer-elevon rigging offset was still compensated after the surface contribution became zero. Contributions are now measured relative to the neutral rigging for the remaining healthy surface. A detached surface has no incremental control authority. Jammed actuator positions now agree between the physical and visual states. Hydraulic loss freezes the actual actuator positions; it does not delete their aerodynamic loads.

Individual surface loads now sum to the reported aggregate force and moment. Linear contributions are evaluated separately. The residual from the combined quadratic deflection drag is apportioned equally across the six control surfaces solely for diagnostic bookkeeping. That allocation is not a claim to resolve local hinge pressure or structural load paths. Jacobian probes skip this detailed bookkeeping and return only aggregate loads.

## Code ownership

| File under `src/aircraft/sr71/` | Responsibility |
|---|---|
| `flight.rs` | Small compatibility facade used by the existing flight loop |
| `config.rs` | Named coefficient tables, typed tuning groups, source/range metadata, parameter export and basic validation |
| `aero.rs` | Pure steady/unsteady coefficient evaluation and induced drag |
| `controls.rs` | Mixing, aerodynamic control increments, actual surface loads, command-rate adapter |
| `engine.rs` | Installed-thrust estimate, inlet recovery and fuel flow |
| `mass.rs` | CG fraction and full inertia tensor |
| `math.rs` | Bounded interpolation and blend helpers |
| `trim.rs` | Steady residual construction, safeguarded solver and initialization |
| `diagnostics.rs` | Checked inputs, runtime snapshot/counters and force/moment budgets |
| `study.rs` | Bounded offline scenarios and streaming CSV/JSON export |
| `tests.rs` | Numerical, failure, trim, closure and convergence checks |

The parent `src/aircraft/sr71.rs` owns the animated actuator state and its rate/health limits. `src/physics.rs` remains the sole rigid-body integrator. The existing SR-specific entry points remain compatible, so other aircraft do not need an architecture rewrite to accommodate this work.

The broader shared flight loop still contains aircraft switches. This pass deliberately removes model internals from that loop rather than adding a second simulator or forcing every aircraft through an untested new abstraction. A future fleet-wide refactor should give each aircraft a common typed load/actuator interface and keep atmosphere, rigid-body integration, contacts and the clock shared. The new `StepLoads` record is a concrete example of grouping related quantities rather than passing a long positional argument list.

## Tuning variables

`config.rs` groups physical and numerical values into `AERO`, `ENGINE`, `MASS`, `CONTROL`, `REGIMES`, and `SOLVER`. Dimensions/reference quantities and derivative tables are named separately. The engine attachment coordinates are shared with the installed propulsion path and trim calculation. Actuator rates and travels are shared with the visual-state update.

Values are compile-time Rust constants. Edit the named value and rebuild. `parameters.json` exports the actual compiled values; it is **not** an input file or a hot-reload override. This avoids implying that changing an exported JSON file changes the game. Mathematical factors such as halves, zero/one bounds, and axis signs remain expressions in the equations. This is not a compile-time dimensional-units library; SI units are documented through names, records and the parameter register.

The new direct `serde` dependency exposes serialization derives already present transitively in the dependency graph; the lockfile adds it to this package's direct dependencies. No new serialization crate version was introduced.

## Debugging tools

Every SR-71 airborne physics step updates a fixed-size snapshot and cumulative counters in `Flight.systems.sr71.diagnostics`. Normal flight does not build JSON, write files, append history, or allocate a trace queue. Runtime capture performs body-frame transformations and budget arithmetic; it has a cost and is not described as free.

The snapshot includes:

- Simulation time, timestep, altitude, Mach, dynamic pressure, incidence and sideslip.
- Baseline, control, damage, engine and total airborne specific-force contributions in body-axis Newtons.
- Corresponding moments in N m, gyroscopic moment, angular acceleration, and Euler-equation closure error.
- Parasite, induced, wave, control and remaining drag, plus drag projected directly from the force vector.
- Requested controls, actual elevons/rudders, rates, left/right attachment, fuel, integrity and inlet recovery.

Gravity and tyre/deck/contact forces are outside this airborne budget. Their absence is explicit; the record does not claim a complete ground-contact force sum.

Flags use these bits: invalid coefficient input 1; repaired flow history 2; outside coefficient-table Mach bounds 4; separated flow 8; saturated normalized command 16; hydraulic loss 32; engine unavailable 64; surface failure 128; invalid load 256; failed initialization trim 512; outside the report's stated Mach flight-test coverage 1024. Latest flags describe the current step, while sticky flags retain prior events. Aircraft selection/reset constructs fresh diagnostic state.

The checked evaluator rejects nonfinite conditions, negative dt, negative density/speed/Mach, and nonpositive temperature before mutating flow state. A corrupt flow history is reinitialized from the current conditions. The compatibility evaluator returns zero coefficients on an invalid evaluation; runtime counters expose rejected inputs. This is a localized containment policy, not a repair system for an arbitrary corrupt world position, attitude or integrator state.

NASA's report describes flight data from Mach 0.4–3.0. The separate evidence-range flag distinguishes that range from the broader coefficient array endpoints. Endpoint holding or a finite value outside the evidence range is not aerodynamic validation. [NASA primary source](https://ntrs.nasa.gov/citations/20020057965).

## Running the offline tools

From the project directory:

```sh
cargo test --locked sr71 -- --nocapture
cargo test --locked
cargo run --locked -- --study-sr71 --sr71-output=tests/sr71-aero-study
python3 scripts/sr71/analyze_aero_study.py tests/sr71-aero-study
```

After building, the study can also run directly with `target/debug/gripen-flight --study-sr71`. It returns before creating a window or renderer. It does not pause, quit, or alter a live Nord Flight session.

The bounded study runs neutral flight, a maneuver/release, left-engine loss, hydraulic loss, a jammed elevon, detached elevons and fuel exhaustion at 60/120/240 Hz. Each case runs at most 60 simulated seconds and stops if the aircraft crashes or its state becomes nonfinite. Failure injection occurs at t=10 seconds. Fault cases are allowed to lose the aircraft: a loss is reported rather than concealed or automatically classified as a numerical failure.

Exports:

| File | Contents |
|---|---|
| `parameters.json` | Exact compiled tuning groups, arrays and reference metadata |
| `trace.csv` | Compact flight/rate/load history, sampled every 0.1 simulation seconds |
| `snapshots.jsonl` | Full force/moment/actuator snapshot at each trace sample |
| `summary.json` | Per-case outcome, peak rate, budget closure, flags and measured mean study-step cost |
| `trim.json` | Valid and intentionally infeasible trim cases with explicit failure reasons |
| `analysis.json` | Analyzer checks, nominal convergence and force-budget closure |

The analyzer checks case completeness, finite trajectories, force/moment budget closure, and nominal 2% timestep convergence. It reports fault-case losses independently. JSONL/CSV are streamed; the runtime does not retain their history. The output directory is bounded by the fixed study length, not an endless soak.

Mean study-step timing includes the diagnostic snapshots and periodic serialization/file writes. It excludes the renderer and is not a pure physics benchmark or whole-app frame-time claim.

The force/moment closure checks establish algebraic bookkeeping and correct interface wiring. They are not an independent validation of the aerodynamic coefficients or real-aircraft response. Jammed positions are clipped to the same surface-specific mechanical stops in the actuator and load paths. Both the angle lag and its derivative use the shortest angular difference; a dedicated ±180° crossing regression prevents a false vortex transient.

## Final measured validation and delivery

- Full suite: **838 passed, 0 failed, 1 ignored**; test execution 26.11 seconds.
- Focused SR-71 suite: **20 passed**. Seven new regression cases cover rejected input/history repair, detached-load cancellation and surface-load sums, explicit trim failures, runtime budget/reset behavior, parameter export, jam stops, and incidence-wrap transients.
- Headless study: **21 cases**, **12,600 detailed snapshots**, all trajectories finite; analyzer passed.
- Force-budget maximum residual: **4.800e-10 N**. Euler moment-budget maximum residual: **7.825e-10 N m**. These are algebraic consistency checks.
- Largest nominal final-altitude difference across the timestep rates: **0.101 m**. Largest relative final-Mach difference: **3.42e-6**. This is nominal convergence evidence, not convergence certification for chaotic loss-of-control cases.
- Neutral Mach-3 flight, 23,000 m start, 120 Hz, 60 seconds: **−59.064 m altitude change**, final **Mach 3.03655**, integrity **1.0**. The earlier pass's equivalent neutral check drifted +208.593 m. Absolute drift decreased; it was not eliminated.
- Trim: all **15 nominal points** solved; maximum combined normalized residual **3.551e-06**. The six intentionally adverse cases returned `InsufficientSpeed` or `NoDescent`; a `NoDescent` report describes solver behavior rather than proving globally that no trim exists.
- Mean measured **headless study-step** cost ranged from **4.26 to 9.47 μs** across cases/rates on this Mac. This includes periodic trace serialization, excludes rendering, and is not an in-app FPS measurement.

At 120 Hz, the injected failures produced these 60-second outcomes:

| Case | Final Mach | Altitude change, m | Integrity |
|---|---:|---:|---:|
| neutral | 3.0366 | -59.06 | 1.000 |
| doublet | 3.0263 | 131.42 | 1.000 |
| left_engine_out | 2.8530 | -233.19 | 1.000 |
| hydraulic_loss | 3.0346 | -28.78 | 1.000 |
| jammed_elevon | 3.1115 | -1731.58 | 1.000 |
| detached_elevons | 1.1895 | 6814.82 | 0.489 |
| fuel_exhaustion | 2.5253 | 74.05 | 1.000 |

Fuel-exhaustion injection sets fuel to zero instantly and therefore also changes mass/inertia abruptly. It is a stress case, not a fuel-burn endurance profile. All-elevon detachment produced a large departure and integrity loss; the study does not conceal this by restoring controls. No real-aircraft failure-response accuracy is claimed.

Compact archived evidence is in [sr71-refinement-evidence](sr71-refinement-evidence/). Full CSV/JSONL remain under `tests/sr71-aero-study/` locally and can be regenerated with the commands above. The source report's coefficient uncertainty and the engineering estimates for nonlinear flow, drag, mass distribution and propulsion remain in effect.

`./build-mac.sh dev` rebuilt the existing Mac app. Ad-hoc signing and `codesign --verify --deep --strict` passed. Executable SHA-256: `99df56fbedb3ed2f8ae01322b0b3fa3df14686a6ebede9acd86581d20b60ca5d`. No live flight session was interrupted; no live flight acceptance or renderer benchmark is claimed.
