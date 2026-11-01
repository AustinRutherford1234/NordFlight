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

Validation on November 1, 2026: all 768 automated tests passed (63.27 seconds),
including the new ground-warp tests and the existing neutral carrier arrest,
catapult, slow-motion, orbital-coast and render-rate convergence regressions.
The 10x application-loop fixture advances 10 simulated seconds over 60 frames
at 60 Hz; both environment and aircraft clocks agree. On deck, the held aircraft
travels more than 120 m in world coordinates without shifting in ship coordinates.
