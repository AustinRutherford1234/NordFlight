# Ground and carrier time warp

Aircraft can use up to 10x time while landed. The existing period/comma controls
raise/lower the selected rate; slash returns to 1x. Rates above 10x remain capped
on the ground. Slow motion is unchanged. Low-altitude airborne flight, crashes,
ejection and active catapult shots retain their existing limits.

While accelerated time (>1x) is active and the landed aircraft's full velocity
relative to its supporting surface is below 0.5 m/s, a parking convenience holds
its position and attitude relative to that surface. Runway velocity is zero;
on the carrier it is the local deck velocity, including modeled damage motion.
This deliberately prevents creeping, suspension oscillation and wind/thrust drift
while waiting. Return to 1x to taxi from a held stop. Moving taxi traffic is not
snapped to a stop. Catapult firing bypasses the hold.

This uses normal bounded physics and combat ticks, not orbital coasting or a
clock-only skip. Fuel/engines, enemies, missiles, carrier motion and damage still
advance. Damage or ejection cannot be undone by the hold, and a disabled carrier
releases it. Existing per-frame work bounds still apply: actual speedup can be
lower than selected on a busy scene or at a very small configured timestep.

Automated checks cover ground-rate limits and slow motion, exclusion of flight,
crash and catapult shots, relative-speed threshold, moving-deck transport,
disabled-carrier release, and the real application step loop at 10x for runway
and deck starts. Live-app inspection is separate from these headless tests.

Initial validation on November 1, 2026: all 768 automated tests passed (63.27 seconds),
including the new ground-warp tests and the existing neutral carrier arrest,
catapult, slow-motion, orbital-coast and render-rate convergence regressions.
The 10x application-loop fixture advances 10 simulated seconds over 60 frames
at 60 Hz; both environment and aircraft clocks agree. On deck, the held aircraft
travels more than 120 m in world coordinates without shifting in ship coordinates.

## Physics priority at 10x

At the full 10x ground rate, window updates become event-responsive with an
approximately 15 Hz idle display cadence. This is a maximum idle refresh cadence,
not a rendering deadline: normal physics/combat work finishes before the next
image, and input/window events can wake the application earlier. The saved
focused and unfocused update policies return when warp is reduced, flight leaves
the ground, the player pauses, or the aircraft crashes/ejects. Visual quality and
physics step sizes are unchanged.

The clock allows up to 0.25 wall seconds and 2,560 bounded physics ticks per
frame in this mode, so fewer rendered frames do not force the former 256-tick
limit to throw away requested simulation time. At ordinary rates it retains the
original 0.1-second/256-tick bounds. Host stalls beyond the bound are discarded;
unlimited catch-up debt is never accumulated. Extreme workloads can still run
below 10x; the mode prioritizes simulation rather than guaranteeing hardware
throughput.

Updated validation: all 771 automated tests passed (46.41 seconds). Clock-only
tests sustain the requested 10x at 5/10/15/30/60 Hz with 1 ms, 1/240 s and 1/60 s
configured intervals. The application-loop fixture now covers both runway and
deck at 15 and 60 Hz, including 1 ms ticks at 15 Hz. One anchor is retained per
physics batch to avoid accumulating world/deck round-trip errors across thousands
of ticks. Mode tests verify the original window policies are restored on pause,
rate reduction and takeoff.

A single 6.44-second fullscreen native sample (2880x1800, carrier-ready cockpit,
no raid, selected 10x) measured 2.19 rendered FPS and 5.43x actual simulation
speed. The simulation CPU span averaged 417 ms per rendered frame versus a
457 ms average frame interval (about 91%). Nested collision checks accounted for
401 ms/frame; these must not be added to the containing physics/simulation span.
This demonstrates physics priority, not attainment of 10x or an A/B performance
gain. The scene is limited by carrier-contact collision work, not a rendering
deadline. Metrics are local at `tests/ground-warp-priority/profile.json`.
The short diagnostic restored the prior running flight and took 16.74 wall seconds
including startup and warm-up. No soak was run.
