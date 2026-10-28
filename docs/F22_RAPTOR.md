# F-22A refinement

The subsequent [flight-model revision](F22_FLIGHT_MODEL_V2.md) supersedes the
engine/polar, actuator, inertia, envelope and approach tuning described in this
historical report, and adds finite missile release dynamics.

The later [differential-vectoring update](F22_DIFFERENTIAL_TVC.md) adds nozzle
roll control, stronger stabilator loads and updated F-22 control allocation.

For the subsequent appearance-only rebuild of exhaust/TVC, canopy, wings, tails,
paint and landing gear, see [F22_VISUAL_REBUILD.md](F22_VISUAL_REBUILD.md).

This pass focuses on the player F-22A. The other jets retain their previous
airframe definitions and flight settings. AI remains in Gripens. No spacecraft,
sensor-signature or missile-guidance overhaul is included.

## Exterior and cockpit

The [lower fuselage volume update](F22_BELLY_VOLUME.md) adds a deeper bay body,
paired lower engine housings, fitted doors and matching collision contours,
using USAF underside and side-view photographs as references.

The Raptor now has a dedicated broad, flattened chine/body loft, clipped diamond
wing, separate flaperons/ailerons, larger swept stabilators, canted fixed fins
and fitted rudders. Moving-panel outlines also define their collision skins;
detaching a panel removes its mesh and aerodynamic contribution. Original round
engine tubes have been replaced by blended decks. Recessed trapezoidal intake
trunks, thermal shields, panel borders, formation strips, gray camouflage and
low-visibility star-and-bar markings add surface detail. Camouflage is vertex
color on the existing geometry; small panels are subdivided on their original
planes to sample the broad patches.

Rectangular exhausts have separate moving top/bottom flaps, sidewalls and
serrated trailing edges. The open rectangular plume has no cone cap across the
exit. Its direction follows the nozzle, and engine shutdown removes the plume.
The canopy has a subtle gold tint and no forward bow across the pilot's view.

The cockpit uses a larger center tactical display, separate RWR/radar displays,
a flight/system readout, lower standby attitude ball and side stick. The system
readout shows incidence, G, FBW/hydraulic state, engine health, nozzle angles,
fuel, bay and gear positions. The wider cockpit field of view and recentering
angle expose more instruments while keeping the eye above the coaming. Existing
Option/Alt-click radar and system controls remain functional. These displays
implement the game's avionics, not the real F-22's classified software/menu tree.
The mouse wheel varies F-22 cockpit field of view from 38 to 82 degrees;
Home restores its 72-degree overview and recentered look angle.

## Flight model

The wing model now uses the F-22's span, area, mean chord and sweep in lift slope,
convective response, rate derivatives, Reynolds effects and induced/ground-effect
drag. Its separation onset and conventional-tail static margin, pitch damping
and post-stall restoring moment are separately tunable. The coefficients remain
gameplay estimates, not reconstructed flight-test derivatives.

At low speed, aft stick commands incidence, blended into pitch-rate control as
airspeed rises. Neutral stick unloads the aircraft. Full aft stick approaches a
roughly 60-degree incidence target; the predictor limits further nose-up rate
near the envelope. High incidence reduces demanded roll rate. The existing G
protection remains active. Leading-edge flaps schedule with incidence, using
the same effective angles in force evaluation, rendering, jams and detachments.

FBW allocates pitch across aerodynamic panels and available pitch-axis thrust
vectoring. The vector derivative comes from each healthy running engine's thrust,
nozzle direction and lever arm relative to the center of mass. Fixed moments from
jammed nozzles are included in allocation bias. Nozzle travel is bounded at
20 degrees each way and actuation remains rate-limited. Engine/nozzle/hydraulic
damage changes the actual authority. No yaw-vectoring thruster, orientation
animation, airspeed floor or post-stall force bypass is added.

With **T / FBW off**, raw stick commands can cause departures or tumbling. Thrust
vectoring cannot prevent energy loss or guarantee recovery at every altitude,
speed, power setting or damage state. Controlled high incidence is a transient
maneuvering capability, not a hover mode.

The existing approach assistant uses F-22-specific approach speed and available
dry thrust, with the outer pitch controller kept separate from manual incidence
commanding. The study includes an assisted approach to actual gear contact.

## Loadouts and controls

**F6 → F-22A → Start / Reset** selects the aircraft. Internal carriage is the
default: two infrared missiles in side bays and six radar missiles in the main
bays. The F-22 loadout button can move two radar missiles to external pylons;
inventory is unchanged. External pylons and weapons add drag, balance changes
and pylon mass. Internal missile mass affects flight without imposing external
store drag. Open weapon bays add drag. Weapon dimensions and masses are simplified
family representations; guidance and sensor behavior remain shared with the game.

- **T:** FBW toggle.
- **Alt+B:** manual bay doors; combat tracking also requests opening.
- **Alt+J:** jettison missiles; external pylons remain attached.
- **Home:** recenter the F-22 cockpit view.
- **Option/Alt+click:** cockpit switches and radar controls.

## Validation and reproducibility

Final numerical checks: **331 tests pass, zero failures**. Identical-seed fleet
study outputs for Gripen, Su-27SM, F-35B and F/A-18E match the prior fleet study
exactly, including the Su-27 cobra samples. A shared drag expression retains its
original floating-point operation order for those aircraft.
The native interaction check also caught a permanent false stall warning caused
by counting an unused control slot. F-22 attachment telemetry now excludes unused
and detached panels; a regression checks normal-flow flight and a detached panel.
The warning fix changes presentation telemetry, not force or integration results.

| Controlled study | Measured result |
|---|---|
| 110 m/s start, 6 km altitude, FBW, aft stick for 8 s then unload | Peak 49.80° AoA; 4.17° at 20 s; no crash |
| 95 m/s start at 40° AoA | Healthy vectoring peaks at 50.00° and unloads to 3.95°; failed vector actuators peak at 40.84° and remain near 39° at 20 s |
| Level acceleration with a scripted load-feedback pilot | 240 → 459.99 m/s in 60 s at full afterburner; final altitude 6,071.6 m for a 6,000 m target; final load 0.99 G |
| 40° bank with the same pilot | Final bank 40.14°; altitude 6,003.3 m; heading changes about 58° |
| Complete assisted approach with default wind | Runway touchdown at x = 0.68 m, z = 800.82 m; sink 2.30 m/s, speed 106.62 m/s |
| Direct controls | Full aft stick can depart/tumble; no attitude or incidence cap is imposed |

These trials use a fixed 1/120 s timestep. The airborne maneuver/acceleration
trials use infinite fuel to isolate handling; their speeds and incidence are
conditional gameplay-model measurements. The level/turn pilot uses normal-load
feedback; an earlier instantaneous-AoA feedback pilot excited large oscillations
and was rejected. The approach controller was corrected after an initial
off-field touchdown. The accepted regression requires paved-runway contact and
sink below 3 m/s. These results do not establish real F-22 performance.
The [measured high-AoA plot](../tests/f22/high-alpha.png) shows incidence and
airspeed with healthy versus failed vector actuators, including the unload at 8 s.

`tests/f22/starting-source.zip` and `starting-hashes.json` preserve the initial
source. Iteration images are kept separate from final accepted captures.
`tests/f22/delivery.json` records test/build results, exact installed executable
identity, source hashes and native interaction checks.
Thirteen development captures cover the shape, cockpit, exhaust on/off, bays,
gear, damaged controls, external loadout, high-AoA pose and hangar. Appearance
captures ran alongside compilation; their FPS overlays are not performance
evidence. Final packaged-app cockpit/exterior captures are kept separately in
`tests/fleet/f22-release-final`, after the final heading/zoom polish and false
stall-warning correction. Each capture
manifest identifies its own executable and source snapshot.
Three additional final-source views in `tests/fleet/f22-final-geometry` verify
the opened intake cores, exterior and corrected cockpit heading before packaging.

The final optimized build completed in 2m 50s, was installed in `Nord Flight.app`,
and passes strict/deep ad-hoc signature verification. Both packaged-release flight
studies exactly match the development studies. The final cockpit capture shows
normal attachment at 4.4 degrees AoA without the former false stall warning.
The warning fix excludes unused and detached F-22 panels from attachment telemetry;
it does not change aerodynamic forces. Artifact hashes and native interaction
coverage are recorded in `tests/f22/delivery.json`.

```sh
cargo test --offline
target/debug/gripen-flight --study-f22 > tests/f22/flight-study.json
target/debug/gripen-flight --study-fleet > tests/f22/fleet-equivalence.json
python3 tests/fleet/capture.py --aircraft f22 --views exterior cockpit rear front top bays gear damage --output f22-check
```

Regressions cover vectoring force/health, incidence/unload commands, full 6DOF
high-AoA maneuvers, direct-control departures, scheduled flaps and jams, force
dissipation, loadout mass/balance/drag, takeoff, gear contact, a full approach,
level acceleration and a sustained bank. Existing damage, engine-out and mesh
tests also exercise the F-22. Screenshots establish appearance, not flight-test
fidelity or a matched FPS benchmark.

## Public references and limits

- [USAF F-22 fact sheet](https://www.af.mil/About-Us/Fact-Sheets/Display/Article/104506/f-22-raptor/): public dimensions, propulsion class, internal armament and aircraft overview.
- [USAF Museum F-22A](https://www.nationalmuseum.af.mil/Visit/Museum-Exhibits/Fact-Sheets/Display/Article/196040/lockheed-martin-f-22a-raptor/): aircraft proportions and two-dimensional vectoring architecture.
- [Official DVIDS demonstration-team photograph](https://www.dvidshub.net/image/8891358/f-22-demonstration-team-arrives-heritage-flight-2025): airframe/underside photographic reference.
- [NASA high-alpha/thrust-vectoring background](https://www.nasa.gov/reference/f-18-harv/): general physical principles and research context; its HARV results are not F-22 performance data.

This is a refined procedural game aircraft. Exact production surface contours,
certified control laws, inlet/engine coupling, aerodynamic cross-derivatives and
real-aircraft vulnerability are not publicly calibrated here. The report separates
measured simulator behavior from these assumptions; it does not claim perfection
or a certified F-22 flight model.
