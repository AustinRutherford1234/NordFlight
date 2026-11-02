# Hornet drag, wheel brakes and naval standoff ranges

## Surface speedbrake

Y toggles the Hornet surface-speedbrake command; the help text identifies it.
No separate airbrake panel or mesh is added. Ailerons move trailing-edge up,
trailing flaps down and rudders flare oppositely, using their existing actuator
and animation paths. Deflection ramps over two seconds at full hydraulic power.
Roll/yaw commands retain priority over the drag-producing bias. Manual half/full
flaps inhibit the airborne surface function; Mach and angle-of-attack scheduling
reduce deployment outside the modeled operating region. Returning the command to
off retracts the bias through the same physical actuators.

Public architecture reference: Navy A1-F18EA-NFM-000, section 2.10.6, reproduced at
https://studylib.net/doc/27350655/f18-ef-000
It describes a distributed surface function, stabilization, operating restrictions
and a roughly two-second command transition. The actual software's mixing gains,
airborne TEF bias, fade widths and drag areas here are documented simulation
estimates. This does not reproduce the proprietary FCS schedule. Spoilers and
independent stabilator feed-forward are not newly modeled; the existing FBW
responds to the resulting aircraft forces/moments. In half/full flaps on the
ground the real aircraft uses spoilers only; this implementation does not invent
an invisible spoiler force where no spoiler is modeled.

The previous immediate +0.05 Cd switch term is removed. Additional control drag
uses achieved flap/aileron/rudder deflections, projected area and squared sine of
deflection, and is applied at the associated airframe locations. Existing lift,
induced drag and hydraulic/actuator limits remain. This deliberately lets actual
jammed or unpowered surfaces affect achievable braking rather than granting the
switch an unconditional drag force.

Gear drag uses an estimated full-extension equivalent drag area of 2.8 m2 plus
0.8 m2 times open-door fraction. The gear term grows with squared extension and
normal dynamic pressure. This replaces 0.02 Cd (0.93 m2 on the 46.5 m2 reference).
These are calibration choices rather than flight-measured Hornet coefficients.

At rho=1 kg/m3 and 150 m/s, the controlled force fixture recorded 21.12 kN clean,
49.78 kN with full surface brake and 52.62 kN with gear down. These are fixed-state
model comparisons, not trimmed drag-polar measurements or real-aircraft data.

## Wheel braking

Only Hornet main wheels receive the stronger brake command. Estimated dry paved
braking friction is 0.85 and carrier-deck braking friction 0.70; command effort is
0.90 instead of the former shared 0.60. Forces remain bounded by normal load,
combined-slip grip and the impulse needed to stop without reversing wheel motion.
Normal unbraked lateral response is preserved to avoid changing the previously
validated arrest/taxi stability. Other aircraft retain the shared original brake
command and friction settings. B remains the wheel-brake control.

## Carrier-defense ranges

New raids spawn at 300 km from the moving carrier, with existing four-aircraft
composition, bounded active entities and bomber-salvo-based wave progression.
Bombers request a physical slowdown ahead of release, open bays within 155 km,
and can begin releasing at 150 km. They retain 24 missiles each and the five-second
release interval, so the whole salvo occurs across a finite inbound flight leg.
They withdraw after release; aircraft retirement is beyond 325 km.

The representative anti-ship weapon retains its 290 m/s cruise cap and existing
navigation/seeker/collision path. Estimated powered endurance increases to 620 s,
with a finite 720 s flight lifetime. This supports the longer standoff flight and
maneuver reserve; it is not a claim about a specific Kh-series missile. Its
149 km supported mission fixture hit the moving carrier at 537.91 s, at about
14.27 m altitude and 155 m from the ship center (bow/hull collision).

The old independent 22 km SAM aircraft gate is removed. Standard-class outer
slant range is estimated at 70 km and ESSM-class at 30 km. A bounded average-speed
and time-to-go estimate also checks target aspect and altitude/climb cost. This
rejects many receding/crossing shots before the outer cap, rather than guaranteeing
that every target at maximum range can be reached. Radar detection, horizon,
obstruction, track support, ammunition and the existing concurrent-shot limits
are still required. SAM propulsion and terminal flight physics are unchanged.
The actual weapon-path fixtures intercepted an approaching 480 m/s bomber from
65 km in 63.475 s and an approaching 450 m/s target from 28 km in 32.833 s.

The naval map initially opens wider and permits zooming out farther to accommodate
300 km raid geometry. Sensor-limited play still hides undetected enemies. Tests
that previously assumed initial detection at 120 km now distinguish distant
undetected raids from deliberately positioned in-range sensor fixtures.

## Validation and delivery

All 778 game tests passed on the final source (95.28 seconds), covering the new
surface function, drag comparisons, hydraulic/envelope limits, FBW transient,
stronger tyre forces, no brake reversal, outer SAM engagements, long cruise flight,
300 km raid progression and wide-map projection. Existing carrier neutral-arrest,
catapult, slow-motion/ground-warp, missile tracking, HUD and other-aircraft tests
also passed. The five-second FBW transient ended at 106.27 m/s with the brake
versus 109.87 m/s without it, roll -1.53 degrees, peak angular rate 0.063 rad/s.
This is a matched initial-state transient, not an equilibrium flight measurement.

The approved native review used 69.78 seconds of its two-minute budget and
restored the previous session. Frozen clean/extended surface views were inspected
in the actual renderer. Extended-state telemetry: deployment 1.0, ailerons
-0.4014 rad each, TEFs +0.2300 rad each, rudders +0.3491/-0.3491 rad.
Two scripted 12-second physical runway rollouts began at 20 m/s with engines off.
The braked aircraft traveled 42.96 m and ended effectively stopped; the unbraked
comparison traveled 225.56 m and retained 17.63 m/s. These are not before/after
measurements of the previous brake implementation. Neither aircraft crashed.
Native evidence is local under `tests/hornet-drag-review/`; the capture script is
`scripts/fa18f/capture_drag_brakes.py`. Views are scripted endpoints, separate from
the user's manual flight review. No soak or performance-gain benchmark was run.

The existing Mac app was rebuilt with `./build-mac.sh dev` and passed
`codesign --verify --deep --strict`. Relaunch the app to load the update.
