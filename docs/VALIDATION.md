# Validation record — October 14–16, 2026

## October 19: remove exposed F-22 nozzle actuators

Removed the decorative nozzle cylinders, rods, clevis feet and cross-pins,
including their presentation-only component and transform updates. Nozzle
flaps, hinges, gray finish, exhaust and physical TVC remain unchanged.
Removed the obsolete cylinder-clearance test; retained the flap hinge and
aperture sweeps. **352 tests pass**. Packaging, a baseline roll diagnostic
comparison and native top/side/vectoring underside captures are recorded in
`tests/f22-no-actuators/delivery.json`. This supersedes the visible actuator
hardware shown in the earlier nozzle blend and finish records.

## October 19: F-22 nozzle finish matched to the airframe

See [F22_NOZZLE_FINISH.md](F22_NOZZLE_FINISH.md). **353 tests pass**.
Fixed housings share the fuselage material and body-space camouflage; moving
flaps use subtler cool-gray heat wear. Native top, side and vectoring underside
views check the finish. Final packaging and unchanged authoritative flight
diagnostics are recorded in `tests/f22-nozzle-finish/delivery.json`.

## October 19: F-22 nozzle housing blend and actuator clearance

See [F22_NOZZLE_BLEND.md](F22_NOZZLE_BLEND.md). **353 tests pass**.
The fixed forward housings conform to the fuselage and loft into the nozzle.
Shorter actuator cylinders attach to raised clevises on the moving sections,
clearing both panels through the sampled command range. Native top, side,
vector-limit and underside views check the joins and actuator placement.
Physics and roll/FBW settings are unchanged. Delivery evidence is in
`tests/f22-nozzle-blend/delivery.json`.

## October 19: triangular nozzle tip moved aft

See [F22_AFT_NOZZLE_TIP.md](F22_AFT_NOZZLE_TIP.md). **351 tests pass**.
The moving upper/lower flaps now have a rectangular forward section and a
triangular aft tip. Edge trim follows the corrected perimeter. Tests explicitly
check aft direction, full-tip aperture and transformed stabilator clearance.
Only nozzle presentation source changes; roll/FBW and flight physics remain
unchanged. Delivery record: `tests/f22-aft-nozzle-tip/delivery.json`.

## October 19: F-22 roll-sign correction and stronger FBW response

See [F22_ROLL_CONTROL_FIX.md](F22_ROLL_CONTROL_FIX.md). **351 tests pass**.
The previous F-22 direct-mode exception and opposing stabilator/nozzle roll
mixes caused a speed-dependent reversal and wasted FBW authority. The signs
now agree, and the low-AoA commanded roll rate rises from about 89 to 160°/s.
Twelve speed/key/mode regressions check Q/E at 80, 160 and 240 m/s, FBW on/off.
High-AoA pull and assisted landing remain controlled. Other aircraft's fleet
diagnostics remain exact. Delivery: `tests/f22-roll-direction/delivery.json`.

## October 19: F-22 differential TVC and stronger stabilators

See [F22_DIFFERENTIAL_TVC.md](F22_DIFFERENTIAL_TVC.md). **349 tests pass**.
The independent pitch-plane nozzles now mix pitch and roll, generating torque
through their actual engine force offsets. FBW allocation includes differential
vectoring, saturation and current stabilator effectiveness. Both stabilators
produce 1.30× their previous modeled loads. Low-speed roll, failures, takeoff,
landing and high-AoA diagnostics pass; other aircraft's fleet-study results
remain exactly equal to the baseline. Native roll captures use simulated
states. Installed-build evidence is in `tests/f22-differential-tvc/delivery.json`.

## October 19: weathered gray F-22 panels with triangular outlines

See [F22_GRAY_NOZZLE_PANELS.md](F22_GRAY_NOZZLE_PANELS.md). **346 tests pass**.
The aft panels now have actual rectangular-plus-triangular perimeters instead
of square slabs with raised decorations. The outer covers and fixed cheeks
use a lighter weathered gray metallic texture; the recessed engine interior
stays dark. Shared hinge motion and authoritative physics remain unchanged.
Exact runtime mesh previews and native captures check geometry and material
appearance. Installed-build evidence is in `tests/f22-gray-panels/delivery.json`.

## October 19: F-22 separately articulated nozzle sections

See [F22_ARTICULATED_NOZZLE.md](F22_ARTICULATED_NOZZLE.md). **345 tests pass**.
Broad rectangular exit flaps replace the previous three-triangle decorations;
larger forward triangular shoulders and aft flaps articulate separately at
connected hinges. Fixed sidewalls remain attached to the airframe. A sweep
checks hinge continuity and aperture clearance. Exact runtime mesh previews
and repeated native captures check geometry and appearance. Public references
guide the exterior; hinge dimensions and angle schedules remain inferred.
Delivery evidence is in `tests/f22-articulated-nozzle/delivery.json`.

## October 19: F-22 fixed-sidewall nozzle and square/triangular flap revision

See [F22_NOZZLE_PANELS.md](F22_NOZZLE_PANELS.md). **344 tests pass**.
The lateral sidewalls, duct and burner remain fixed while the upper/lower
flaps combine TVC and nozzle opening. Raised triangular external panels,
weathered metal and removal of the repeated liner bars address the requested
appearance. A GPU-free export feeds the exact runtime meshes to Matplotlib
for faster shape/animation checks; native captures validate materials and
airframe integration. The installed-build identity and numerical equivalence
are recorded in `tests/f22-nozzle-panels/delivery.json`.

## October 19: F-22 tire, gear-deployment and internal-glow fixes

See [F22_GEAR_NOZZLE_FIX.md](F22_GEAR_NOZZLE_FIX.md). **343 tests pass**,
including new outward-normal/watertight tire, continuous gear-sequence,
closed-door clearance and recessed-glow checks. Native captures include the
open bay before extension, early movement, intermediate/deployed poses, main
and nose gear close-ups, recessed glow at idle, full afterburner and shutdown.
The signed release and exact simulation-equivalence/capture identities are
recorded in `tests/f22-gear-nozzle/delivery.json`. This is an appearance-only
follow-up; physical contact positions at full deployment remain unchanged.

## October 18–19: F-22 appearance and animation rebuild

See [F22_VISUAL_REBUILD.md](F22_VISUAL_REBUILD.md) for photographic references,
exhaust/TVC geometry, forward nozzle placement, canopy/airframe fit, coating,
landing gear, HDR exhaust/shock cells and native captures. **339 tests pass**. The F-22 and fleet
diagnostic outputs exactly match the preserved starting executable. No flight
model changes or FPS improvement are claimed. Final installed-build identity,
signature and capture coverage are recorded in `tests/f22-visual/delivery.json`.

## October 18: dedicated F-22 refinement

See [F22_RAPTOR.md](F22_RAPTOR.md) for the exterior, cockpit, swept-hinge,
wing-profile, FBW/vector allocation and loadout changes. **331 tests pass**.
The fixed-step study reaches 49.80° AoA with FBW in the low-speed pull, then
unloads to 4.17° at 20 seconds. Full approach guidance reaches the paved runway
at 2.30 m/s sink, 0.68 m off centerline. Level acceleration, a controlled 40°
bank, direct-control departures, nozzle failure, surface jams/detachment and
loadout changes are checked. These are simulator results, not real-aircraft
calibration. Identical-seed outputs for the four other aircraft match the prior
fleet study exactly; see `tests/f22/equivalence-result.json`.

Thirteen development render checks have their own source/executable manifest in
`tests/fleet/f22-accepted`; packaged-app cockpit/exterior checks after the final
display-heading, wheel-zoom and false stall-warning fixes are in
`tests/fleet/f22-release-final`. The final optimized build took 2m 50s; both
packaged-release studies exactly match their development equivalents. Strict/deep
ad-hoc signature verification passes.
Final build, signature, native interaction results and artifact identities are
recorded in `tests/f22/delivery.json`.
No new FPS benchmark is claimed from the screenshots.

## October 18: selectable four-aircraft fleet

The [fleet guide](FLEET.md) documents the player-selectable Su-27SM, F-35B,
F-22A and F/A-18E alongside Gripen, unique procedural geometry/cockpits,
aircraft-specific six-degree-of-freedom loads, failures, STOVL and vectoring.
AI remains Gripen; spacecraft scenarios remain separate. All **320 tests pass**.
Fixed-step flight studies remain finite/airborne for all five types; the Su-27
post-stall trial peaks at 99.89° AoA and returns below 25° within 12 seconds.
The coupled hover/fan-failure test passes. Geometry, inventory, hangar, damage
and independent control-authority regressions pass. These are game-model tests,
not validated military-aircraft performance.

Native images and their source/executable manifest are in
`tests/fleet/native-delivery`. The optimized release build completes in 3m 11s;
the packaged app passes strict/deep ad-hoc signature verification. Artifact
identity and final interaction results are recorded in `tests/fleet/delivery.json`.
No new performance benchmark or FPS improvement is claimed for this feature pass.

## October 18: near-terrain and forward-cockpit rendering

See [TERRAIN_PERFORMANCE.md](TERRAIN_PERFORMANCE.md) for the focused forest pass,
accepted/rejected optimizations, measured limits and background-load caveats.
All 307 application tests pass. Native comparisons cover forests, the forward
cockpit and Earth at 12 km/60 km; forest output is pixel-identical. The final
release app is packaged and passes strict ad-hoc signature verification.
`tests/terrain-performance/delivery.json` records executable hashes and evidence.
Fullscreen 60 FPS remains unmet; graphics resolution, tree population and
authoritative simulation were preserved.

## October 17: space Sun, Nordic compositing and cockpit gun director

See [SPACE_LIGHTING_AND_GUNSIGHT.md](SPACE_LIGHTING_AND_GUNSIGHT.md) for the
visible solar proxy, emissive/shadow correction, single-surface regional detail
and two-ring cockpit director. All 304 application tests pass; native images
cover solar visibility/occultation, orbital Sweden and maps, and normal/simplified
cockpit views. Final packaged build evidence is in
`tests/space-lighting/delivery.json`. Previous optimization measurements remain
historical; no new FPS claim is made from these screenshots.

## October 17: completed cockpit/atmosphere follow-up

See [ATMOSPHERIC_OPTIMIZATION.md](ATMOSPHERIC_OPTIMIZATION.md) for the final
focused pass: 299 application tests, ten engine tests, nine equivalent JSON
diagnostics and two identical AI CSVs; six paired release workloads; 100 native
transitions with 1,200 consecutive frames per reference/candidate configuration;
and a passed 600-second atmospheric/cockpit stability run. The first-frame terrain
spawn corruption was reproduced and fixed. The accepted build is packaged,
ad-hoc signed, strict/deep verified and its native window confirmed. Detailed
results, caveats and remaining p95 misses are in the report. Earlier status
paragraphs below describe the prior pass, not the current installed build.

## October 17: optimization results and user-requested stop

Subsequent deployment on user request: installed the preserved optimized
candidate, verified all 236 source/asset hashes, stripped/ad-hoc signed the app,
passed strict/deep signature verification and confirmed the native window after
launch. The prior executable is backed up; `installed-build.json` records hashes.
No further benchmark runs were performed. Earlier packaging status below is
the state at initial report delivery.

See [OPTIMIZATION_REPORT.md](OPTIMIZATION_REPORT.md) for the completed results:
297 application tests, eight engine tests, three harness tests, nine matching
diagnostic JSONs and two byte-identical AI CSVs; ten complete render comparisons;
and a passed 1,800-second mixed-workload soak. Remaining diagnostics were stopped
at the user's request. Full visual/control sign-off and packaging are unfinished;
the installed app has not been replaced. The report distinguishes measured gains
from the still-unmet 60 FPS target.

## October 16: continuous Earth appearance and warp-to-maneuver control

Earth now shares its surface shading between flight and map views. The base
NASA July composite is 8192 × 4096, with a 4096 × 3511 Nordic layer feathered into
the globe and a separate 2048 × 1024 historical NASA cloud composite. The globe
uses a 768 × 384 grid. Three atmosphere shells use the actual camera position
for the limb angle, with soft outer edges and restrained column haze. Initial
captures exposed hard shell bands, blurred regional geography, a reflective
local-water triangle and a vehicle-lighting change at 100 km; subsequent passes
correct those issues rather than merely changing the orbital map colors.

Terrain and local reflective water fade together over 18–60 km; stars emerge
over 45–100 km. Native sky scattering and both the source and generated
environment-map light fade over 45–95 km, before atmosphere removal at 100 km.
Bevy 0.18.1 initializes `GeneratedEnvironmentMapLight.intensity` from the source
once, so the simulator explicitly synchronizes both intensities. The visual
weights are stateless functions of altitude and apply in either travel direction.
Ground contact and spacecraft/aircraft force models are unchanged. Clouds are
historical imagery on a shell, not volumetric weather; terrain is procedural.

The node editor adds **WARP TO MANEUVER · BURN −15s**. It requires a nonzero
maneuver and safe settled orbital coast, resumes pause only on success, selects
the highest configured warp and uses the existing burn-stop/encounter/entry
protections. The stop is fifteen seconds before the estimated centered burn
starts; this does not ignite or steer the engine. Invalid requests keep the
previous pause and warp selection and show a reason.

All **291 tests pass, 0 failures** (`tests/earth-continuity-tests.log`), including
transition bounds/continuity, cloud-alpha conversion, regional mesh winding and
seam weights, warp request safety and the existing planned-burn stopping
regression. Native captures inspect 12/60/99/101/220 km, a day-side map and the
maneuver editor. These are paused renderer poses, not a manually flown ascent
or a matched performance benchmark. Source image processing and credits are in
`assets/EARTH_SOURCE.md`; temporary large NASA source downloads are removed.

The final 99/101 km pair shows consistent vehicle lighting across the former
boundary. The fixed vehicle/background ROI (440,350)–(850,675) differs by a mean
2.28/2.42/2.40 RGB levels out of 255 (`tests/earth-boundary-image-check.json`);
it includes legitimately moving background geometry, not only hull pixels.
The final 60 km capture confirms the local reflective-water triangle is gone.
The ordinary low-altitude approach capture (`tests/visual-dry.png`) and day/map
captures were inspected as well. The lunar mission diagnostic still completes
(`tests/earth-continuity-lunar.log`).

Formatting and strict/deep app signature verification pass. The final signed
development rebuild took **1m 50s** (`tests/earth-continuity-build.log`).
Executable SHA-256:
`b3a5b54720351a5595997ebfa2fac029800c024eca52a10e537a79193f0ea2bf`.
Earth base / Nordic / cloud asset SHA-256 respectively:
`0e9da40eeee2d2dd56f5a0e8c0b98487aa2a9c6bc4000aa3047d0e0e68cfcc2d`,
`7cd341035092f8db5075a34160d4da67171a9b3a470350ddf67544c630b7f4aa`,
`daddaad84d7a33bbbc86cdda3f591099f57cee8607b7bcf3b67eb7e4f7a1c793`.

## October 16 follow-up: reverse orbital brightness direction

At the user's request, orbit and maneuver prediction opacity now increases from
10% at the start to 100% at the future end. For a closed orbit the brightest end
is immediately before wrapping behind the spacecraft. The map legend and guide
now say dim → bright in forward time. The focused direction/fade regression
passes (`tests/orbit-reverse-fade-tests.log`), checking monotonic brightening and
both endpoint opacities. The development app is rebuilt and signed.

## October 16: forward-fading orbital paths and lunar-plane starting orbit

Blue osculating paths now start at the spacecraft's current anomaly and fade
from full opacity to 10% along the forward arc. Green/orange patched-conic
forecasts fade using their prediction timestamps. Bound predictions without
encounters stop after one revolution, including maneuver forecasts, so repeated
laps cannot wash out the gradient. A map legend explains bright → dim as forward
in time. The same renderer handles Earth and Moon orbits.

The default 220 km circular Earth orbit is now prograde and coplanar with the
simulated lunar ephemeris, with the vehicle oriented along inertial orbital
velocity. The transfer planner's upper offset bracket expands to 24,000 km to
retain its safe-approach solution for this coplanar geometry. The lunar study
measures **0 degrees initial plane error**, approximately **0 m/s normal TLI
delta-v**, **3,244.845 m/s total planned TLI delta-v**, and a **120.091 km lunar
approach altitude**, with a predicted Moon SOI encounter. These are instantaneous
patched-conic planning results; finite burns remain manual.

All **287 tests pass, 0 failures** (`tests/orbit-direction-tests.log`), including
current-position/forward-path direction for Earth and Moon, bounded monotonic
fade, initial circularity/plane/orientation, and the lunar-transfer encounter and
fuel budget. The full lunar diagnostic completes (`tests/orbit-direction-lunar.log`,
`tests/lunar-study.json`), retaining landing/recovery and daily coast checks.
Native captures `tests/visual-map.png` and `tests/visual-lunar-earth-map.png`
inspect maneuver/coast gradients and map legend layout; they are posed renderer
captures, not evidence of a manually flown lunar mission.

Formatting and strict/deep signature verification pass. The signed development
rebuild took **18.42 s** (`tests/orbit-direction-build.log`). Executable SHA-256:
`ee1ac9e159f9c54e3fa647d73cd4f6f1d245259d0da495ba465c00dc2f92989d`.

## October 16: IR missile radar-datalink fallback

Shared missile guidance now prefers IR measurements but falls back to a valid
launcher radar track if the IR seeker cannot detect the target. The existing
radar support geometry, 20 km range, terrain masking, notching and caller-provided
track/system availability remain authoritative. Reacquisition returns to IR;
loss of both sources propagates memory without unseen target-state updates.
Flare contacts cannot receive aircraft radar support. Launch acquisition,
motor/energy coefficients and autonomous radar activation rules are unchanged.
Missile telemetry distinguishes `IR / RADAR DATALINK`.

All **285 tests pass, 0 failures** (`tests/ir-datalink-tests.log`). Added tests
cover glare fallback/reacquisition, seeker range/gimbal loss, unsupported memory
and ballistic transition, no hidden-target update, support cone/range/terrain/
notch rejection, and decoy/dead-target rejection. The missile study records
**8.792 s** of fallback in its sun-glare engagement, which intercepts; dry and
afterburner engagements remain entirely on IR (`tests/ir-datalink-study.log`,
`tests/missile-gnc-study.json`). The integrated air-combat study completes
successfully (`tests/ir-datalink-combat.log`). These are controlled simulation
checks, not real missile hardware validation.

Formatting and strict/deep signature checks pass. The development app rebuild
took **8.97 s** (`tests/ir-datalink-build.log`). Executable SHA-256:
`1b3ff4ed0c0aaa11ed5fd11eb8f6bf0389fdcb31a08f4517a0ac30db2bd03d41`.

## October 16: energy-saving long-range missile trajectories

Shared player/AI LR-AAM guidance now replaces the abrupt altitude bias with a
bounded climb/cruise/descent demand and smooth terminal PN blending. Motor,
propellant, drag, sensor limits and available fin authority are unchanged. The
path follows filtered estimates, including in track memory; short-range weapons
retain their direct trajectory. Missile telemetry reports path altitude/blend and
has moved below the aircraft damage schematic to avoid the overlap found in the
first native capture.

All **281 tests pass, 0 failures** (`tests/missile-loft-tests.log`). New regressions
cover matched drag/terminal energy gains, terminal PN equivalence, unchanged
short-range guidance, unseen-target altitude isolation and 120/240 Hz trajectory
agreement within 1%. Formatting and strict/deep signature verification pass.
The signed development rebuild took **10.42 s** (`tests/missile-loft-build.log`).
Executable SHA-256: `52e2141fd77e78d33dd459540b0f78592095674b80d8eafcabb87568ae9207ed`.

`tests/missile-gnc-study.json` records 24 matched previous/new trajectory runs.
At 100 km head-on range and 11 km launch altitude, encounter speed improves
974 → 1,344 m/s and drag energy loss falls 1.402 → 0.981 MJ/kg; peak altitude
is 23.2 km. At 3 km launch altitude the corresponding encounter speed improves
351 → 635 m/s. Both profiles intercept 10/12 geometries; the 70/100 km
low-altitude retreating targets still escape, with smaller closest approaches.
Original seven seeker-study hit/miss outcomes match the saved pre-edit baseline
(`tests/missile-guidance-baseline.log`). All 24 integrated BVR scenarios remain
finite (`tests/missile-loft-bvr.log`, `tests/bvr-study.json`). BVR weapon trials
intercept steady 70 km head-on/cold and 100 km head-on targets; the unsupported
turning target escapes. These are deterministic game-model studies with a
kinematic target/player diagnostic, not calibrated weapons or manually flown
combat validation. `tests/visual-combat-follow.png` is a native paused renderer
capture used to inspect missile-camera telemetry placement.

## October 16: lunar appearance, maneuver editor and geographic handedness

The lunar globe now uses a 4096 × 2048 JPEG derivative of NASA SVS's December 2025 LROC color mosaic, with a 512 × 256 globe grid, smooth radial normals, matte shading and explicit lunar pole/prime-meridian UV mapping. Local landable terrain shares the color map and uses restrained world-anchored regolith modulation instead of anchor-relative color waves. Geometric relief and terrain contacts remain procedural. NASA credit, source URL and conversion details are in `assets/MOON_SOURCE.md`; image SHA-256 is `dd119fa9a75101700d36c0e2da161f156714214cc85a5b130c2975a10e892949`.

The maneuver editor separates the orbit overview, view controls and burn-editing card. It adds six signed P/R/N handles, component −/+ buttons, selectable 0.1/1/10/100 m/s steps, selectable 1/10/60/300 s timing steps, earlier/later controls, zero Δv, node removal and maneuver-SAS selection. Orbit clicks can place a node; captured node drags suppress left-button camera dragging. Map markers are hidden beneath the side panels. Planned apsides, total Δv, burn ETA/start/duration and fuel/entry warnings remain visible. Timing edits use Kepler propagation, and orbit picking reuses the orbital/render frame within its search.

Earth's previous east/west reversal came from the sign of longitude in the geographic coordinate mapping relative to the north/spin convention. The mapping and globe triangle winding are now consistent. Regression checks verify positive longitude follows east, positive latitude follows north, Saudi Arabia projects right/east of Africa in a north-up view, and globe triangles face outward.

All **276 tests pass, 0 failures**, with no compiler warnings (`tests/moon-editor-tests.log`); formatting passes. The fast development app rebuilt in **16.20 s** and was packaged/ad hoc signed (`tests/moon-editor-build.log`); strict/deep signature verification passes (`tests/moon-editor-signature.log`). Executable SHA-256 is `79a57cf86d4d0126e785e5cfc6f3c29c1a1ea777c17b837ac6b310fb6237834b`.

Native captures inspect the textured Moon with a populated node editor (`tests/visual-lunar-node-map.png`), the Africa/Arabia orientation (`tests/visual-lunar-earth-map-geography.png`), and lunar surface/landing appearance (`tests/visual-lunar-ground.png`). Final capture logs use `tests/moon-editor-final-*.log`. These are ordinary renderer captures of paused or diagnostically advanced states; no manual full mission or matched FPS benchmark is claimed. Tests cover exact timing changes around Earth/Moon, signed handle pairs, component clamps/reset/SAS actions, a precision-button system invocation without a cursor, and drag/release capture.

## October 16: solid spacecraft, Earth map, extending legs and attitude selection

Main capsule/service/descent/transfer shells now have closed end bulkheads. The capsule has an inward-facing inner liner matching its glazing cutout. Engine bells have inner walls, closed recessed throats and chamber floors beyond the tank bulkheads. Axial sightline regressions check the closed shells and bell throat. Native underside inspection exposed a gold bulkhead intersecting the descent chambers; additional recessed floors remove that visible intersection. Isolated return-stage inspection hides posed debris so it does not obscure the engine being checked.

Earth's map retains the included NASA Blue Marble geography, with dark blue water pixels lifted for readability, wrapped/clamped linear texture sampling, a smoother globe, diffuse day/night color modulation instead of reflective map washout, and a thin atmospheric rim. The night-side floor is a cartographic readability choice. Normal flight retains lit surface rendering; this is not a weather or radiative-transfer model.

Leg deployment takes three simulated seconds with a smooth foot path and four overlapping nested sleeves per main strut. A dedicated button supplements G, the panel reports deployment percentage, lunar orbit starts stowed, and landing approach starts deployed. Rendering and terrain contacts use the same animated leg anchors; spring compression affects the rendered pads. Screenshots at 0, 1.5 and 3 simulated seconds show stowed, 50% and fully deployed states. A regression checks sleeve overlap/endpoints throughout deployment.

Direct attitude buttons enable SAS and highlight rate hold, prograde, retrograde, maneuver, radial out/in and normal +/−. F7 and the existing mode button cycle all eight modes. Tests verify Earth/Moon-relative target axes, actual RCS-driven alignment and fuel usage for all seven pointing modes, safe rate damping when no maneuver direction exists, and persistent button/leg command state.

All **271 tests pass, 0 failures** (`tests/spacecraft-solid-tests.log`); formatting and strict/deep app signature checks pass. The signed bundle uses the fast development profile (`tests/spacecraft-solid-build.log`, `tests/spacecraft-solid-signature.log`). Native captures inspected include `visual-lunar-earth-map.png`, `visual-lunar-solid-under.png`, `visual-lunar-solid-top.png`, `visual-lunar-solid-return.png`, `visual-lunar-solid-transfer.png`, `visual-capsule-solid-under.png`, `visual-lunar-legs-stowed.png`, `visual-lunar-legs-half.png`, and `visual-lunar-legs-extended.png` under `tests/`. These are ordinary renderer captures of posed or diagnostically advanced states. They do not establish a manually flown mission or a matched FPS benchmark.

The final Moon-facing cockpit inspection (`tests/visual-lunar-cockpit-window-orbit.png`) confirms the exterior remains visible through the glazing after adding the inner liner. The last development compile took **12.71 s**. Delivered executable SHA-256: `bd7d5e40a9e4bdacca0fd6a437c9ef989f4a52ec1d6be14c180ce636ab9ba2ab`.

## October 16: north-up world map and detailed spacecraft exterior

The orbital/world map now uses the selected body's geographic north axis as camera up. Earth and system views use Earth's rotation axis; Moon focus uses the lunar rotation axis. Orbit dragging changes azimuth/elevation without introducing roll, and the existing pitch limit avoids pole flips. A regression checks axis selection and screen-up/right projections across Earth/Moon scenarios, focus choices, time and camera angles.

The spacecraft exterior now includes capsule seams, glazing frames, a conformal hatch outline, heat-shield joints, radiator/louver panels, reaction-control hardware, engine supports and cooling rings, thermal-blanket facets, adapter trusses, skirt corrugations, feed lines and roll-pattern markings. Landing shafts have telescoping profiles and collars, with ribbed footpads. Native ground inspection exposed the old leg shoulders outside the octagonal hull; their attachment points and clevises now meet the hull. Static details are combined into at most twenty stage/material batches, share mesh assets with debris, and retain stage visibility. Mesh regressions check finite positions/normals, valid indices and bounded extents. This is exterior modeling, without new simulated subsystems or changed propulsion/aerodynamic coefficients.

The expanded regression suite passes **266 tests, 0 failures** (`tests/spacecraft-detail-tests.log`). Native full-stack, landing and map renders are captured in `tests/visual-lunar-stack.png`, `tests/visual-lunar-ground.png` and `tests/visual-lunar-map.png`; these inspect posed/diagnostic states, not a manually flown mission or a matched FPS benchmark. The visual reference is NASA's [Apollo drawings and diagrams](https://www.nasa.gov/history/diagrams/apollo.html), adapted to this fictional larger direct-return architecture.

The final optimized build completed in **12m 36s** with no compiler warnings and was packaged/ad hoc signed (`tests/spacecraft-detail-release-build.log`). Deep/strict signature verification passes (`tests/spacecraft-detail-signature.log`). The delivered executable SHA-256 is `a49b9f6a577b808c588391af708a84e4f52678790f11df4caa0cbffb41940d0a`. Final packaged ground/map renders are logged in `tests/spacecraft-detail-final-ground.log` and `tests/spacecraft-detail-final-map.log`. The fast development profile remains available through `./build-mac.sh dev`.

## October 15–16: Earth–Moon spacecraft, capsule aerodynamics and direct return

The expanded source passes **264 tests, 0 failures** in `tests/lunar-tests.log`; formatting checks pass. The suite retains the aircraft/combat regressions and adds finite staged propellant/mass/delta-v accounting, capsule lift/drag/restoring moments and zero-density loads, Earth/Moon Kepler coasts, continuous state across SOI changes, atmospheric-entry stops, 100,000× rate limits, planned-burn warp stops, Moon-relative maneuver timing and plotted trajectories, terrain longitude/pole continuity, mesh/contact height agreement and normals, spring-leg touchdown/hard impact, leg geometry/extension, staging/parachute controls, and capsule entry/recovery.

`--study-lunar` writes `tests/lunar-study.json`. Measured game-model results:

| Check | Result |
|---|---|
| Initial finite-fuel stack | 243,400 kg |
| Ideal transfer / descent / return delta-v | 4,949.29 / 2,188.40 / 3,640.68 m/s |
| Suggested impulsive lunar injection | 3,284.60 m/s; departure-center ETA 339.01 s |
| Patched-conic lunar approach forecast | Enters lunar SOI; approximately 120.02 km minimum altitude |
| One-day Earth coast | Apsis changes below 0.000002 m |
| One-day lunar coast | Apsis changes below 0.000006 m |
| Controlled 1.5 km lunar approach | Touchdown after 97.41 s; 30,281.71 kg descent propellant remaining; no crash |
| Capsule recovery from 5 km | Landed after 469.87 s; full parachute deployment; no crash |
| High-speed capsule entry from 120 km | Landed after 704.58 s; peak 12.32 g; peak heating proxy 2.89 MW/m²; shield fraction 0.8766 remaining |

The high-speed entry uses 10,800 m/s initial Earth-frame tangential speed and −1,100 m/s radial speed. Its load/heating values are conditional outputs from tunable coefficients and a heating proxy, not Apollo flight-data validation. The landing diagnostic has a feedback controller used only for testing. Transfer prediction is impulsive; a complete manually flown finite-burn Earth–Moon–Earth mission has **not** been performed in this validation pass. Extremely small coast errors measure consistency with the selected two-body model, not accuracy against lunar ephemerides or third-body perturbations.

Native development captures exposed and drove fixes for external cockpit protrusions, local terrain/globe overlap, the system-map far-clip distance, full/simplified panel placement, and predictions drawn in the wrong frame around the moving Moon. The finalized scene modes include `lunar-stack`, `lunar-orbit`, `lunar-ground`, `lunar-ascent`, `lunar-cockpit`, `lunar-map`, `lunar-transfer-map`, and `capsule-reentry`. These use the ordinary native renderer and posed or diagnostically advanced states. See [spaceflight](SPACEFLIGHT.md) for controls, reference sources, approximations and mission guidance.

The final optimized bundle rebuilt through `./build-mac.sh release` in **2m 45s** with no compiler warnings (`tests/lunar-release-build.log`), including ordinary-flight Moon visibility. Strict/deep code-signature verification passes (`tests/lunar-signature.log`). Its `--study-lunar` results reproduce the measurements above (`tests/lunar-study-release.log`). Optimized native renders were inspected for the full stack, lunar ascent, capsule reentry and Moon map; the final packaged map capture is recorded in `tests/lunar-native-final-map.log` and `tests/visual-lunar-map.png`. These are renderer inspections, not manual mouse/keyboard mission completion. The delivered executable SHA-256 is `cddf02b44b1c5af7b5094f7df0bcac9f3a32e420b65517169e7a690741b8248c`.

## October 15: timed performance follow-up

The frozen 39-module source passes **240 tests**
(`tests/perf40-stage6-tests.log`); its development build completes without
compiler warnings (`tests/perf40-stage6-build.log`). The detailed review,
unchanged fidelity settings and reproduction commands are in
[PERFORMANCE_40_MINUTE_PASS.md](PERFORMANCE_40_MINUTE_PASS.md).

Three alternating paired CPU comparisons against the executable saved at the
start of this follow-up produced these whole-process wall-time medians:

| Study | Before | After | Throughput ratio | Before range | After range |
| --- | ---: | ---: | ---: | ---: | ---: |
| Flight | 13.717 s | 9.552 s | 1.44× | 13.694–13.763 s | 9.509–11.929 s |
| Close combat | 2.698 s | 2.003 s | 1.35× | 2.696–2.735 s | 1.987–2.009 s |
| BVR | 22.421 s | 15.404 s | 1.46× | 22.055–22.475 s | 15.286–15.944 s |
| Existing collision/LOS workload | 0.377 s | 0.375 s | 1.01× | 0.344–0.392 s | 0.340–0.389 s |

All reported flight polars/maneuvers/step counts and the complete deterministic
combat/BVR reports match exactly across all runs. The unchanged focused
collision/visibility workload shows no meaningful additional improvement in
this pass. Both executables use the same development profile. No compiler,
test suite or renderer runs during the CPU comparisons; no user game instance
remains running during this final follow-up comparison. Variation and first-run
effects remain visible in the raw samples. These timings are distinct from FPS
and do not establish a release-build speedup. Binary hashes and all samples are
in `tests/perf40-cpu-comparison.json`.

The expanded AI equivalence pass also succeeds: **six flight cases and 28
combat scenarios**, including all BVR force sizes/difficulties and both mission
types, retain exactly matching physical JSON reports and complete CSV traces.
The combat matrix records **2,894,400 NPC physics steps** with finite states in
every scenario. Whole-process time for this single additional validation pass
is 71.09 s before and 61.72 s after; this one pair is a validation timing rather
than a repeated performance estimate. See `tests/perf40-ai-comparison.json` and
the saved `tests/perf40-before-ai-*.json`, `tests/perf40-after-ai-*.json` and CSV
artifacts.

The separate four-run warmed cockpit comparison has **no meaningful steady
FPS gain**: before runs are 32.60/32.65 FPS and after runs are 32.09/32.53 FPS
(two-run averages 32.62 → 32.31 FPS). Individual p95 intervals are 32.53/32.26 ms
before and 33.89/32.95 ms after. This result is essentially unchanged/slightly
slower, not an improvement claim. The quiet, paused scene does not exercise
moving missile trails or the simulation speedups. Both versions retain the same
1440 × 900 window configuration, VSync and graphics settings, with twenty
seconds of warmup and forty seconds of sampling per run. The cockpit baseline
predates the instrument-upload fix, unlike the CPU baseline saved at the start
of this follow-up. Binary hashes and complete samples are in
`tests/perf40-render-comparison.json`.

Five fresh packaged-development-app native captures were inspected: cockpit
front controls, right side console, missile flight/trail, component damage/fire
and spacecraft cockpit. The label atlases preserve readable labels; MFD/HUD and
navball pixels display correctly; the new cullable missile trail remains visible;
the damage schematic and detached/jammed states retain their appearance. The
spacecraft's large pale foreground panel also appears in a separate baseline
capture and is not introduced by this optimization pass. Capture hashes are in
`tests/perf40-native-captures.json`. These are deterministic posed renders,
not a flown combat recording or manual interaction verification.

An additional desktop keyboard/mouse check could not proceed because the Mac
was locked and computer-use access reported that it could not unlock it. No
alternate unlock method was attempted. Input regressions pass; active foreground
interaction is not claimed. Host/compositor/display state can affect native
renderer timing and these figures should not be extrapolated to fullscreen
Retina performance.

Final delivery uses `./build-mac.sh release`: optimization level 3 with thin
LTO, completed in **2 min 49 s**, at the end of the work period. The packaged
executable is ARM64. Strict/deep code-signature verification confirms it is
valid on disk and satisfies its designated requirement
(`tests/perf40-signature.log`). The optimized executable completes the runway
approach/stop and one-hour orbital diagnostic
(`tests/perf40-release-flight.log`). Its expanded AI physical JSON reports and
both complete CSV traces match the frozen development candidate exactly
(`tests/perf40-release-equivalence.json`).

A single final-release cockpit renderer smoke benchmark reports about **32.3
FPS**, median 30.69 ms and p95 32.96 ms, with the same twenty-second warmup and
forty-second sample. This is a delivery smoke check, not a paired release
performance comparison. It supports no additional FPS gain claim. The source
inventory, final binary/capture hashes and actual start/finish UTC timestamps
are saved in `tests/perf40-delivery-manifest.json`; the recorded work duration
exceeds the requested forty minutes.

## October 15: source optimization pass

All **232 tests pass** (`tests/optimization2-tests.log`), including new
regressions for exact cached visibility, conservative ground-contact rejection,
landing-gear visual-state invalidation, unchanged material asset events and a
complete asynchronous terrain replacement. The development app rebuilt in
34.43 seconds (`tests/optimization2-build.log`). The source review and retained
constraints are documented in [OPTIMIZATION_AUDIT.md](OPTIMIZATION_AUDIT.md).

Three paired repetitions used saved before/after executables with the same
development profile, alternating order. Every reported flight polar, maneuver
and step count and the complete deterministic air-combat/BVR reports matched
exactly. Focused visibility/contact counts also matched. Numerical equivalence
here concerns those diagnostics, not every possible flight trajectory.

| CPU study | Before median | After median | Throughput ratio | Before range | After range |
| --- | ---: | ---: | ---: | ---: | ---: |
| Flight | 43.020 s | 22.451 s | 1.92× | 37.202–48.759 s | 16.173–26.784 s |
| Air combat | 4.871 s | 4.365 s | 1.12× | 4.110–5.029 s | 3.526–4.694 s |
| BVR | 44.123 s | 28.873 s | 1.53× | 33.690–46.986 s | 27.877–36.940 s |
| Focused collision + visibility | 1.059 s | 0.468 s | 2.26× | 0.974–1.628 s | 0.444–0.684 s |

These are whole-process wall times from matched **development builds**, not
release-build speedups or renderer FPS. The focused workload deliberately
includes repeated identical LOS requests; it is not a universal cache-hit-rate
estimate. Results vary with host load and thermals. A preexisting user-owned
game instance remained running and was not interrupted; its background activity
is a source of noise. No compilation, test suite or additional diagnostic batch
ran concurrently with these studies. Full samples and executable hashes are in
`tests/optimization2-cpu-comparison.json`.

The separate paused-runway renderer pass used before/after/after/before order,
five seconds of warmup and forty seconds of sampling per run. Development-build
mean FPS was 12.72 and 12.87 before, versus 14.68 and 14.82 after: **12.79 →
14.75 FPS**, about **15.3% higher average throughput** on this loaded host.
The individual p95 frame intervals were 77.70/122.00 ms before and
93.11/87.09 ms after. Tail intervals remain variable; this does not establish
uniformly smoother frames or release performance. The preexisting game instance
also remained running throughout this pass. No compiler, CPU study or test suite
ran concurrently. See `tests/optimization2-render-comparison.json` for all runs.

The rebuilt packaged app produced fresh native captures for the right side
console, cockpit missile display, tactical map, forest terrain and component
damage (`tests/visual-bvr-cockpit-console-right.png`,
`tests/visual-bvr-cockpit-missiles.png`, `tests/visual-bvr.png`,
`tests/visual-terrain-forest.png`, `tests/visual-air-component-detail.png`).
Inspection confirmed visible switch indicators, radar/missile symbology, map
contacts, complete forest coverage and persistent damage marks/surface states.
These are deterministic posed renders, not manual mouse interaction or a flown
terrain-streaming recording. The asynchronous transition regression verifies
replacement coverage. `cargo fmt --all -- --check` passes, and
`codesign --verify --deep --strict --verbose=2 'Nord Flight.app'` confirms the
bundle is valid on disk and satisfies its designated requirement. Source,
executable and image hashes are recorded in
`tests/optimization2-delivery-manifest.json`.

## October 15: landscape landings and side-console controls

The landscape/landing phase passed **224 tests** before side-console work began.
The combined update passes **227 tests** (`tests/side-console-tests.log`). A new
dynamic regression lands at 70 m/s on the grass meadow, brakes to below 1 m/s
within 20 simulated seconds, and survives; the existing runway approach still
passes. Gear-up terrain impact remains destructive. Terrain-normal, tree-volume,
clear-meadow and bounded/non-overlapping quadtree tests also pass. The old
48 m/s off-runway destruction rule is removed; contact now accounts for ground
normal, slope, sink and surface-dependent rolling/braking resistance.

Adaptive tiles reach 8 m spacing near the ground across the local and BVR
theater. Shared edge normals fixed shading seams found in the first capture;
mesh retirement bounds cache growth during movement. Procedural fields/grass,
rock colors, ground grain and batched multi-tier trees replace the previous
coarser terrain and sparse forest. Terrain vertices retain queried world x/z
coordinates, including in the distant theater. Deterministic tree trunks and
canopies generate matching approximate swept collision boxes. The native
`terrain-field`, `terrain-landscape` and `terrain-forest` captures were inspected.
The meadow is centered at x=6,500 m, z=3,200 m, elevation 62 m. See LANDSCAPE.md.

After completing that phase, sixteen functional side-console controls were added
beside the pilot's legs: eight on each side, with moving toggle levers and
backlit button faces. The existing front controls remain. The left console
handles power, lighting, engine, gear, flaps, airbrake and FBW; the right handles
radar, target/weapon selection, paired dispensers, approach assistance and view
controls. Option/Alt + click remains the interaction modifier. Picking transforms
the ray into each horizontal face's coordinates. Tests verify unique side-face
hits, rearward-ray rejection, shared radar/weapon state and clearance from the
throttle's full travel. An initially obscured APU/engine row was moved forward
of the throttle, based on native captures.

Native side checks are `tests/visual-bvr-cockpit-console-left.png` and
`tests/visual-bvr-cockpit-console-right.png`; logs use `tests/side-console-`.
Captures and automated ray/command checks are distinguished from manual mouse
interaction testing. The final development build and strict/deep app signature
verification are recorded in `tests/side-console-build.log` and
`tests/landscape-console-signature.log`. No matched FPS benchmark or surveyed
terrain/soil/vegetation fidelity is claimed. Source/binary hashes are recorded
in `tests/landscape-console-delivery.json`.

## October 15: cockpit controls, exterior lighting and damage detail

All **219 tests pass**. The final development compile completed in **20.68 s**;
the packaged app passes deep/strict signature verification.

The Gripen now has 24 labelled physical buttons: the existing ten radar keys
and fourteen flight/system/weapon/view controls. The same Option/Alt modifier
gates both systems. Added command/ray tests check unique physical click targets,
engine battery/APU interlocks, gear commands and weapon selection. Native
`bvr-cockpit-controls` captures exposed two console-obscured keys; the lower row
was moved inward and upward, and all fourteen new labels are visible in the
final simplified cockpit capture. These are render inspections plus automated
ray/command checks; manual mouse interaction has not been claimed.

Exterior lights have brighter emissive lenses, recessed collars, view-facing
radial-gradient coronas and shadow-free local spill lights. Wingtip lenses sit
above the rails; beacon mounts follow the fuselage skin. Landing beams increase
from 200,000 to 450,000 lumens. Electrical/gear gating and the double-flash strobe
tests remain passing. Native day/top and night/ground captures inspect their
placement, glow and runway illumination.

Two batched meshes add wing panel seams, root walkway strips and skin/wing
fasteners. Eight hidden-until-damaged regions per aircraft add surface-fitted
scorch, fragment pits and exposed-metal flecks. Player and AI versions follow
their respective mesh shapes and component damage, clearing with reset. New
tests check clean/reset state, left/right damage separation and finite geometry
for both models. Existing damage forces, severity probabilities and explosion
hazard remain unchanged. Smoke, sparks and folded-metal breakup geometry share
the existing 256-particle pool. A high-speed regression checks that short-lived
flames stay within 2 m of the damaged region at 500 m/s, rather than visibly
shedding many metres behind it. Pause/reset/pool-bound tests remain passing.

Evidence is in `tests/cockpit-detail-tests.log`,
`tests/cockpit-detail-build.log`, `tests/cockpit-detail-signature.log`, native
capture logs prefixed `tests/cockpit-detail-native-`, and
`tests/cockpit-detail-delivery.json`. The principal render artifacts are
`tests/visual-bvr-cockpit-controls.png`, `tests/visual-gear-canopy-top.png`,
`tests/visual-gear-night.png` and `tests/visual-air-component-detail.png`.
No matched performance benchmark or calibrated structural-fracture fidelity is
claimed for this visual/control update.

## October 15: illuminated radar bezel

All **214 tests pass** (`tests/radar-buttons-tests.log`). New regressions cast
rays at every visible button corner, reject the gaps between buttons, and check
power/STT/TWS/page lamp state against actual radar commands. The existing shared
cockpit/map commands and radar contact selection regressions remain passing.

The ten right-MFD keys now have raised metal sockets, larger caps, backlit face
labels, state illumination, modifier-hover highlighting and a depressed position
while clicked. Picking uses the same cap dimensions and face plane as rendering.
The higher side rows keep LOCK above the cockpit console; old tiny scope-edge
labels were removed. Option/Alt remains required for interaction, and the hint
explains each hovered command. Both radar pages retain tracks and missile cues.

Final packaged native captures were inspected on range/azimuth with missiles and
the tactical page (`tests/visual-bvr-cockpit-missiles.png` and
`tests/visual-bvr-cockpit-tactical.png`). All ten keys are visible; power and lock
or page/TWS lighting reflect each scene. These repeatable render captures do not
constitute manual mouse interaction testing; button selection and state changes
are covered by the automated ray/command tests. The development compile finished
in **7.41 seconds** (`tests/radar-buttons-build.log`), and deep/strict app signature
verification passed (`tests/radar-buttons-signature.log`).

## October 15: other-aircraft camera

All **212 tests pass** (`tests/aircraft-camera-tests.log`). Added coverage checks
plain 2 versus Alt/Option+2 APU isolation, living-aircraft cycling/death fallback,
unchanged player flight state, 7 restoring camera/look/zoom, transitions between
aircraft and missile tracking, and M returning to the player's map. Existing
missile-camera, flight, AI, damage, audio and avionics regressions remain passing.

Plain **2** follows/cycles living enemies, allies and practice targets. Drag and
wheel controls orbit/zoom around the aircraft center, with radial up. **7**
restores the original view; switching between 2/6 retains the same return view.
The watched aircraft's identity, airspeed and altitude appear in the header.
Own-aircraft instrument overlays are hidden during tracking. **Alt/Option+2**
now toggles the APU. Follow identity changes also inhibit false camera-switch
sonic booms. The four regular C views remain unchanged.

The final development build completed in **21.53s**, **23.41s** including
packaging/signing (`tests/aircraft-camera-build.log`), and strict/deep signature
verification passes (`tests/aircraft-camera-signature.log`). Final packaged
native captures were inspected for close combat and a BVR enemy roughly 123 km
from the player (`tests/visual-air-aircraft-camera.png`,
`tests/visual-bvr-aircraft-camera.png`, startup logs
`tests/aircraft-camera-native-*.log`). Both keep the watched aircraft centered
and display the follow controls/identity. These paused captures check framing;
keyboard transitions and systems isolation are regression-tested rather than
claimed as a manual live flight session.

## Shared aircraft physics and AI pilots

All **209 tests pass** (`tests/ai-tests.log`). Nine added AI regressions cover
identical AI/player plant evolution across all three difficulties with intact,
jammed and detached controls; a pilot-command boundary that never changes pose,
velocity or rates; engine failure and identical substep subdivision; unknown
contacts and memory expiry; 60-second turn/stall recovery; equal full-rate
physics for all eleven NPCs with pause/reset; detected versus unseen missiles;
long-range search-beam acquisition/retention; and independence from the unseen
player's true velocity while pursuing a coasting contact. Existing route tests
now check physical following rather than exact altitude teleportation, and
difficulty tests require common aircraft/weapon power rather than different
kinematic G limits and cannon lethality.

Every enemy, allied wingman and practice target owns a persistent `Flight`
state and uses the same `physics::simulate` plant and environment as the player.
Runtime kinematic speed/orientation steering, the enemy speed floor, artificial
damage sink rates and separate fuel/damage advancement have been removed.
AI pilots command the normal FBW and real actuators; rendering, shaped aircraft
contact and missile jam sampling use their actual control-surface state.

The six 60-second profiles and 28 integrated scenarios are recorded in
`tests/ai-flight-study.json`, `tests/ai-flight-study.csv`,
`tests/ai-combat-study.json` and `tests/ai-combat-flight.csv`. They cover practice,
three dogfight difficulties and all four BVR formations across every difficulty
and mission type. The integrated scripted player also uses the full plant.
These are deterministic control/flight regressions, not a human combat
evaluation or calibrated Gripen vulnerability/performance evidence. Detailed
intent/control/load telemetry stays in diagnostic files, with no normal UI
overlay. See [AI.md](AI.md) for behavior, assumptions and reproduction.

The final integration sweep covers **4,800 simulated seconds** and
**2,894,400 AI physics steps**. All states remain finite and every aircraft
receives the full equal-frequency step count, including the eleven-NPC cases.
Recorded modes include energy extension, vertical lead, lag/overshoot control,
intercept/support, cranking, notching, defensive dragging and stall/terrain
recovery. Peak observed load is **9.227 G**, including physical transients;
the pilot requests at most 8.5 G and uses the same predictive FBW as the player.
That measured peak is not a new hard limit or an envelope certification.

Rejecting aircraft outside the avoidance radius before visibility tests leaves
every recorded integration outcome **exactly equal**, excluding wall-clock
timings. The sweep measured **113.05s before / 76.10s after**
(`tests/ai-validation-summary.json`). These are headless development-build
diagnostic timings, not a matched graphics/FPS benchmark; other user processes
were preserved. The six physical profiles survive all 60 seconds, and engine
failure produces exactly zero thrust. The intact turn and stall-recovery cases
finish near their commanded 250 m/s with less than four degrees of heading
error in these particular initial conditions.

The final development app was packaged/ad hoc signed in **1.98s** after the
cached compile (`tests/ai-package.log`) and passes strict/deep signature
verification (`tests/ai-signature.log`). Final packaged native captures of live
close combat and the largest BVR flight view were inspected
(`tests/visual-air-audio.png`, `tests/visual-bvr-flight.png`, startup logs
`tests/ai-native-*.log`). The live capture advances the new aircraft physics;
the BVR capture is a paused layout check. Native audio pulled **351,872 stereo
frames**, RMS **0.02301** (`tests/ai-native-audio.json`). These native checks do
not replace the automated long-duration flight/combat diagnostics or claim a
manual combat evaluation. Source/app hashes and all summary evidence are in
`tests/ai-delivery.json`. Final project/cache disk use is about **4.9/4.7 GiB**,
with the existing 12-GiB Mac build-cache safeguard retained. The user's running
game was preserved and must be restarted to load the new executable.

## Supersonic aircraft fly-by booms

All **200 tests pass** (`tests/flyby-boom-tests.log`). New regressions move an
NPC aircraft past a stationary listener in exterior and cockpit views, check
one boom per crossing and a second boom on a later pass, reject subsonic fly-bys
and own-aircraft cockpit booms, and exercise the real audio mixer at the new
distance/cabin gains. Existing camera-switch suppression and all aircraft,
weapons, audio and RWR tests remain passing. The first dependency/test rebuild
after clearing the old debug cache took **14m 04s**; test execution took **1.13s**.

Previously, cockpit view suppressed every sonic boom and boom volume used the
engine's short-distance attenuation. Other aircraft's shock-cone crossings now
remain audible inside the cockpit at 45% gain. The pressure-wave sound uses a
1.5 km distance scale and 30 km cutoff, retaining the existing synthetic boom
waveform at fixed pitch. The player aircraft and all eleven NPC slots are
checked; spacecraft and destroyed player airframes are excluded as boom
sources. Continuous engine and cannon synthesis are unchanged. These are
gameplay acoustics, not calibrated sonic-overpressure or atmospheric-ray models.
See [AUDIO.md](AUDIO.md) for behavior and the NASA reference.

The development app rebuilt in **34.00s** (**35.29s** including packaging/signing)
after dependencies were cached (`tests/flyby-boom-build.log`). Strict/deep
signature verification passes (`tests/flyby-boom-signature.log`). A packaged
native audio startup/capture completed (`tests/flyby-boom-native.log`), with
**460,416 stereo frames** pulled by the device and nonzero RMS **0.02298**
(`tests/flyby-boom-native-audio.json`). This establishes native audio operation;
the new fly-by cases and boom audibility are verified by simulation/mixer tests,
not a claimed manual listening session. Final generated cache size is about
**4.2 GiB**, with development and release dependencies retained.

## Build-cache storage control

The project occupied approximately **26 GiB** on disk, including **24 GiB**
of generated development artifacts (`target/debug`, mostly dependency objects
and debug symbols). Removing that regenerable directory reduced the project to
approximately **2.4 GiB**, retaining the release cache, packaged app, source,
assets and diagnostic evidence. Development profiles now omit debug symbols
while retaining incremental compilation. A fresh development build must rebuild
its dependencies once; no development-build timing improvement is claimed.

`build-mac.sh` checks the generated Cargo cache before compilation and after
successful packaging/signing. Above **12 GiB**, it uses `cargo clean` to clear
generated artifacts. The three isolated packaging-policy tests pass
(`tests/rwr-cache-tests.log`): retain a small cache, clean an oversized initial
cache, and clean an oversized result only after the app has been copied and
signed. These tests use command shims, never the real app/cache. Peak compile
space is not hard-limited, and direct Cargo commands bypass the script policy.

## Cockpit radar warning receiver

All **197 tests pass** (`tests/rwr-tests.log`). Added coverage checks passive
search detection with the player's radar switched off, rejection of friendly,
failed and rear-facing emitters, contact memory/expiry, reset, electrical and
avionics failure, radar support tracking, active radar versus passive/IR missile
emissions, and priority-colored display output. The existing geometry, radar,
missile, camera, audio and six-degree-of-freedom regressions remain passing.

The left Gripen MFD now carries the RWR, with aircraft-relative clock bearings,
search/track/active-missile symbols, priority rings, highest-threat messaging,
flashing missile brackets, collision-separated labels and five-second fading
memory. The center engine and right radar displays are retained. Speed and
navigation remain on the HUD and panel attitude ball. The passive receiver is
independent of the radar's power switch but requires battery and functioning
avionics; target practice does not invent emitting threats.

A repeated native capture exposed the attitude-ball housing over the RWR
header. Its Gripen installation was moved forward so the entire receiver is
visible (`tests/visual-bvr-cockpit-rwr-missiles.png`, `tests/rwr-native-missiles.log`).
That scene uses actual BVR missile launches and shows a tracking emitter and an
active radar missile warning. It establishes rendered layout; detection, pause,
memory and failure behavior are tested rather than claimed as manually flown.

The 24 BVR and six close-range deterministic study reports compare exactly equal
to the preceding records (`tests/rwr-study-comparison.json`,
`tests/rwr-bvr-study.log`, `tests/rwr-close-study.log`). The receiver samples
existing emissions/AI support states and never changes flight or weapon state.
Its emission ranges, angular gates and display classifications are explicit
game approximations; the implementation adds neither an RF propagation solver
nor a calibrated Gripen threat library. See [BVR.md](BVR.md) for operation and
primary-source background.

The final optimized compile completed in **9m 07s**. The first packaging shell
reported a parse error after signing because its script had been edited while
running; a fresh invocation of the finished script reused the compiled binary,
packaged and signed successfully in **1.99 seconds**
(`tests/rwr-release-build.log`, `tests/rwr-final-package.log`). Strict/deep
signature verification passes (`tests/rwr-signature.log`).
Both final packaged native captures were inspected: `tests/visual-cockpit.png`
shows the forward HUD/attitude-ball clearance, and
`tests/visual-bvr-cockpit-rwr-missiles.png` shows the complete receiver with
tracking and active-missile threats. Startup logs are
`tests/rwr-release-native-*.log`. The delivery hashes and measured disk usage
are recorded in `tests/rwr-delivery.json`; final project size is about **2.5 GiB**.

## Shared Gripen radar, launch-zone scales, missiles and revised engine mix

All **193 tests pass** (`tests/gripen-avionics-tests.log`). New regressions cover
local non-BVR radar acquisition and power inhibition, common HUD projection,
weapon-dependent launch-zone estimates and coasting/expiry, friendly missile
symbols, closing-threat visibility, dead/receding missile removal, the revised
dry-thrust volume curve, exterior cone-crossing booms, one-time boom emission
and suppression on camera changes. Prior track picking, radar, camera, cannon,
aircraft damage and flight regressions also pass.

The physical Gripen HUD and right MFD now use the same clean radar presentation
in every Gripen scenario. Non-BVR sensors use 5/10/20/40 km scales; BVR retains
40/80/160/240 km. MAX/NEZ launch-zone scales use a moving target-range pointer
and an estimated-zone indication. Short-range weapons use their own bounded
range model rather than the long-range missile envelope. Stale tracks remain
explicitly coasting and do not assert the in-zone cue. Blue friendly and red
closing-threat missile triangles use a separate legend on the MFD. The hostile
missile display uses the documented finite-range warning approximation.

Native captures were inspected with real scenario missile launches in the
largest BVR setup and with the shared radar in close-range combat. Repeated HUD
captures moved the in-zone text away from the pitch ladder. The radar's coasting
range readout and the launch-zone pointer now use the same predicted position.
These posed native captures establish layout; the regressions establish sensor,
selection, filtering and failure behavior rather than a manual combat session.

The 24 BVR and six close-range deterministic study reports remain **exactly
equal** to their pre-change records (`tests/gripen-avionics-study-comparison.json`,
`tests/gripen-avionics-bvr-study.log`, `tests/gripen-avionics-close-study.log`).
This includes the BVR missile trials and is a regression result rather than a
claim of calibrated weapon envelopes or new combat-balance validation.

Dry engine source gain now rises to 0.78 before afterburner. Wet crackle gain
falls from 1.80 to 0.35, its input bandwidth is reduced, raw white noise drops
to 0.002 at full burner, and bass gain increases. Exact-mixer showcase idle RMS
increases from **0.00823 to 0.01597** (+5.76 dB) against the preceding delivered
report; full afterburner RMS changes from **0.25434 to 0.22447** (−1.08 dB).
Current wet output has **6.57%** of Hann-windowed spectral power above 2 kHz;
no historical spectrum is claimed. Peak output is **0.61379**, with zero pause
and external-vacuum RMS (`tests/gripen-audio-comparison.json`,
`tests/gripen-avionics-audio-study.log`). These are digital measurements rather
than subjective listening approval or calibrated sound-pressure levels.

Sonic booms now trigger when any exterior camera crosses an existing aircraft
Mach cone, covering the player and eleven NPC slots. They do not repeat while
inside, and camera switches, reset, cockpit view and initially passing Mach 1
do not manufacture a crossing. The existing transient waveform is retained.

The optimized app built in **3m 37s**, was packaged and ad hoc signed
(`tests/gripen-avionics-release-build.log`), and its strict/deep signature verifies
(`tests/gripen-avionics-signature.log`). Final packaged captures were inspected:
`tests/visual-bvr-hud-missiles.png`, `tests/visual-bvr-cockpit-missiles.png` and
`tests/visual-air-radar-hud.png`. The ordinary approach HUD was also inspected
in the development build (`tests/visual-cockpit.png`). Startup logs are
`tests/gripen-avionics-release-native-*.log`. Native audio consumed **334,464
stereo frames** with nonzero RMS **0.02282** (`tests/audio-native-report.json`).
The packaged offline mixer reproduces the same level measurements above
(`tests/gripen-avionics-release-audio-study.log`). No matched FPS benchmark or
subjective listening approval is claimed. The existing user game process was
preserved and needs restarting to load the new executable.

## Exhaust roar, supersonic quiet regions and cockpit radar tracks

All **188 tests pass** (`tests/afterburner-cone-hud-tests.log`). Added regressions
cover narrowing Mach-cone geometry, front/side silence versus aft audibility,
the transonic crossfade, connected camera-angle gating of engine/airframe audio,
cockpit transmission and independent explosion propagation. A spectral check
limits steady turbine/rumble tone dominance in the wet-engine output. Existing
cannon sustain, shutdown, pause, mute, bounded-mixer and spacecraft checks pass.

The exact mixer renders a 27-second 48 kHz stereo showcase in **0.164 seconds**
excluding file writing (`tests/afterburner-roar-study.log`, `tests/audio-study.json`).
Afterburner RMS increases from **0.15395 to 0.25434**, or **4.36 dB**; the showcase
peak is **0.64896**. Pause and external vacuum remain zero RMS. Bass, mixing
noise, irregular swells and distorted crackle replace most of the wet-engine
periodic hum. These are digital signal measurements, not calibrated acoustic
levels or subjective listening validation. A steady wet-engine listening clip
is saved at `tests/audio-afterburner-roar-preview.wav`; the spool-up preview is
`tests/audio-engine-preview.wav`.

Continuous supersonic engine sources and exterior player airframe noise now
follow a trailing Mach cone, using each source's velocity relative to the
player's local wind. An interior edge fade and Mach 1.00–1.03 crossfade smooth
transitions. Cockpit structural sound and instrument alerts remain audible;
transients propagate from their emission points. This is a steady-motion
geometric approximation rather than an acoustic-history or refraction solver.

HUD regressions verify both normal and simplified radar symbology, standby
gating, measured-track projection without target-truth updates, coasting and
expiry, and selected-target steering behind/outside the combiner. Full view
shows sensed contacts; simplified view retains the designation. Selected range
and closure use the same extrapolated track. The BVR cannon funnel is restricted
to targets within 2 km to keep it from crowding long-range radar symbology.
Development native captures of the full and simplified combiner were inspected
(`tests/visual-bvr-hud.png`, `tests/visual-bvr-hud-simple.png`), including a repeat
after moving the designation label away from the flight-path marker. Native
audio QA confirmed backend stream consumption and nonzero output. Captures
establish appearance; the cone, sensor-data and failure behavior above is covered
by regressions rather than a manual supersonic flight recording.

The final optimized build completed in **8m 29s**, was packaged and ad hoc signed
(`tests/afterburner-cone-hud-release-build.log`), and passed strict/deep signature
verification (`tests/afterburner-cone-hud-signature.log`).
Both HUD captures were repeated and inspected from the packaged release
(`tests/afterburner-hud-release-native.log`,
`tests/afterburner-hud-simple-release-native.log`).
Packaged native audio QA consumed **333,440 stereo frames** with nonzero RMS
**0.01539** (`tests/afterburner-audio-release-native.log`,
`tests/audio-native-report.json`). This confirms backend playback separately
from the offline wet-engine level and spectral checks.

## Cockpit radar and mission setup refinement

All **183 tests pass** (`tests/radar-ui-tests.log`). New checks exercise direct
contact selection on both MFD pages, the shared radar commands and bounds,
standby launch inhibition, every physical bezel button's picking ray, screen
projection and rejected backward/parallel rays, heading-up stability during roll,
and modifier-gated camera interaction. The setup-button regression now selects
the visible scenario/difficulty/mission cards, starts that combination, and
checks cancellation restores both previously paused and running flights.

The dedicated mission screen separates configuration from the live tactical map.
The cockpit radar uses the right MFD, with Option/Alt-click contact designation
and bezel controls. Mouse-wheel cockpit zoom and Home reset support instrument
readability. Native captures exposed side-console geometry covering the bottom
of the MFD; the Gripen console's front section and switch positions were revised
and the capture repeated until the target readout and lower buttons were clear.
The duplicate BVR defense panel is hidden in cockpit view; existing incoming
threat cues and the damage schematic remain available.

The deterministic BVR study's **24 cases** and close-range combat study's **six
cases** compare exactly equal to their pre-refinement reports, including the
isolated BVR missile trials (`tests/radar-ui-study-comparison.json`,
`tests/radar-ui-bvr-study.log`, `tests/radar-ui-close-combat.log`). Radar power
defaults on, preserving those engagements; its new cockpit/map power control
gates own detections, launch authorization and player midcourse support.

Native captures verify appearance; command, picking-geometry and input behavior
are covered by regressions. These are not a manual mouse-play recording or a
matched rendering-performance benchmark.

The final optimized app built in **7m 53s** and was ad hoc signed through
`build-mac.sh release` (`tests/radar-ui-release-build.log`). Strict/deep code
signature verification passed (`tests/radar-ui-signature.log`).
Packaged-app captures were inspected for mission setup, both cockpit radar
pages and the streamlined tactical-map controls: `tests/visual-bvr-setup.png`,
`tests/visual-bvr-cockpit.png`, `tests/visual-bvr-cockpit-tactical.png` and
`tests/visual-bvr-map.png`. Their startup logs are
`tests/radar-ui-release-native-*.log`.

## BVR scenarios, tactical map and long-range missiles

All **177 tests pass** (`tests/bvr-tests.log`), including the actual setup-button
handlers, map-label selection, terrain masking, the last pair in a 12-aircraft
collision set, radar acquisition/aging, launch inhibition, mission/reset/pause
rules, extended motor burn and long-range fuse-distance trials. A ten-second
check advances the player's full flight dynamics with BVR and verifies finite
states, no crash and synchronized flight/combat clocks. Scenario setup now runs
before combat-generation initialization so a new flight cannot receive a step
of the previous engagement.

The optimized app rebuilt in **6m 55s** (`tests/bvr-release-build.log`) and its
strict/deep signature verifies (`tests/bvr-signature.log`). Native captures of the
packaged release were inspected in sensor mode, reveal mode and flight view:
`tests/visual-bvr-map.png`, `tests/visual-bvr-reveal.png`,
`tests/visual-bvr-flight.png`; corresponding logs are `tests/bvr-native-*.log`.
These verify rendered layout and native startup; interaction handlers are
covered by the UI regression rather than claimed as a manual mouse-play session.
No matched renderer-FPS study was performed. The existing user game process
remained open and needs restarting to load the new executable.

All six existing close-range combat diagnostic reports compare **exactly equal**
to the saved pre-BVR reports (`tests/bvr-legacy-comparison.json`,
`tests/bvr-close-combat.log`). This establishes regression consistency for those
recorded deterministic cases, not equivalence of every possible engagement.

The BVR implementation provides four force configurations (1 vs 2, 2 vs 2,
2 vs 4, 4 vs 8), each with Training/Balanced/Hard and elimination/defense choices.
Starts are approximately 125 km apart. The extended mountain theater and tactical
map use the shared terrain function, including line-of-sight masking. Search,
TWS, STT, scan range/elevation, sensor/reveal views, friendly reports and aged
tracks have dedicated controls. See [BVR.md](BVR.md) for the model and controls.

The headless study (`tests/bvr-study.json`) runs all 24 combinations with a
kinematic, nondefending player intercept path. These runs test tactical integration
and finite state, not six-degree-of-freedom combat flight or player win rates.
The nondefending diagnostic player is lost in these runs; this is not a claim
that the scenarios have balanced difficulty or established survival rates.

Isolated long-range missile trials, with 0.01 s integration:

| Trial | Closest swept distance | Time at checkpoint | Final speed |
| --- | ---: | ---: | ---: |
| 70 km, head-on | 10.60 m | 44.32 s | 1,156 m/s |
| 70 km, receding target | 10.52 m | 75.56 s | 867 m/s |
| 70 km, support removed at 10 s, straight target | 0.17 m | 44.42 s | 1,146 m/s |
| 70 km, support removed and target turns at 12 s | 11,170 m | 82.79 s | 597 m/s |
| 100 km, head-on | 8.84 m | 65.49 s | 967 m/s |

Trials stop at fuse distance or terrain contact. Straight targets can still be
reacquired after support loss; the turning target defeats the stale track in
this diagnostic. All five enter terminal-active logic, but the unsupported turn
ends with no track. These are game-model results, not real missile specifications
or calibrated RMAX/NEZ data.

Native captures exposed crowded labels and a flight-readout overlap. Label
placement now separates labels with leader lines to true map positions, including
column fallback for dense missile groups. The BVR flight readout has a dedicated
right-hand panel, and the tactical map has an opaque background. Map labels are
selectable as well as their contact positions.

## Cannon sustain during slow simulation

The player cannon's 160 ms wall-time hold could expire before the next simulation-time shot when the game ran slower than real time. Its recorded loop now sustains directly from the held, powered, ammunition-available firing state. Camera changes update the same voice, and individual player shot events do not restart the held loop. The sound sample, engine mix, weapon cadence and ballistics are unchanged.

New regressions run three seconds of device audio without a scene update or shot event, check continuing output, preserve the sample cursor during a large camera pan/pitch change, and verify silence after release. A combat-state regression checks held input, empty ammunition, electrical loss and aircraft destruction. See `tests/cannon-sustain-tests.log` for the full suite.

All **164 tests pass**. The optimized app rebuilt in **6m 34s** and its strict/deep code signature verifies (`tests/cannon-sustain-release-build.log`, `tests/cannon-sustain-signature.log`). A packaged native run with isolated QA preferences consumed **366,208 stereo frames** with nonzero output RMS **0.01623** (`tests/audio-native-report.json`, `tests/cannon-sustain-native.log`). Native playback verification establishes backend consumption; the slow-simulation and held-input behavior is covered by the regressions above. The unchanged cannon PCM hash remains `b7b1345529cbb99e0a91d3d0cf7d2d5ef847cec6d89a46f63c2f27c62a95f793`.

Updated previews use the final exact mixer (`tests/cannon-sustain-study.log`, `tests/audio-cannon-preview.wav`, `tests/audio-engine-preview.wav`). Showcase peak is **0.436**, afterburner RMS **0.15395**, cannon RMS **0.11335**; pause and external-vacuum RMS are zero. These are digital signal measurements, not calibrated acoustic levels or subjective listening validation.

## Recorded A-10 cannon and exhaust-heavy engine mix

All **162 tests pass** (`tests/a10-audio-tests.log`). The gun now plays an embedded excerpt from the public-domain U.S. Air Force / 355th Wing **A-10 gatling gun strafe** recording (DVIDS 593024, Betty Chevalier). Its first firing pass was inspected using source video frames and audio spectra; the 6.2–7.6 second segment supplies the loop. Source hashes, filtering/crossfade details and attribution are in `assets/audio/README.md` and `assets/audio/a10-cannon-source.json`. No claim of subjective listening validation or BK27 acoustic fidelity is made.

The cannon sustain regression spans three seconds, crossing the 1.375-second sample loop twice. It verifies one continuing voice per stationary emitter, no silent gaps at 15 ms measurement windows, and silence after release. A separate regression checks bounded PCM values and loop-seam continuity. Actual rounds refresh playback rather than restarting the sample. Projectile speed, firing intervals, ammunition and damage are unchanged by this sound revision.

The three jet-whine harmonic amplitudes are exactly one quarter of the preceding revision. Filtered turbulent roar increases from 0.65–1.50 to 1.60–3.80, and exhaust rumble is stronger. These source amplitudes precede cockpit/spatial filtering and the mix limiter; they are not perceived-loudness ratios.

## Cannon speed and sound revision

All **161 tests pass** (`tests/audio-revision-tests.log`). New checks verify the 4,250 m/s aircraft-relative muzzle velocity, an airframe crossing within one substep, continuous burst energy between actual rounds, and silence after the sound tail finishes. The lead solution is checked against the same gravity and inherited aircraft velocity as the actual round, and the funnel uses the shared muzzle speed. The unreachable-target regression now uses a target receding faster than the new muzzle velocity. Both sides use the fivefold speed increase; firing intervals and nominal per-round damage retain their previous scale.

The cannon replaces the 105 Hz bass report with overlapping raspy grains and a slower release. The jet mix reduces broadband hiss and emphasizes turbine harmonics and a separate exhaust rumble. These are original procedural sounds, with subjective preference and recorded-weapon fidelity still requiring listening. The earlier exact combat comparison below predates this intentional ballistics change.

The optimized app rebuilt successfully in **6m 35s** (`tests/audio-revision-release-build.log`), and `codesign --verify --deep --strict` passes (`tests/audio-revision-signature.log`). All six integrated combat diagnostic runs complete (`tests/audio-revision-combat.log`). The packaged native audio backend consumed **352,896 stereo frames**, with four active engine sources and nonzero output RMS **0.01721**, using a separate QA settings path (`tests/audio-native-report.json`, `tests/audio-revision-native.log`). The existing user game process was left open.

The revised exact-mixer showcase renders 27 seconds in **0.07725 s**, excluding file writing, with peak **0.409**, cannon RMS **0.13248** and afterburner RMS **0.12936**. Paused and external-vacuum RMS remain **zero** (`tests/audio-study.json`). Updated listening clips are `tests/audio-cannon-preview.wav` and `tests/audio-engine-preview.wav`. These digital measurements establish signal generation and backend consumption, not subjective listening quality or calibrated acoustic fidelity.

## Flight audio

All **159 tests pass** (`tests/audio-tests.log`). New coverage checks per-round cannon events and ammunition exhaustion, relative Doppler signs, attenuation/pan, vacuum transmission rules, powered spacecraft cockpit versus external view, actual gear motion and one-time latches, simulated propagation delays and pause, shutdown, mute/reset, bounded handoff/voice pools, nonblocking callback operation, high-warp coast, settings bounds and audio-panel input during warp editing. The six deterministic combat reports match the pre-audio build exactly (`tests/audio-physics-comparison.json`, `tests/audio-combat-study.log`). Flight/weapon coefficients and firing intervals are retained.

The 48 kHz stereo showcase uses the exact game mixer. It generates 27 seconds of PCM in approximately **0.069 seconds** on this host, excluding WAV-file writing. Its peak is **0.480** of full scale; the external-vacuum and paused sections have **zero RMS**. Idle, afterburner and cannon RMS are approximately 0.00562, 0.12567 and 0.09099. These are normalized digital values, not calibrated sound-pressure levels or full-game CPU measurements. See `tests/audio-study.json`, `tests/audio-showcase.wav` and the extracted `tests/audio-cannon-preview.wav`.

The packaged native release was run with a separate QA settings path. Its audio backend consumed **334,976 stereo frames** from the custom source by the diagnostic checkpoint, with four active engine sources and nonzero output RMS (**0.0156**): `tests/audio-native-report.json`. Native captures verified the audio panel and its corrected opaque overlay (`tests/visual-air-audio.png`). This establishes PCM generation and native backend playback; subjective listening quality and exact RM12/BK27 recording fidelity remain unvalidated.

The optimized release bundle is rebuilt and its signature verifies (`tests/audio-release-build.log`, `tests/audio-signature.log`). Cargo reports zero dependencies with future-incompatible warnings (`tests/audio-compatibility-build.log`). The existing running game was left open; it needs restarting to load the new binary. **9** opens audio levels; **0** mutes. Settings are separate from warp/timestep preferences. Sound models, primary references and limitations are documented in [AUDIO.md](AUDIO.md). No new matched renderer-FPS benchmark was run for this audio pass.

## Balanced runtime optimization

Measured on this eight-core M1 MacBook Air with 8 GiB RAM, using release builds at the same graphics settings and physics timestep. All **147 application tests pass** (`tests/optimization-final-tests.log`), including bounded-raster pixel equivalence, swept-contact symmetry/contact-time consistency with failed surfaces, and navball asset-modification events. Both patched Objective-C block doctests pass (`tests/block-compatibility-tests.log`). The new Cargo report explicitly says **0 dependencies had future-incompatible warnings** (`tests/optimization-compatibility-build.log`).

Three paired CPU trials alternated executable order with compilation and rendering stopped. Median wall times:

| CPU-only study | Before | After | Throughput ratio |
| --- | ---: | ---: | ---: |
| Flight: polars, maneuvers, 100 landings and orbit | 22.205 s | 20.578 s | 1.079× |
| Six deterministic air-combat engagements | 1.641 s | 0.661 s | 2.484× |

All saved polar/maneuver results and entire deterministic combat reports compare exactly across both executables and all repeats. All 100 landing assertions pass in each flight study. These comparisons exclude timing fields; they do not establish equality of unrecorded trajectory samples or real aircraft fidelity. Executable hashes, individual times and full reports are in `tests/performance-comparison.json` and `tests/performance-{before,after}-{flight,air-combat}-*.json`.

Four native renderer trials used before/after/after/before order. Each used the unchanged paused runway scene, VSync, five seconds of warmup and a 40-second sample, with no concurrent compilation or CPU study:

| Native release run | Mean FPS | Median frame | 95th percentile |
| --- | ---: | ---: | ---: |
| Before 1 | 35.30 | 33.30 ms | 34.09 ms |
| After 1 | 40.22 | 17.11 ms | 34.16 ms |
| After 2 | 37.02 | 33.13 ms | 34.82 ms |
| Before 2 | 36.27 | 33.24 ms | 34.25 ms |

The two-run average increases from **35.79 to 38.62 FPS**, about **7.9%**. The p95 result does not improve; compositor/VSync scheduling, thermal state and background work remain part of these host measurements. This is a paused renderer comparison, not active-combat FPS or a guarantee on other machines. The earlier single baseline was 34.33 FPS; the paired repeats above are the reported comparison. Raw runs, hashes and worker counts are in `tests/performance-render-comparison.json`. The optimized native process confirms **6 compute / 1 asynchronous / 1 I/O worker**. Worker count is capacity, not measured utilization of every core.

Physics coefficients, timestep/substep limits, collision sampling, meshes, particle budgets and graphics quality are retained. Numerical flight verification still lands/stops with 2.4 m/s touchdown sink and reports approximately 0.000003 relative orbital-energy error over one simulated hour (`tests/optimization-flight-validation.log`). Native baseline and optimized simplified-cockpit captures were compared, including the damage schematic, readable failure labels and the working attitude ball. The cockpit's material refresh is retained because the 3D display needs the prepared texture rebound. Packaged-app breakup and deep-space captures were also inspected (`tests/performance-optimized-{cockpit,breakup,space}.png`); these are posed renders, not recordings of flown combat. The packaged release is rebuilt and ad-hoc signed; ARM64 architecture and signature verification pass (`tests/optimization-signature.log`). See [Performance and build notes](PERFORMANCE.md) for implementation details and reproducible commands.

## IR countermeasures

IR countermeasure revision: all **144 tests pass** (`tests/ir-countermeasure-tests.log`). New tests cover reacquisition only for detectable aircraft, unchanged radar retention behavior, flare-only versus combined maneuver outcomes, and a 120/240 Hz comparison. The shared IR model now favors aircraft track continuity over a nearby straight-flight flare and revisits the aircraft before flare expiration. Closing IR warnings no longer disappear solely because a flare was selected. The native countermeasure capture was inspected (`tests/visual-air-defenses.png`), the development bundle rebuilt (`tests/ir-countermeasure-build.log`), and its signature verified.

The missile diagnostic runs 100 seeded rear-aspect trials per case at 1.8 km initial range, 400 m/s missile speed, 230 m/s aircraft speed, 3 km altitude and 65% throttle. Three flare bursts alone produce 100 hits despite 89 temporary diversions/reacquisitions; a 3 g break alone also produces 100 hits. Combining those bursts and the break produces 25 hits. The no-defense baseline produces 100 hits. Results are in `tests/missile-gnc-study.json` and `tests/ir-countermeasure-study.log`; they are conditional gameplay-model results. Integrated combat was rerun in `tests/ir-countermeasure-combat-study.log`: defended Training/Balanced/Hard finish 120 seconds with 98%/65%/51% hull. The previous damage-study values below are historical.

## Component failures and probabilistic missile damage

The current revision passes **142 tests** (`tests/component-damage-tests.log`). New regressions check all three probability anchors and 60,000 seeded category rolls; local missile engine/surface failures; jammed-angle fixed loads and loss of command response; neutral/control load removal after detachment; changed six-degree-of-freedom flight; paired flare/chaff inventory and cooldown; high-speed swept contact; low-speed separation/debouncing; enemy–enemy destruction and one-time kill counts; fire timing across two timesteps; and particle bounds, pause and restart. A rendering regression also verifies that the general wreck-visibility update cannot restore a detached panel or change a jammed angle.

The same missile resolver serves player and enemy hit paths. Burst distance and fuse activation use the compound convex airframe skin. Aircraft contact checks account for jammed angles and detached panels, with orientation held fixed inside each short sweep. Fragment and fire effects use simulation time and a 256-particle cap. Existing flight, landing, propulsion, gear, camera and missile regressions remain passing.

Native posed renders were inspected and revised to remove schematic/airspeed overlap, remove the intact model after breakup, feather smoke edges, and bound explosion brightness. Captures include jams, detached surfaces, fire, player breakup, enemy–enemy collision and simplified cockpit damage (`tests/visual-air-component-*.png`). These are deterministic inspections of posed failures and advancing effects, not recordings of manually flown missile hits. The development app rebuilt successfully and its signature verifies (`tests/component-damage-build.log`). No new sustained renderer benchmark was run.

The latest six integrated engagements are recorded in `tests/air-combat-study.json` and `tests/component-damage-study.log`. Undefended Training survives 120 seconds with 80% hull and a failed engine; Balanced and Hard are destroyed at 94.4 and 71.0 seconds. Automatically defended Training/Balanced/Hard survive 120 seconds with 98%/80%/83% hull; defended Hard also loses its engine. These scripted runs demonstrate survivable critical failures and catastrophic outcomes under the implemented seeded rules, not real aircraft survivability. Earlier combat-study numbers below describe superseded damage revisions. See `DAMAGE.md` for the current probabilities, controls and physical assumptions.

## Earlier revisions

Combat HUD revision: the cannon intercept cue solves for the earliest constant-velocity target intercept within the actual 2.5-second round lifetime, including the gun muzzle offset, 850 m/s relative muzzle speed, inherited player velocity and gravity. Tests verify the predicted projectile and target positions coincide, reject unreachable/dead targets, and check funnel wingspan, straight-flight drop and turn-rate curvature. The F1 input regression verifies toggling while paused, camera/help preservation, and toolbar hiding/restoration. The Gripen combiner and its projection were raised together so forward sightlines lie within the useful HUD region. The right cockpit display now includes weapon ammunition, target range, closing speed, lock and damage.

All 130 tests pass (`tests/combat-hud-tests.log`). Native posed renders cover full/simplified external and cockpit views, an incoming radar threat (`tests/visual-air-hud*.png`), and the updated missile-follow telemetry (`tests/visual-combat-follow.png`). Missile-follow mode hides player flight overlays to prevent them overlapping weapon telemetry. The funnel uses a constant-turn virtual-shot model and known 8.4 m procedural-target wingspan; the direct lead circle assumes constant target velocity. These checks verify the implementation under those assumptions, not aircraft avionics fidelity or guaranteed hit probability. The development bundle build is recorded in `tests/combat-hud-build.log`; native captures establish appearance, while F1 control behavior is covered by input-system tests.

Combat-damage revision (October 14): all 126 tests pass (`tests/combat-damage-tests.log`). Player and enemy missiles now use the nearest relative swept point to locate a proximity burst; both sides use the same component damage calculation. Cannon rounds use shaped-airframe swept tests and can leave a target flying. Tests cover a survivable enemy-wing missile hit, a survivable player-wing missile hit, damage falloff with distance, cumulative structural failure, fire suppression, shaped projectile intersections and a one-second six-degree-of-freedom comparison showing reduced lift and changed roll rate from a damaged wing. The development app builds and its signature verifies (`tests/combat-damage-build.log`). Paused native renders of player and enemy damage were inspected (`tests/visual-air-damage.png`, `tests/visual-air-target-damage.png`).

The latest six integrated 120-second combat-study runs are in `tests/air-combat-study.json` and `tests/combat-damage-study.log`. Undefended Training/Balanced/Hard end in destruction at 118.7/53.0/30.6 seconds; automatically defended runs survive to 120 seconds with remaining hull 96%/76%/85% and damaged systems. These scripted results show that incoming weapons can hurt the player and that countermeasures change the outcome under the implemented assumptions. They do not establish real-world weapon lethality or aircraft survivability. See `DAMAGE.md` for component, flight-effect and collision assumptions.

Target practice and performance revision (October 14): the F12 scenario starts airborne with three steady-route aircraft and two missiles of each type. Tests cover acquisition/lock and ammunition limits, heat-seeker aspect versus radar acquisition, intercepts with both weapons, an intercept while player aerodynamics runs, high-speed relative collision sweeps, guidance loss, target altitude, keyboard target/weapon/fire controls, scenario cleanup, and configured timestep/pause for targets and missiles. All 91 tests pass (`tests/combat-performance-tests.log`). Native exterior, cockpit, close target, launch, missile-flight and impact renders were inspected (`tests/visual-combat*.png`); the smoke has feathered edges, motor exhaust uses layered fading surfaces, and impact geometry fades toward its rim. The live native app also displayed a target hit and reduced ammunition. Close inspection cameras apply only to the disposable render-check process; normal gameplay keeps the existing four cameras. `tests/combat-performance-build.log` records the development app build.

The afterburner increment is increased from 30.1 to 60.0 kN and the transonic drag peak reduced. Nominal static wet thrust is 110.4 kN. Smooth high-Mach intake loss sets the clean reference terminal speed to Mach 2.0001 at 60,000 ft (18,288 m). Two controlled six-degree-of-freedom flights starting at Mach 1.8 and 2.1 finish after 360 simulated seconds at Mach 2.0023 and 2.0049, with altitude error below 0.72 m and no crashes. A third flight at 2,500 m accelerates from Mach 0.85 to 1.3887 in 180 simulated seconds without a crash; its altitude remains within 11.55 m of the target. These runs use still air, gear/flaps stowed, 1,500 kg fuel, and infinite fuel to keep mass fixed. They use an external altitude feedback controller for the diagnostic. Raw trim balances and flight results are in `tests/jet-performance.json` and `tests/jet-performance.log`. This establishes the requested game envelope rather than real Gripen engine fidelity. No hard velocity cap is applied.

Daytime afterburner revision (October 14): full-power jet exhaust now increases core/envelope opacity and outer-layer emission as daylight rises, making the burner more visible against a bright sky. Night appearance and engine thrust are unchanged. The day/night material comparison and thrust check pass with the rest of the suite: 79 tests (`tests/daytime-afterburner-tests.log`). The packaged development build is recorded in `tests/daytime-afterburner-build.log`. Native daylight, night, and engine-off captures were inspected (`tests/visual-engine-jet-full.png`, `tests/visual-engine-jet-night.png`, `tests/visual-engine-jet-off.png`).

Nozzle actuation revision (October 14): the petal tips and lip rings now expand with engine spool, from 0.82× to 1.12× their nominal radius, while the rear petal roots remain fixed. The exhaust base follows the same opening. Shutdown returns the geometry to 0.82× even when stale spool telemetry remains. A geometry regression checks intermediate and full power, root attachment, lip tracking, and shutdown. All 78 tests pass (`tests/nozzle-expansion-tests.log`); the development app build is recorded in `tests/nozzle-expansion-build.log` and its signature verified. Native low/full jet, powered spacecraft, and engine-off captures were inspected (`tests/visual-engine-jet-idle.png`, `tests/visual-engine-jet-full.png`, `tests/visual-engine-space.png`, `tests/visual-engine-jet-off.png`).

Engine appearance revision (October 14): both nozzles now use segmented petals, a recessed throat, and a powered combustion face. The jet has a blue dry-thrust core with an orange high-spool envelope; the spacecraft has a broader blue-white plume. The engine-off capture shows no exhaust or powered glow. The packaged app was inspected in five paused native captures: `tests/visual-engine-jet-idle.png`, `tests/visual-engine-jet-full.png`, `tests/visual-engine-jet-night.png`, `tests/visual-engine-space.png`, and `tests/visual-engine-jet-off.png`. All 77 tests pass (`tests/engine-visual-tests.log`), the development bundle rebuilt (`tests/engine-visual-build.log`), and its code signature verified. These posed captures establish appearance and state gating, not plume-flow fidelity or a new performance benchmark.

Gripen C canopy study: Saab's [C-series foldout](https://www.saab.com/globalassets/products/aeronautics/gripen-c-series/gripen_c-foldout.pdf), especially the labeled cutaway, informed the tapered windscreen, cockpit bubble and rear shoulder. The aircraft remains a procedural visual approximation rather than a CAD reproduction. The latest mesh uses seven smooth width/height stations, shared vertices and smooth normals on the glass, curved skin filler outside the narrowing rim, and thin posts that follow the windscreen. Cockpit fittings were moved inside the profile. Side, top, opposite and cockpit captures are saved as `tests/visual-gear-canopy*.png` and `tests/visual-cockpit.png`; build and regression logs are `tests/gripen-canopy-build.log` and `tests/gripen-canopy-tests.log`.

Sleeker canopy revision: replaced the raised plateau with a single forward cubic crown, inclined windshield and long aft taper tangent to the airframe. The existing flush skin boundary is retained. Canopy bow, seat/headrest, HUD and navball positions were fitted under the lower shell. Packaged side, front and cockpit screenshots were inspected (`tests/visual-gear-canopy.png`, `tests/visual-gear-front.png`, `tests/visual-cockpit.png`). Development build and regression logs are `tests/sleek-canopy-build.log` and `tests/sleek-canopy-tests.log`.

Flush cockpit revision: the canopy rim now lies directly on `upper_skin`, including both longitudinal ends. The raised side surround and end walls are removed; narrow rim strips and curved rails follow the airframe. Cockpit walls and glare shield are inset to remain inside the shell. Packaged exterior and cockpit screenshots were inspected. `./build-mac.sh` now defaults to the incremental development profile; `./build-mac.sh release` retains the optimized release option. The development compile took 10.66 seconds (`tests/flush-canopy-build.log`), and bundle signature verification passed. Regression results are in `tests/flush-canopy-tests.log`.

Cockpit-to-airframe repair: the glass shell and canopy bows now share a fitted profile. A continuous coaming connects the glass rim to the exact fuselage cutout boundary, including front and rear shoulders. The new geometry test checks the full shared rim, skin attachment and finite normals. All 76 tests pass (`tests/canopy-tests.log`). Native front exterior, opposite exterior and cockpit captures were inspected; the cockpit profile preserves the HUD sightline. Release packaging is recorded in `tests/canopy-release.log`.

Latest door/HUD/model/light revision: 75 tests pass (`tests/door-hud-light-tests.log`). Main doors use offset hinge axes; fixed wing-root triangles are clipped out of the well openings. Checks cover fully open doors against the fuselage envelope and wing-root removal, alongside the existing tire and ground-clearance sweeps. Curved wing sections have finite normals after clipping, and the wing collision envelope covers their increased thickness. HUD tests verify direction projection and transparent symbol rendering; light tests verify electrical/gear gating and the double-flash pattern. Native captures cover level/banked cockpit, both exterior sides, the frontal model, compressed ground gear, and night landing beams (`tests/visual-cockpit*.png`, `tests/visual-gear-*.png`). The approach landed and stopped without damage at 2.4 m/s sink (`tests/door-hud-light-flight.log`); the one-hour orbital check retained 0.000003 relative energy error.

Gripen C gear revision: 68 tests pass (`tests/gripen-c-gear-tests.log`). The external box wells are replaced by recessed bays and door skins fitted to the fuselage's elliptical stations. Large wheel covers close after both deployment and retraction; separate leg doors remain open around the struts. Tests check flattened tire containment, conformity of closed skins, steering response, door ground clearance at full suspension compression, and tire clearance against skins, liners and ribs across 101 deployment positions plus door sweeps at both end positions, with unloaded and fully compressed gear. Native screenshots include closed, opening, intermediate deployment, deployed, opposite underside, and ground-level views. The mechanism uses Saab's C-series cutaway as a visual reference; dimensions and detailed motion are fitted approximations.

Before this gear revision, the cockpit/precision implementation passed 63 Rust unit/regression tests. New checks cover exactly four camera choices, separate map input, maneuver-handle drag and release, orbital node timing, safe warp limits, bounded high-warp coast, configured per-frame timestep independent of wall time, gear-door sequencing and moving-wheel clearance, atmospheric-effect activation, deep-space precision and mesh cutout clipping. Existing aero, landing, collision and shutdown tests remain passing. Results are saved in `tests/cockpit-map-warp-tests.log` and the newer `tests/precision-cockpit-tests.log`. The Mac bundle is built in release mode, stripped, and ad-hoc signed. Windows runtime was not tested.

## Numerical flight checks

- 294 steady polar samples span seven Mach values, three flap settings, and 14 incidence values, including reverse-flow conditions. All reported lift/drag values are finite and drag remains nonnegative.
- Six 20-second maneuver cases apply pitch, yaw, and roll pulses with FBW enabled and disabled. All remain finite and avoid vehicle loss in the saved study. The raw pitch case stalls; the FBW pitch case retains attached flow.
- 100 still-air approach-assist repeats land and stop successfully.
- The final approach diagnostic touches down at approximately 2.4 m/s sink and stops on the runway.
- A one-hour spacecraft coast remains near 220 km altitude with approximately 0.0003% relative orbital-energy drift.
- CPU-only release study throughput is about 6.03 µs per jet physics step on this host. This is not renderer FPS.

The diagnostics and study are saved in `tests/final-flight-validation.log`, `tests/final-study.log`, and `tests/aerodynamic-study.json`. `tests/aerodynamic-study.png` visualizes the polars and maneuver load factors.

## Render timing

Each sustained run uses a paused deterministic scene, a continuous event loop, VSync, a five-second warmup, and a 40-second wall-clock sample. Host/compositor/display effects are included. These results should not be extrapolated to Windows or other GPUs.

| Release scene | Mean FPS | Median frame | 95th percentile |
| --- | ---: | ---: | ---: |
| Balanced / FXAA | 40.0 | 16.86 ms | 34.30 ms |
| Orbital overview / balanced | 41.6 | 16.78 ms | 34.28 ms |
| Same flight scene / 2× MSAA | 37.6 | 33.20 ms | 35.25 ms |

The paired FXAA/MSAA result is a modest average-rate improvement, with a larger difference in median frame interval. Slow frames remain near two display refresh intervals. Early exploratory runs and the reverted terrain LOD variant remain in `tests/render-debug-*.json` for provenance; they are not the final matched release comparison.

## Visual and interaction checks

App-native screenshots were inspected for low/full throttle exhaust, separate deflected surfaces, the new instruments, and spacecraft details. Vacuum plume expansion was corrected to develop downstream while retaining the nozzle-exit radius. A regression checks that attachment contract.

The packaged app was opened and visually inspected. Native keyboard pause, spacecraft scenario selection, orbital overview, and the SAS mode button were exercised. Prograde mode changed the in-app SAS readout and produced RCS torque without main-engine thrust. Mouse orbit accepts raw motion and a cursor-event fallback. Input-system regression tests verify left-drag rotation, release ending capture, and button-started drags staying with the UI. The desktop automation tool's synthetic drag did not visibly rotate the view, so native physical mouse behavior was not verified by that tool.

The gear/collision update was exercised in the native app: side-view gear down, fully stowed gear, a paused 37% deployment pose, and F11 collision-skin outlines were visually inspected. Shift/Ctrl input-system tests pass, including paused input and simultaneous-modifier cut priority. The desktop automation tool refuses isolated modifier keys; its synthetic modifier chords did not set throttle, so physical modifier-key interaction is not claimed as visually verified. The updated 50-test result is in `tests/gear-collision-tests.log`; repeated landing/orbit diagnostics are in `tests/gear-collision-flight.log`. The earlier render benchmarks were not repeated for this change.

Strict Clippy with `-D warnings` reported 17 style warnings, primarily large Bevy system signatures/query types, default-field assignment patterns, and small style suggestions. It is not a clean strict-lint run. Compilation, formatting checks, and the functional tests pass.

## Physical scope

Missile GNC update: all 117 tests pass (`tests/missile-gnc-tests.log`). Alongside the initial 104 combat/camera/airframe tests, the new tests cover infrared rear aspect, sun glare, afterburner signature and flare competition, look-down ground-relative Doppler notching, terrain masking, finite track memory without target-truth leakage, autonomous radar handoff, repeatable countermeasure successes and failures, enemy-owned finite countermeasures, fin slew limits, density-dependent acceleration authority, propellant exhaustion, coast/turn energy loss, closed-loop interceptions, and trajectory consistency when halving the timestep. Tab/O/Space, V/Backspace and 6/7 camera regressions remain covered.

Seven shared-GNC engagement checks are recorded in `tests/missile-gnc-study.json`. Dry and afterburning straight targets are intercepted by the infrared weapon at about 9 seconds; the sun-aligned case briefly loses measurements, then reacquires and intercepts. The supported radar case intercepts at 16.3 seconds; losing support against a maneuvering target produces a 2.66 km closest approach. A sustained look-down notch produces a 2.06 km closest approach, while the look-up comparison intercepts. Burnout and coasting speed, achieved g, track-memory duration and energy are recorded. In 200 seeded flare trials at fixed geometry, 131 dry-thrust cases divert versus 75 afterburner cases. These counts are conditional on that geometry and burst radiance, not universal flare effectiveness.

Earlier missile-GNC engagements using the real player physics and a level/speed feedback controller were recorded in `tests/missile-air-combat-study.log`. Their damage results were superseded by the current damage revision above. The diagnostic verifies connected combat behavior under scripted conditions, rather than guaranteeing survival or establishing real combat fidelity. NPC motion remains bounded kinematic steering. Missile control uses a reduced-order three-dimensional acceleration plant and body-rate lag, rather than a calibrated rigid-body 6DOF weapon model. See `MISSILES.md` for assumptions, equations and primary-source background.

Native render checks include the three difficulty panels, an incoming radar warning, flare/chaff bursts, damaged systems, the cockpit and missile-follow views. Captures use a separate paused process; they establish appearance rather than manual keyboard use or a recording of a flown encounter. The development app is rebuilt and ad hoc signed through `build-mac.sh`.

Precision/cockpit update: all 63 tests pass (`tests/precision-cockpit-tests.log`). High-altitude spacecraft subtract the vehicle's double-precision position before creating float render coordinates, keeping the root near zero. Tests cover distances from 220 km altitude to 10^12 m with millimetre-scale offsets. Map planet centers use the same origin; distant projection limits expand as needed. Above 100 km the surface-origin atmospheric shader is removed. A posed native render at 10^10 m is saved in `tests/visual-far-space.png`; `tests/visual-cockpit.png` shows the smaller left-side attitude ball with the engine/navigation displays unobstructed. Exact polygon clipping replaces centroid-based cockpit/gear mesh removal to avoid sawtooth boundaries.

Native packaged-app checks exercised M opening the independent map, simulation advancing at selected warp, adding a node and saving rates. The final packaged build also displayed and saved the timestep, confirmed in `~/.nord-flight/settings.json`. The automation tool's synthetic node drag did not visibly edit delta-v; input-system drag/release tests pass, but physical mouse interaction is not claimed as verified by that tool. The timestep regression advances five frames by 0.01 s each despite negligible elapsed wall time and confirms pause stops the simulation clock.

Cockpit/map/warp render checks are saved as `tests/visual-cockpit.png`, `tests/visual-space-cockpit.png`, `tests/visual-map.png`, `tests/visual-gear.png`, `tests/visual-reentry.png`, and `tests/visual-transonic.png`. These are deterministic posed app renders, not recordings of a flown reentry. Cockpit telemetry textures and the panel-mounted attitude ball were inspected. Atmospheric effects are density/speed/Mach/load-driven visual proxies; no thermal damage or shock-flow solver was added. The raised eye and lowered glare shield improve the outside sightline. Cockpit screen/HUD surfaces are hidden from external cameras.

Gear sequencing uses open/move/close phases and widened openings/hinges outside the wheel sweep. A clearance test samples 101 deployment positions. The per-wheel spring and collision model remain approximate. Maneuver predictions use an instantaneous impulse in the orbital frame; nodes do not automatically fire engines. High-warp coasting is checked against orbital-element drift and elapsed clocks. No new matched render benchmark was run; captured frame rates vary during shader compilation and concurrent builds.

These checks establish numerical consistency, regression behavior, and rendered appearance. They do not establish real Gripen flight fidelity. The panel coefficients, transient stall terms, servo-load reduction, inertia model, propulsion lapse, and upper-atmosphere vacuum transition remain explicit game approximations. See `AERODYNAMICS.md` for the equations and limitations.
# Enemy weapon lethality, all-aspect IR and hit reports — October 15, 2026

Final optimized app built through `./build-mac.sh release` in 3m 05s and passed `codesign --verify --deep --strict`. Formatting checks pass. Native captures from this signed build were inspected: `tests/visual-air-hit-report-cockpit-simple.png` (surviving enemy, jammed/detached controls, engine failure and fire), `tests/visual-bvr-hit-report-destroyed.png` (destroyed enemy in full BVR view), `tests/visual-air-hit-report-fade.png` (50% opacity at report age 3.325 s), and `tests/visual-combat-impact.png` (an actual simulated practice missile hit, one strike/one kill, destroyed-enemy report). Initial captures exposed overlap with the view-status badge and BVR status; corrected placement was rebuilt and recaptured. These are native renderer/diagnostic checks, not manual interactive playtesting.

`tests/enemy-weapons-tests.log`: **247 passed, 0 failed**. Added regressions check the enemy missile profile over 101 normalized burst distances, seeded direct-hit outcomes for both missile types, reproducibility and surviving outcomes, fourfold localized/cumulative enemy cannon damage, unchanged player probability anchors, head-on/beam IR acquisition, missile and cannon hit-report publication in close combat and BVR, one-time kill counting, friendly damage strength, snapshot persistence, replacement/reset, three-second hold and 0.65-second fade, pause, and full/simplified cockpit/map visibility and placement. Existing guidance, sun, countermeasure, energy, physics and rendering regressions also pass.

The release missile diagnostic is recorded in `tests/enemy-weapons-missile-study.log` and `tests/missile-gnc-study.json`. In its fixed rear-aspect test, no defense, flares alone and a break alone each produce 100/100 hits; flares plus the break produce 25/100 hits. All 89 temporary straight-flight flare diversions are reacquired. These are conditional gameplay-model results; all-aspect acquisition does not imply calibrated real-world missile performance. Enemy immediate-destruction odds are configured at 97% for skin contact, 88.5% at half fuse distance and 80% at the outer edge. Accumulated damage can also be fatal on a surviving-severity roll.

