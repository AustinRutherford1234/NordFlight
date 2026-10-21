# F-22 AIM-9X Sidewinder

The F-22's infrared weapon slot now launches a distinct AIM-9X model. Select
the F-22 in the hangar, select the infrared weapon with **O**, acquire a target,
and fire with **Space**. It uses the existing two Sidewinder stations, ammunition,
bay animation, damage resolver, missile camera, audio and countermeasure paths.
Practice, close combat and BVR use the same profile. AI Gripens do not receive it.
Other aircraft keep their existing generic IR dynamics; the F-35B and Hornet's
previous AIM-9X labels now read generic IR MISSILE so this addition is F-22-only.

## Public specifications used

[NAVAIR's AIM-9X product page](https://www.navair.navy.mil/product/AIM-9X-Sidewinder)
publishes:

| Property | Implemented value or capability |
| --- | --- |
| Launch mass | 84.37 kg, also used in aircraft store mass/balance |
| Length | 3.02 m, using NAVAIR's listed overall length |
| Body diameter | 0.127 m (5 inches; NAVAIR rounds to 0.13 m) |
| Overall fin span | 0.45 m |
| Seeker | All-aspect infrared; distinct high off-boresight limits |
| Countermeasures | Improved IR counter-countermeasure behavior |
| Steering | Aerodynamic control plus powered thrust-vector authority |
| Warhead | Existing blast/fragmentation damage path |

NAVAIR describes a focal-plane-array seeker, thrust vectoring and, for Block II,
datalink guidance. The game retains its radar-support bridge when IR tracking is
unavailable; it does not provide omniscient tracking or a helmet-mounted sight.

## Explicit simulation estimates

NAVAIR lists **speed and range as classified**. This implementation does not claim
an exact real-world missile envelope. The 10 km launch gate is a gameplay limit,
not a guaranteed engagement range; launch speed, altitude, aspect, target
maneuvers, gravity and drag still determine whether the missile arrives.

The following are uncalibrated estimates: 80-degree seeker gimbal, 90 degrees/s
slew, 60-degree aircraft acquisition cone, 12 km seeker range cap, 40-g structural
cap, 20-degree equivalent jet-vane deflection, 4.5 s motor duration, 14 kN thrust,
20 kg propellant, aerodynamic coefficients and controller gains. Motor data uses
the existing short-range missile approximation. No artificial Mach clamp or
instant heading changes are used.

Jet-vane authority exists only during powered flight. Its lateral thrust demand
reduces axial thrust, and aerodynamic induced drag uses the aerodynamic portion
of the load. This permits stronger initial turns without unlimited post-burn
steering. The imaging-seeker approximation reduces each eligible flare's diversion
probability to 30% of the generic IR value and checks track recovery sooner
(0.25 s versus 0.75 s). Flares can still win; these are gameplay parameters, not
published probabilities. Sun glare and terrain masking remain active.

Regression coverage checks F-22-only assignment in both launch paths, acquisition
angles, mass, mesh dimensions, motor-dependent control authority, high off-boresight
tracking and nonzero but improved flare rejection. These checks validate the game
implementation, not real missile performance.

## Detailed visuals and actuation

The AIM-9X now has a 64-sided body with a curved, reflective seeker window,
section joints, identification bands, a tapered wiring cover, shaft bearings,
solid forward wings and tail fins, and a recessed exhaust throat. Mounted missiles
include neutral tail surfaces; airborne missiles have independently hinged tail
fins and four internal jet vanes. The forward wings and nozzle housing remain fixed.

The mechanism follows the public description of a tail-control actuation system
with jet vanes in [NAVAIR's initial production announcement](https://www.navair.navy.mil/node/7616)
and the four coupled fins/vanes described in [OPNAV M-8000.16 Volume II](https://www.secnav.navy.mil/doni/SECNAV%20Manuals1/8000.16.pdf).
Visible deflection follows the guidance controller's existing lagged, rate-limited
output in the missile's local frame. Opposite fins use opposite shaft rotation;
the vanes follow their corresponding tail-fin angles. The estimated 20-degree
limit is visual tuning, not a published AIM-9X actuator specification.

The animation represents aft-control pitch/yaw moments: the tail initially reacts
opposite the turn demand to rotate the nose into the turn. This is a presentation
mapping from the existing point-mass guidance model, not a new six-degree-of-freedom
missile model or a calibrated fin-angle solution. No weapon trajectory, damage,
seeker, motor impulse or authoritative simulation settings changed in this visual
pass. The exhaust has a fixed mounting and a deflected flow axis while powered;
its layered hot core, shock-cell modulation and translucent sheath fade toward
the end. Motor shutdown hides the plume.

Smoke uses one shared procedural soft texture, overlapping camera-facing lobes,
simulation-time expansion and fading, and persistent mesh buffers. It remains
bounded by the existing 48-point trail history and 64-missile visual pool. The
old thin debug line is suppressed for this missile. Pausing freezes controller
and plume motion; rotating the camera still correctly reorients the smoke cards.

`python3 tests/capture_aim9x.py` captures the model close-up, an actual off-axis
guidance demand from behind and beside the plume, and the longer smoke trail.
These are native renderer captures; their scenario poses are diagnostic setups.
