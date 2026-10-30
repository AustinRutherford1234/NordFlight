# Carrier nose-over and roll stability fix — 2026-10-30

## Reproduction and cause

The reported behavior was violent roll after slowing/stopping with controls released. The user also observed excessive nose compression and main-wheel unloading during deceleration.

A deterministic headless reproduction started the Hornet just above the moving deck, at approximately 70.8 m/s deck-relative speed and 4 m/s sink, with manual flaps at the existing carrier setting. A neutral-control catch with -3° initial bank and +6 m lateral offset reached **18.62° roll** and crashed into the deck. Other off-centre catches oscillated and unloaded a main wheel. A sampled baseline trace showed 0.295 m nose compression at 0.5 s and -6.51° pitch with roughly 10.9° roll at 3.5 s in a separate neutral-bank/+6 m case.

The arresting force and its rotational point velocity were evaluated at the deployed hook tip, about two metres below the aircraft reference, treating the shank as a rigid cantilever. That generated a large nose-down and rolling reaction moment through a hook represented visually as pivoted. This was a load-path modeling issue; changing the whole airframe's bending stiffness would not correct that applied moment.

## Change

`carrier::HOOK_PIVOT` now names the existing rig pivot, body coordinates **(0, 0.15, 5.16) m**. The deployed hook tip is still used for engagement, payout geometry and wire rendering. The rigid aircraft's arresting reaction and its dissipated-work velocity use the structural pivot relative to the current CG.

This is a **reduced-order articulated-hook approximation**. Hook joint inertia and explicit lateral/vertical hinge-damper moments are not modeled by this correction. A public [Boeing arresting-hook patent](https://patents.google.com/patent/US12116116B2/en) describes pivot and damper interfaces; it motivates separating the hook point from the airframe interface, but does not validate this simulator's F/A-18F geometry or joint loads. Pivot uncertainty and the limitation are recorded in `PARAMETERS.md`.

No nose/main spring, damping, tyre-friction, steering or FBW parameter changes were retained. Suspension/tyre tuning was explored but did not address the complete failure. The existing arresting force schedule and stroke/energy limits remain in use. Aircraft translation/rotation continue through the normal physics integration; no bank clamp, attitude snap or crash suppression was added.

## Automated evidence

The full existing suite plus the new tests passed **670 tests, zero failures**. New regressions cover:

- 18 neutral-control landing/arrest/12-second-hold cases: FBW enabled and disabled, initial bank -3° / 0° / +3°, lateral offset -6 / 0 / +6 m. Every case caught a wire and remained uncrashed.
- Across the measured corrected matrix, maximum overall roll was **3.61°**, and maximum roll below 3 m/s deck-relative speed was **1.03°**. Each case ended below 1° roll, below 0.02 rad/s roll rate, with positive compression on all three wheels.
- The matching -3° bank / +6 m / FBW-enabled case reached **3.42°** peak roll with no crash.
- A selected off-centre landing agrees at 60, 120 and 240 Hz call rates within 1 cm position and 0.01 m/s velocity. The existing solver still uses bounded physical substeps.
- Arresting force power at the application point is negative, and absorber work agrees with the opposing rigid-body force/moment work. This checks dissipation and torque consistency for the reduced model.

The regression limits were tightened to 5° overall / 2° after slowing. The final focused rerun passed all three new tests with zero failures. These conditions are simulation regressions, not certification of real Super Hornet arrestment limits or coverage of every approach/configuration.

## Native review and package

The user approved a short in-app review. `tests/carrier-roll/capture.py` captures the same neutral-control, +1° bank / +6 m offset landing at 0.2, 2, 4 and 12 seconds. The four views inspect touchdown, deceleration, slowing and rest, including gear contact. The harness pauses/restores any existing flight and keeps logs/captures local under the ignored test directory. This is a scripted native physics/render review, not the user's interactive flight review.

All four native views were captured and inspected. No fatal renderer errors or landing assertions occurred. Main-wheel-first touchdown was followed by a stable three-wheel attitude during deceleration and after stopping. Existing glTF hierarchy warnings remain outside this physics fix.

| Time | Deck-relative speed | Roll | Pitch | Nose compression | Left / right main compression |
|---|---:|---:|---:|---:|---:|
| 0.2 s | 70.930 m/s | -0.754° | +3.342° | 0 cm | 26.5 / 22.2 cm |
| 2 s | 38.286 m/s | +0.542° | -0.505° | 5.45 cm | 13.0 / 15.7 cm |
| 4 s | 0.035 m/s | +0.149° | -0.940° | 9.21 cm | 13.5 / 14.3 cm |
| 12 s | 0.042 m/s | -0.044° | -0.862° | 8.71 cm | 14.3 / 14.0 cm |

`./build-mac.sh dev` rebuilt and updated the existing `Nord Flight.app`. Strict deep code-sign verification passed. Built and packaged ARM64 executables share UUID **5E598DFD-3D16-34A7-BC94-F6903FDE6F8A**. Relaunch an already-running game to load this executable.

The user's own approach/configuration remains an interactive follow-up. The scripted tests establish the reproduced neutral-control failure is resolved in the specified conditions, not that every possible touchdown or the exact real-aircraft hook dynamics are validated.
