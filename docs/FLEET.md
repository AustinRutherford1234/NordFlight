# Player aircraft fleet

This update adds four selectable player aircraft alongside the existing Gripen:
Su-27SM, F-35B, F-22A and F/A-18E. These are original procedural game models,
refined after an initial native capture pass. They are not scanned replicas or
validated reproductions of military flight-control software. Public dimensions
guide their proportions; aerodynamic derivatives, inertias, vulnerability,
propulsion installation and control laws are gameplay estimates.

## Selection and controls

Open **F6 / Aircraft Hangar**, select an aircraft card and press **Start / Reset
Scenario**. Opening the hangar pauses flight. Cancel restores the previous pause
state, including when entering from the BVR setup screen. Selecting an aircraft
resets the current atmospheric scenario. The choice carries through subsequent
takeoff, approach, practice, close-combat and BVR resets. AI aircraft remain in
Gripens. Spacecraft physics and scenarios remain separate; starting from the
hangar while in a spacecraft starts the runway scenario.

| Control | Function |
|---|---|
| F6 | Aircraft hangar |
| Alt+F6 | Existing approach assistance |
| T | Fly-by-wire on/off; direct controls permit departures and post-stall flight |
| Y, F-35B | Toggle STOVL conversion below 150 m/s |
| Alt+Y, F-35B | Airbrake; Y remains the airbrake in other aircraft |
| Alt+B | Manual weapon-bay control on F-22/F-35B |
| Alt+J | Jettison the modeled missile stores |
| Alt/Option+click | Cockpit switches and radar controls |
| Shift / Control | Full / zero throttle, as before |

New cockpit switches include individual engine fuel valves, jettison, weapon
bays, and F-35B conversion where applicable. Existing camera, radar, map,
countermeasure and missile shortcuts remain available. The hangar button lives
in the header rather than covering the runway readout.

## Distinct models and flight behavior

| Aircraft | Airframe and systems |
|---|---|
| Su-27SM | Long blended forebody, widely separated twin nacelles, twin tails, blue finish, conventional stabilators, dorsal brake, teal cockpit with two digital displays and working analog gauges |
| F-35B | Broader body/chines, forward cockpit, single engine, rotating aft nozzle, animated lift fan, fitted intake and outlet doors, wide segmented cockpit display, side stick, gun pod and main weapon bays |
| F-22A | Broad diamond wing, canted twin tails, twin rectangular vectoring nozzles, main and side weapon bays, side-stick cockpit and additional flight/system readout |
| F/A-18E | Extended leading-edge roots, carrier-style wing and twin tails, two engines, conventional stabilators, LEX spoilers and three multifunction displays |

All use the existing double-precision six-degree-of-freedom integration and
configured timestep. The selected aircraft supplies wing area, dimensions, mass,
fuel capacity, inertia, thrust, envelope limits and surface geometry. Atmospheric
gravity, rotating-frame terms, collision handling and the terrain remain shared.
Successive integration steps remain serial. No aircraft's physics is replaced
with an animation or kinematic flight path.

Control loads use local relative flow, including angular velocity at each panel,
nonlinear incidence/deflection response, compressibility, drag and force lever
arms. FBW allocates commands against the selected aircraft's actual moment
derivatives. The conventional-tail pitch model separates static margin,
rotational damping and the deep-stall restoring moment. These coefficients are
tuned for plausible handling, not calibrated against flight-test data.

The Su-27 post-stall regression disables FBW, pulls with normal stick commands,
unloads during deep separation, then makes small recovery corrections. It checks
that the aircraft passes 90 degrees angle of attack and returns below 25 degrees
within the trial. There is no production cobra key or direct orientation update.

F-22 vectoring moves toward a finite nozzle-angle command with a rate limit.
Thrust acts at each engine's position and produces forces and moments. Losing an
engine creates asymmetric thrust; a failed nozzle actuator retains its angle.
The subsequent [F-22 refinement](F22_RAPTOR.md) replaces its shared initial
geometry/polar settings with aircraft-specific shapes, swept hinges, wing
parameters, low-speed incidence control and combined panel/vector allocation.
It also adds a clean/internal versus two-external-missile hangar loadout.

F-35B conversion is rate-limited over approximately three simulated seconds.
The model distributes available thrust between the aft nozzle, fan and roll
posts. Pitch control transfers lift fore/aft; differential post forces provide
roll and yaw couples. FBW adds bounded attitude/rate feedback during conversion.
Hover therefore depends on weight, density, power and system health. The default
fuel load is lower than the conventional jets' fuel fraction to permit a useful
hover margin. This is a coupled gameplay model of the LiftSystem architecture,
not an engine-cycle or CFD model.

## Damage, stores and geometry

The failure arrays now have sixteen slots. Gripen's original nine retain their
mapping. New aircraft use separate left/right stabilators, trailing flaps,
ailerons, canted rudders and leading flaps. Su-27 adds a dorsal brake; Super
Hornet adds individual LEX spoilers. Raptor, Lightning and Super Hornet braking
also uses opposing control deflections.

A jam freezes the actual surface angle; command authority is lost but fixed
aerodynamic loads remain. A detached panel loses its loads and attached mesh,
leaves a cutout in the fixed wing, and triggers pooled debris. Rendering,
collision hinges and force evaluation share the same panel definitions. The
damage schematic uses the selected aircraft's silhouette and component labels.
Individual engine, nozzle-actuator and lift-fan failures feed propulsion and
the failure readout. Existing fire, smoke, breakup and seeded hit outcomes remain.

Stores contribute mass, balance and drag; asymmetric expenditure/jettison changes
the modeled center of mass. Missile stations and cannon muzzle locations differ
by aircraft. Inventory code supports more than two rounds of each missile type.
Gun ammunition and cadence differ, while the existing game's accelerated cannon
projectile speed and shared seeker/sensor behavior are retained. Missile names
represent compatible weapon families; this update does not add separate
manufacturer-calibrated missile variants or aircraft stealth signatures.

The closed fuselages have fitted cockpit apertures and precisely clipped gear,
fan and weapon-bay openings. Gear uses sequenced doors, folding wheels, braces,
spring/damper contact and nose-wheel steering. Fan and bay covers are separate
moving meshes. Internal stores use compact stowage geometry. These mechanisms
are aircraft-inspired game approximations; exact maintenance geometry, complete
avionics menus and photographic cockpit replicas remain beyond this first fleet
release's fidelity.

## References

Photographs and public system descriptions were used as proportion/mechanism
references, not redistributed as game textures:

- [Su-27SM RF-92211 photograph, AirTeamImages](https://www.airteamimages.com/sukhoi-su-27_rf-92211_russia-russian-air-force_360114).
- [US Marine Corps F-35B vertical-landing photograph](https://www.marines.mil/Photos/igphoto/2001479746/).
- [Rolls-Royce LiftSystem architecture](https://www.rolls-royce.com/products-and-services/defence/aerospace/combat-jets/rolls-royce-liftsystem.aspx).
- [USAF F-22 fact sheet and photographs](https://www.af.mil/About-Us/Fact-Sheets/Display/Article/104506/f-22-raptor/).
- [US Navy Super Hornet overview and photographs](https://www.navy.mil/Resources/Fact-Files/Display-FactFiles/Article/2383479/fa-18a-d-hornet-and-fa-18ef-super-hornet-strike-fighter/).
- [DVIDS Super Hornet launch photograph](https://www.dvidshub.net/image/6440695/f-18e-super-hornet-launches-off-flight-deck).

## Validation evidence

The October 18 fleet revision passes **320 tests, zero failures**. Focused checks
cover panel authority, persistent jams/detachment, engine-out moments, frozen
nozzle actuators, STOVL conversion, full 6DOF hover and lift-fan failure, shaped
collision skins, gear stowage/support, full weapon inventories and hangar resets.
All five aircraft remain finite and airborne in the fixed-step flight study.
The Su-27 trial reaches **99.89° AoA**, then reaches **−19.31° at 12 seconds**,
without a crash. This is a scripted gameplay-model result, not a flight-test claim.

The final optimized build completed in **3m 11s**. The packaged native app passes
`codesign --verify --deep --strict`. `native-delivery/manifest.json` identifies
the development executable and source hashes used for the native exterior,
cockpit, gear, fan, weapon-bay, damage and hangar images. Posed captures establish
appearance; they are separate from flight tests and do not establish FPS.
Fourteen native captures were inspected. The signed app was launched and F6,
F-35B selection, Start/Reset and cycling into its cockpit were exercised through
the native UI. The app is left paused. `delivery.json` records the installed
binary hash, prior executable backup, test logs and source/capture identity.

`tests/fleet/starting-source.zip` and `starting-hashes.json` preserve the starting
source. `tests.log` contains the full regression result. `flight-study.json`
records fixed-step controlled flight and the post-stall trial. Native images
are captured by `tests/fleet/capture.py` using the actual application renderer.
Initial captures are kept separate from the accepted revision.

Reproduction:

```sh
cargo test --offline
target/debug/gripen-flight --study-fleet
python3 tests/fleet/capture.py --output my-fleet-captures
```

`NORD_AIRCRAFT=su27|f35b|f22|fa18e|gripen` and `NORD_FLEET_CAPTURE` are developer
fixtures. They do not replace the in-game hangar or alter ordinary simulation
timing. Final measured results and artifact identity are recorded in
`tests/fleet/delivery.json` after packaging.
