# AI aircraft and pilots

Every enemy, allied wingman and target-practice aircraft runs the same
`physics::simulate` six-degree-of-freedom plant as the player. Each has its own
position, velocity, quaternion, angular rates, actuator state, engine spool,
fuel, aerodynamic attachment history, gear and persistent component damage.
The AI commands throttle, pitch, roll, yaw and airbrakes. It does not set a
turn rate, force a minimum speed, rotate the aircraft directly or pin altitude.
Scenario spawning and collision response can still initialize/change physical
state, just as they can for the player.

The plant includes the same atmosphere, wind, gravity, mass/fuel coupling,
installed thrust, lift/drag, moving-surface forces, stall behavior, rate damping,
FBW allocation, actuator limits, ground collision and structural limits. An
engine failure removes thrust; it does not assign a slower cruise speed. A jam
retains its frozen aerodynamic load, and a detached surface loses its forces.
Rendered surface angles and missile-hit jam angles use the actual actuators.

These remain the game's tunable Gripen-inspired aerodynamics, not a calibrated
Saab aircraft-performance or pilot-training model.

## Pilot control and safety

The pilot converts a desired flight direction into bank, flight-path and load
commands, then sends those demands through the normal FBW system. Throttle
balances estimated drag/climb demand and speed error against the existing
engine model, including spool lag and the available afterburner. It cannot
restore lost kinetic energy or accelerate without thrust/gravity.

The pilot normally requests at most 8.5 G while the shared FBW retains its
9 G / approximately 0.32-radian positive-incidence protection. Those requests
do not impose a hard physical G cap; actuator lag, damage and existing physics
can cause transients. Stall/low-speed recovery unloads the aircraft, levels the
wings and requests full power. Terrain recovery uses current clearance, descent
rate and a five-second terrain lookahead. These are strong recovery commands,
not a guarantee of survival when altitude or control authority is insufficient.

All aircraft receive the same simulation substeps, at most 1/120 second, and
pilot-control updates on every substep. AI tactical and awareness evaluation
runs every combat tick. There is no reduced-fidelity distant-aircraft mode.
Pause and the existing slower-than-real-time timestep behavior are preserved.

## Awareness and tactics

Visual acquisition requires range, view direction, terrain visibility and a
sun-glare gate. Radar acquisition requires working radar/avionics, terrain LOS,
a forward scan volume and the modeled search beam, or an existing nearby track.
BVR uses the same long-range signal model as the player's aircraft radar rather
than the short-range missile-seeker signal curve. Observations have delay and
position error. Lost contacts coast through stored position/velocity estimates
and expire; the AI does not continually read the unseen player's position.

The passive RF cue supplies a bearing toward a radiating player radar, without
granting a ranged contact. Missile defense requires an active seeker pointing
toward the aircraft or a nearby visible burning missile. A missile's hidden
target assignment does not by itself warn the pilot. These are bounded game
sensor approximations, not an RF/optical propagation simulation.

Close combat uses lead/lag pursuit, overshoot avoidance, energy extension and
vertical lead maneuvers when the pilot has an energy advantage. Nearby aircraft
produce predicted-closest-approach avoidance commands. Wingmen use offset
support positions rather than trying to occupy the same intercept point.
The cannon fires along the aircraft's nose with difficulty-dependent dispersion;
the pilot must fly the nose onto the firing solution.

BVR pilots distribute target assignments, share fresh contacts with a paired
wingman, fly intercept/formation goals, respect launch envelopes and support
their missiles before active guidance. Supporting shooters crank within the
radar field of view; threatened pilots notch relative to the detected threat
bearing or drag away at longer range. Avoidance and recovery can override
tactical demands through normal controls. Wingman contact sharing assumes a
working game datalink; radio propagation and voice communications are not modeled.

Training, Balanced and Hard change observation/control accuracy, reaction,
tactical choices, firing discipline and countermeasure timing. They share the
same physical aircraft and weapon power. Close-combat opponents carry the same
three IR/three radar missile inventory at each difficulty; firing policy still
differs. Aircraft continue fighting when damaged, low on fuel or out of weapons
until physical failures prevent them. Automated return-to-base/landing is not
part of this update.

## Diagnostics

Run the native executable with `--study-ai` from the project folder:

```sh
./'Nord Flight.app'/Contents/MacOS/gripen-flight --study-ai
```

It writes `tests/ai-flight-study.json` and `.csv` for six 60-second physical
profiles: straight flight, a turn, high-incidence recovery, engine failure,
jammed elevon and detached elevon. The CSV records airspeed, Mach, altitude,
G, incidence, bank, pitch, fuel, throttle, controls and specific mechanical energy.

`tests/ai-combat-study.json` and `tests/ai-combat-flight.csv` cover target
practice, all three dogfight difficulties and all four BVR formations at each
difficulty for both mission types (28 cases). AI intent, flight loads, controls,
fuel, survival and physics-step counts are recorded without adding normal UI
overlays. The scripted player also uses the shared plant, but this is a
repeatable diagnostic trajectory rather than a human combat evaluation.
Reports overwrite the same files to avoid accumulating telemetry per run.
