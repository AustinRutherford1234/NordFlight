# F-22 differential vectoring and stabilator authority — October 19, 2026

**Superseded control-sign details:** the subsequent
[roll-control correction](F22_ROLL_CONTROL_FIX.md) fixes the conflicting
stabilator/nozzle signs and the direct-mode mapping described below. Its
speed-sweep evidence and FBW response measurements replace this revision's
low-speed-only direction claim.

The normal roll controls now command differential left/right F-22 nozzle
angles as well as the aerodynamic surfaces. Common nozzle deflection still
provides pitch control. No separate pilot binding is needed. FBW-off roll
commands use the same direction as FBW-on roll commands; an F-22-specific
direct-mode sign mismatch was corrected.

## Forces and allocation

For normalized actuator commands `p` (pitch) and `r` (roll), nozzle targets are
`clamp(-p + r, -1, 1) × 20°` on the left and
`clamp(-p - r, -1, 1) × 20°` on the right. Each nozzle retains its own
20-degree travel limit, 1 rad/s actuation limit and independent jam state.
Hydraulic failure freezes actuation. An unpowered engine produces no thrust
or vector-control authority. Combined commands can saturate one nozzle before
the other; pitch and roll cannot both obtain unrestricted maximum authority.

Thrust continues to act at the separate engine positions. Opposing vertical
thrust therefore generates roll through `r × F`; no angular velocity or
orientation is assigned directly. The nozzle axes remain two-dimensional:
no lateral thrust or independent yaw-vector axis was added. Forward thrust
falls with vector angle through the existing cosine projection.

The FBW model now includes both common-pitch and differential-roll moment
derivatives, per-engine clipping, actual angles, engine power, damage and CG.
Finite-difference tests compare that allocation matrix against the propulsion
model's moments about the same CG, including one engine off and one nozzle
jammed. F-22 aerodynamic allocation is linearized about the current actuator
position, with the corresponding fixed-load bias. This avoids assuming neutral
surface effectiveness when large high-AoA stabilator deflections change the
local force response.

Both horizontal stabilators use a **1.30× aerodynamic-effectiveness factor**.
This scales their modeled force, drag and moment contributions at a given
state and deflection. Geometry, travel, servo limits and failure handling are
preserved. FBW accounts for the stronger loads, so it may use less deflection
to achieve the same commanded rate. These are simulator tuning parameters,
not a claim of calibrated real F-22 control laws or flight-test derivatives.

## Validation

All **349 application tests pass**. Coverage includes differential torque sign,
independent travel/rate limits, jam isolation, combined-command saturation,
power/hydraulic/fuel loss, allocation derivatives, low-speed rolls in both
directions, and FBW-off direction. Existing takeoff, landing, sustained-turn,
high-AoA, stall/recovery and engine-out tests pass.

The expanded `--study-f22` diagnostic starts the roll cases at 80 m/s and
31.5° incidence, at 6,000 m, then holds a 60% roll command for one second:

| Case | Peak nozzle separation | Body roll rate after 1 s |
|---|---:|---:|
| FBW right | 18.64° | −11.16°/s |
| FBW left | 19.43° | +16.93°/s |
| Direct right | 24.00° | −14.69°/s |

These are finite-duration simulator measurements, not maximum roll rates.
The engine-out case remains finite but does not track this combined high-AoA
roll demand: asymmetric yaw/pitch and reduced control authority remain present.
It is not a promise of equivalent handling on one engine.

The existing high-AoA hold diagnostic peaks at 50.03° with vectoring and
45.98° with both nozzles disabled. The stronger stabilators improve the latter
from 40.84°. The regression's required vectoring benefit was updated from 5°
to 3° to reflect that intentional change; the measured benefit is 4.05°.
The assisted approach still lands, at 2.45 m/s sink versus 2.30 m/s before.

The Gripen, Su-27, F-35B and F/A-18E fleet-study results are exactly equal to
the preserved starting build. F-22 outputs intentionally change. Final release
results are compared exactly with the inspected development candidate.

Native captures use the actual flight state after the one-second roll trials,
rather than assigning nozzle angles solely for the screenshots. They verify
that the moving flaps and exhaust follow each engine independently while the
sidewalls remain fixed. Captures are sampled poses, not a per-frame animation
recording or performance benchmark.

Source/executable snapshots, tests, diagnostic output, signature verification
and capture identities are in `tests/f22-differential-tvc/`; `delivery.json`
identifies the accepted installed app.
