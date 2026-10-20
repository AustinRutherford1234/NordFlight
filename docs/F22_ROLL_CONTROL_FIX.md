# F-22 roll direction and FBW response — October 19, 2026

This revision corrects the roll-sign conflict introduced in the differential
TVC update and increases FBW's commanded roll rate. The previous low-speed
test was insufficient: it passed while normal-flight direct controls could
reverse, and it did not catch the controls opposing one another.

## Confirmed cause

The keyboard maps Q to −1 and E to +1. Body +Z angular velocity is a left roll;
the established pilot-to-actuator mapping is therefore `actuator.z = -input.z`.
The previous update added an F-22-only exception to that mapping. Meanwhile,
its stabilator and nozzle roll mixing had opposite moment signs from the
flaperons and ailerons. Thrust and aerodynamic contributions vary differently
with speed and incidence, so a low-speed test did not establish correct
direction at cruise.

The new panel regression reproduced the conflict directly: at 200 m/s in
attached flow, one stabilator's roll derivative was −442,215 N·m per normalized
actuator unit, opposing the wing surfaces. The two-second baseline speed
sweep also reproduced Q rolling the wrong way at 240 m/s with FBW off.

## Correction

- Restore the established `-input.z` direct-control mapping for the F-22.
- Reverse the stabilators' differential roll mix and the differential nozzle
  mix so their roll moments agree with the flaperons and ailerons in attached
  flow. Common pitch mixing is unchanged. For normalized actuator pitch `p`
  and roll `r`, nozzle targets are now `clamp(-p-r, -1, 1) × 20°` on the left
  and `clamp(-p+r, -1, 1) × 20°` on the right.
- Retain current-state roll-effectiveness estimation and its fixed-load bias
  for high-incidence mixed deflections. Preserve the tested neutral-reference
  pitch/yaw allocation model; applying the local update to every axis caused
  excessive pitch overshoot in the high-AoA regression and was rejected.
- Increase the low-AoA FBW roll-rate target from 1.55 to 2.80 rad/s, about
  89 to 160°/s at full input. The existing high-AoA reduction remains. This
  raises a rate request, not physical thrust, surface area or torque limits.

The 1.30× stabilator-effectiveness setting, independent nozzle limits,
actuation rates, damage, finite thrust and surface meshes remain intact.
These control-law parameters are simulator estimates, not calibrated real
F-22 flight-test data.

## Measured response

The developer diagnostic `--study-f22-roll` holds each key's full command for
two simulated seconds at 6,000 m, starting at 80, 160 or 240 m/s. The 80 m/s
case begins at 31.5° incidence; the faster cases begin at 5.7° and 3.4°.
The same diagnostic was run before changing the control signs.

| Start speed | Mode | Key | Before body roll rate | Corrected body roll rate |
|---|---|---|---:|---:|
| 240 m/s | FBW off | Q | −12.0°/s — wrong direction | +361.5°/s — left |
| 240 m/s | FBW off | E | +10.0°/s — wrong direction | −361.6°/s — right |
| 240 m/s | FBW on | Q | +70.0°/s | +154.1°/s |
| 240 m/s | FBW on | E | −72.1°/s | −153.8°/s |
| 80 m/s | FBW on | Q | +8.6°/s | +57.0°/s |
| 80 m/s | FBW on | E | −5.4°/s | −52.9°/s |

These are transient simulator results after two seconds, not aircraft maximum
roll-rate specifications. The raw-control rates are not FBW-limited. All 12
speed/key/mode combinations finish finite, without a crash, and with the
expected roll direction. Complete samples are in `tests/f22-roll-direction/`.

All **351 application tests pass**. New regressions require the six roll
surfaces to agree on moment direction in attached flow; Q/E to roll correctly
with FBW on/off across all three speeds; and FBW to exceed 120°/s in the faster
test cases. Existing torque, saturation, jam, engine-out, takeoff, landing and
high-AoA tests pass. The high-AoA pull remains near 50°; the assisted approach
lands at about 2.32 m/s sink.

The former minimum healthy-versus-failed *peak AoA gap* assertion was replaced
by checks of actual nozzle use, a frozen failed-nozzle trace and recovery.
Peak AoA is limited by the command law, so demanding a minimum difference
between those peaks is not an authority measurement. The independent force
and moment-derivative tests continue to check actual vectoring authority.

Other aircraft's fleet-study outputs remain exactly equal to the preserved
installed baseline. Release diagnostics are compared against the accepted
development candidate. Native Q captures use simulated cruise states with
FBW off and on. Delivery hashes, signature and capture identities are recorded
in `tests/f22-roll-direction/delivery.json`.
