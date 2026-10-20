# Earth–Moon direct-return spacecraft

F5 starts a finite-propellant lunar mission in a 220 km Earth orbit. The spacecraft now consists of a capsule, an ascent/Earth-return service stage, a four-engine landing stage, and a transfer booster. This is a **fictional enlarged Apollo-inspired direct-return vehicle**, not an Apollo reconstruction. The landing stage remains on the Moon; the ascent/return vehicle carries the capsule back to Earth without lunar rendezvous.

The initial circular orbit is prograde and coplanar with the simulated Moon's
orbit, with the spacecraft pointing along its orbital velocity. This removes the
initial lunar-plane mismatch; transfer timing, injection and capture burns still
need to be planned. Restart the Earth-orbit scenario to use this starting state.

Earth uses an 8192 × 4096 NASA July Blue Marble base, a higher-resolution Nordic
albedo layer with feathered borders, and a separate historical NASA cloud
composite. Ground terrain detail gradually blends into orbital imagery between
18 and 60 km. Sky scattering and its environment-light contribution fade together
between 45 and 95 km, while layered blue limb haze remains visible from space.
Stars emerge gradually between 45 and 100 km. These visual weights depend on
altitude rather than ascent/descent state, so the same transition applies on
reentry. Surface shading is shared with map view. Imagery credits and processing
are in [Earth image sources](../assets/EARTH_SOURCE.md). The cloud shell and limb
are appearance approximations, not volumetric weather or calibrated radiative
transfer.

## Scenarios and controls

The spacecraft mission panel provides **Earth orbit / full stack**, **Moon orbit / lander** (100 km), and **landing approach** (1.5 km above procedural lunar terrain, initially descending at 18 m/s). The approach site is selected for a gentle local slope. Other terrain retains its craters and slopes.

| Control | Action |
|---|---|
| Shift / Control | Full throttle / throttle cut |
| Z / X | Increase / decrease throttle |
| W/S, A/D, Q/E | Pitch, yaw, roll |
| Space / STAGE | Separate the current stage; throttle must be off |
| G | Extend/retract landing legs |
| F7 / MODE | Cycle rate hold, prograde, retrograde, radial out, normal +, maneuver, radial in, normal − |
| T / SAS | Toggle attitude assistance |
| K / ARM PARACHUTES | Arm capsule parachutes |
| M | Toggle orbital map |
| Comma / period / slash | Lower warp / raise warp / return to 1× |
| C / Home | Cycle the four flight cameras / recenter |
| R / P | Restart the selected scenario / pause |

Pilot input overrides target-pointing SAS. Maneuver SAS uses the planned node's direction; the pilot still controls the engine. Stage separation discards unused propellant in that stage. Below 30 m, descent-stage separation requires touchdown, preventing an accidental near-ground separation before landing. Scenario restart clears stage debris, heating, failures, and parachutes.

The mission panel has direct attitude-selection buttons. Selecting one enables SAS, and the active target is highlighted. Prograde/retrograde use primary-relative inertial velocity; radial out/in use position relative to the current primary; normal +/− use the orbital angular momentum and its opposite (orbit-plane up/down). Maneuver requires a planned node; without one it damps rotation instead of aiming at an arbitrary direction.

Use **G** or **EXTEND / RETRACT LEGS** to command the lander legs. Four nested sleeves slide outward over three simulated seconds, with a smooth deployment path and a visible percentage in the panel. Lunar orbit starts stowed; the landing-approach scenario starts deployed. Pause freezes the animation with the rest of the simulation. Landing pads and braces follow the animated foot locations.

## Propulsion and delta-v

Each stage consumes its own main propellant using `mass flow = thrust / (Isp × g0)`. RCS uses a separate 400 kg initial supply and a 220 s modeled specific impulse. Vehicle mass includes retained stages and their unused propellant. Stage delta-v uses the rocket equation and the mass actually carried during that burn.

| Configuration | Initial vehicle mass | Main propellant | Vacuum thrust | Isp | Ideal stage delta-v |
|---|---:|---:|---:|---:|---:|
| Earth-orbit transfer stack | 243.4 t | 170,000 kg | 1,000 kN | 421 s | 4,949 m/s |
| Lunar descent vehicle | 66.4 t | 34,000 kg | 186.8 kN | 311 s | 2,188 m/s |
| Ascent / direct Earth return | 27.4 t | 19,000 kg | 91 kN | 314 s | 3,641 m/s |
| Reentry capsule | 5.9 t with full RCS | None | None | — | None |

The transfer allocation covers lunar injection and orbital capture with a reserve; descent and ascent/return have independent budgets. These are **ideal vacuum budgets**, not guaranteed maneuver costs. Finite burn duration, steering, gravity losses, inaccurate navigation and wasted fuel still matter. The panel shows current-stage delta-v and separately retained descent/return reserves.

Engine performance is inspired by Apollo-era hardware. NASA's [J-2 systems material](https://ntrs.nasa.gov/api/citations/20240002641/downloads/02-Systems_rev3.pdf) and [Apollo propulsion reference](https://www.nasa.gov/wp-content/uploads/static/history/alsj/LM09_Main_Propulsion_ppMP1-22.pdf) provide context. NASA also describes the [SPS engine's original direct-ascent design role](https://www.nasa.gov/history/60-years-ago-first-test-firing-of-the-apollo-service-propulsion-system/). The larger tanks, four-engine descent cluster, vehicle masses, continuous throttling and control tuning are game design assumptions.

## Gravity, map and manual mission planning

**WARP TO MANEUVER · BURN −15s** in the node editor resumes a paused safe orbital
coast and selects the highest configured warp rate. It uses the existing burn
stop and encounter/terrain protections, returning to 1× fifteen seconds before
the calculated burn start (half the estimated burn duration before the node).
The button requires a nonzero maneuver, throttle off and a settled spacecraft
in a safe coast; otherwise it leaves the flight and warp selection unchanged
and explains the reason. It does not ignite or steer the engine.

Orbital paths brighten in their forward direction: dim near the spacecraft (or
the start of a maneuver forecast), bright farther into the future. On a closed
orbit, the brightest end lies just before the path wraps behind the spacecraft. The blue
osculating orbit starts at the spacecraft rather than at periapsis. Green coast
and orange maneuver forecasts fade by their prediction timestamps. Ordinary
bound coast forecasts show at most one revolution so overlapping laps do not
hide the direction gradient. Bound maneuver forecasts also stop after one lap;
encounter forecasts retain their longer horizon.

Earth retains a 6,371 km reference radius and its existing gravitational parameter. The Moon has a 1,737.4 km reference radius, a 384,400 km mean orbital distance, and a gravitational parameter of 4.9048695 × 10¹² m³/s². These agree in scale with NASA's [Moon Fact Sheet](https://nssdc.gsfc.nasa.gov/planetary/factsheet/moonfact.html). The game uses a prescribed circular, synchronously rotating mean lunar orbit, with an illustrative plane and initial phase; it does not provide a date-calibrated ephemeris, lunar eccentricity, mascons, or an N-body solar system.

Gravity switches between Earth and Moon at the 66,183 km lunar sphere-of-influence boundary. Position and velocity stay continuous in the same double-precision Earth-rotating coordinate frame. Inside the lunar region, gravity is Moon-relative and includes the prescribed Moon-center acceleration. The patched gravity acceleration is discontinuous at the boundary, as expected for this KSP-style approximation. Spacecraft do not also receive a second simultaneous Earth/Moon gravity force.

The map supports primary-body, spacecraft, Moon, Earth and system focus. Earth/Moon labels are clickable. Apsides, eccentricity, period, speed, navball markers and maneuver nodes use the current primary body's relative inertial state. The system view retains real scale; use **FOCUS MOON** or the wheel to inspect the Moon closely.

The world map keeps geographic north at screen top while you orbit and zoom. Earth/system views use Earth's rotation axis; Moon focus uses the Moon's rotation axis. It no longer inherits the spacecraft's changing local horizon. Pitch stops short of the poles to avoid an upside-down view.

Earth's orbital map uses the included NASA Blue Marble texture with linear sampling and a wrapped longitude/clamped latitude seam, a smoother globe, a soft day/night transition with readable night-side terrain, and a thin atmospheric rim. Map shading avoids the previous reflective washout. The image is a static base map, not live weather. Main spacecraft shells have closed end bulkheads; the capsule has an inner liner behind its glazing, and engine bells have inner walls and recessed closed throats rather than open tubes through the vehicle.

The detailed exterior uses capsule shell seams, framed glazing, a conformal hatch outline, heat-shield joints, service-stage radiator panels and reaction-control hardware, layered engine hardware, an open structural adapter, corrugated transfer-stage skirts and faceted lunar thermal blankets. Landing struts have telescoping sections, collars, attachment joints and ribbed footpads; their endpoints follow the existing leg animation. Static hardware is batched by stage and finish and shares geometry with separated stages. These are visual details inspired by NASA's [Apollo exterior and system diagrams](https://www.nasa.gov/history/diagrams/apollo.html), adapted to the fictional direct-return vehicle; decorative radiator/RCS hardware does not add new simulated subsystems.

Add a node and drag its position and P/R/N handles. **PLAN LUNAR TRANSFER** is an Earth-orbit planning aid: it uses a Lambert departure estimate and refines the approach offset with the same patched-conic prediction used in flight, targeting an approximately 120 km lunar approach. It does not fire the engine. The map shows burn-center ETA, estimated burn duration, an approximate start countdown, available delta-v, predicted SOI transitions, and atmosphere/surface warnings. The live trajectory samples up to seven simulated days, with bounded resolution; closest approaches are estimates.

The dedicated **MANEUVER EDITOR** separates orbit information from burn editing. Click the blue current orbit to place a node, or use **ADD NODE**. Drag the node along its orbit; six signed P/R/N handles adjust prograde/retrograde, radial out/in and normal/antinormal components. The matching numeric rows offer −/+ buttons. **Δv STEP** cycles 1, 10, 100 and 0.1 m/s; **TIME STEP** cycles 60, 10, 1 and 300 seconds for the earlier/later buttons. Timing uses the current primary's Kepler trajectory rather than treating anomaly as uniform elapsed time. **ZERO Δv** retains the node position, **REMOVE NODE** deletes it, and **ALIGN SAS TO NODE** selects maneuver pointing while leaving the burn manual. The editor shows total Δv, node ETA, estimated start/duration, planned apsides and fuel/entry warnings. Markers are hidden under the control panels so they cannot cover editor buttons.

Earth's globe uses an east-positive geographic mapping consistent with the spin axis and north-up camera. Saudi Arabia appears east of Africa; the previous mirrored longitude mapping is corrected. Globe triangle winding is adjusted with that mapping so surfaces remain outward-facing.

The Moon uses the December 2025 NASA SVS/LROC color mosaic at 4096 × 2048, providing recognizable maria, crater ejecta and highlands. The globe has a denser mesh and smooth radial shading, and nearby terrain uses the same geographically anchored texture with restrained regolith variation instead of broad moving color bands. Lunar north is body −Z and the Earth-facing prime meridian is body −Y. Color imagery improves appearance; the landable relief and contacts remain procedural. Source and conversion details are in [the lunar image credit](../assets/MOON_SOURCE.md), with the original data products at [NASA's CGI Moon Kit](https://svs.gsfc.nasa.gov/4720/).

Use maneuver SAS to point for a planned burn, begin near the displayed start time, and control the burn manually. Recheck the live trajectory after the burn and make corrections. The node is an **impulse plan**; it does not model the complete finite burn, consume the planned delta-v automatically, or execute a mission. Enter lunar orbit using the transfer stage, discard that stage, and fly descent with the landing stage. After touchdown, cut throttle and separate the landing stage. Use the return stage for ascent and Earth-return maneuvers. Separate the return stage before atmospheric entry and present the capsule's broad heat shield to the airflow (retrograde attitude).

## Capsule aerodynamics and recovery

Atmospheric forces depend on local air density, air-relative velocity, Mach, attitude, area and angular rates. The capsule uses a 12.6 m² reference area, stronger broad-heat-shield drag than nose-first drag, a modest transonic increment, and limited angle-dependent lift. Lift is perpendicular to airflow. A pressure-center offset and aerodynamic rate damping produce actual rigid-body moments. The attached stages have larger drag areas and different inertia; they are not treated as the reentry capsule. Forces and heating are exactly zero at zero density, including lunar flight.

The heating proxy scales with square-root density and velocity cubed. A finite shield allowance and an exposed-side heating threshold can destroy the spacecraft. The broad heat shield is at body +Z, opposite the normal main-thrust direction. Heating, shield limits, coefficients and parachute behavior are tunable approximations rather than validated Apollo aerothermal data.

Armed parachutes deploy only on the separated capsule in Earth's atmosphere, below 8 km and below 350 m/s. Inflation takes five simulated seconds; the full aerodynamic model uses a 750 m² equivalent area. Low-speed capsule contact can complete recovery; hard contact destroys it. Lunar landing uses four terrain-normal spring/damper contacts, friction, individual compression, slope/impact limits and mechanical leg animation. Capsule/body impact is an approximate clearance test, not a triangle-exact structural collision solver.

## Warp and visual precision

Warp settings accept rates up to **100,000×**, and older settings gain 10,000× and 100,000× choices without changing their existing rates or timestep. Powered/controlled spacecraft flight remains limited to 4×, with 1× near terrain or after landing. High warp is allowed only during quiet coast above the body-specific safety altitude. It stops at primary-body transitions, before atmospheric/terrain entry, and before a planned burn's estimated start window.

Coasts use universal-variable Kepler propagation in bounded intervals, with additional subdivision near SOI boundaries. They preserve configured simulation-time behavior instead of catching up to wall time. Spacecraft-local subtraction happens in double precision before GPU float conversion, including on the lunar surface. Terrain uses shared procedural crater heights for rendering and contacts, nested local LOD rings, continuous longitude seams and outward normals. Abandoned landed stages remain fixed to the rotating Moon. Free stages inherit motion and use approximate conics, with visibility suppressed below the reference surface and after one simulated day; they are not independently integrated colliding rigid bodies.

## Validation scope

`cargo test --locked` covers orbital propagation, SOI continuity, warp guards, Moon-relative maneuver timing/plotting, capsule forces/moments/vacuum behavior, stage accounting, lunar contacts, terrain continuity/mesh normals, controls and parachute recovery alongside the existing aircraft/combat regressions. `--study-lunar` records budgets, an impulsive lunar transfer forecast, one-day Earth/Moon coasts, a controlled 1.5 km landing approach, parachute recovery from 5 km, and high-speed entry/recovery from 120 km in `tests/lunar-study.json`.

The landing study uses a diagnostic feedback controller, not a gameplay autopilot. These separate numerical checks do not establish that a complete manually flown Earth–Moon–Earth mission has been performed, or validate real spacecraft performance. Native captures inspect model assembly, separation, terrain, map controls, cockpit displays and reentry visuals.
