# Aft arresting-wire layout and stronger absorber

All four wire centers move 20 m toward the stern, from deck-frame Z positions
112/100/88/76 m to 132/120/108/96 m. The bow is negative Z. Spacing remains 12 m,
and each wire remains 30 m wide across the existing angled landing lane. A shared
`carrier::WIRES` register drives rendering and hook-crossing detection. Tests
check both endpoints against the measured deck outline and catch each wire at its
new location. Cable payout continues to use the captured wire's stored anchor.

The approach height reference moves aft from Z=75 to Z=95 m, keeping the same
lineup and glide slope. Landing regression fixtures and diagnostic captures begin
relative to the new first wire rather than retaining an inconsistent old touchdown
position. The painted angled-lane reference itself is unchanged.

The estimated absorber has an 850 kN force cap (previously 700 kN), a 130 m payout
limit (previously 120 m) and 110 MJ energy limit (previously 90 MJ). Above 40 m/s
payout speed, the nominal resistance is 210 kN + 3 kN/m payout + 2.3 kN per m/s
payout speed, versus the old 180 kN + 2.5 kN/m + 2.0 kN per m/s. The additional
strength blends out between 40 and 10 m/s. Below 5 m/s, a proportional fade makes
the terminal hold gentler. This avoids extending the stronger high-speed pull
into a violent final stop. Dissipated energy is still accumulated, and exceeding
either stroke or energy limits releases the wire. Raising the hook still releases
immediately. No trajectory, aircraft attitude or stopping velocity is prescribed.

These are game calibration changes, not claimed specifications of actual Navy
arresting gear. Tests cover deck geometry, per-wire swept hook crossings, increased
pull, energy dissipation, hook release, overload release, banked/off-center arrest
stability, timestep consistency and the unchanged catapult behavior.

Final-source regression run: all 780 tests passed (76.63 seconds). Banked and
6 m off-center landings remained below the existing 5-degree whole-arrest peak
roll limit, finished below 1 degree with low roll rate, and retained contact at
all three gear legs. Settled roll is measured after eight seconds; the old
"first below 3 m/s" sample conflated a quicker deceleration with suspension
settling and rejected a brief 2-3 degree touchdown-bank transient. The updated
check keeps the whole-event peak bound, final attitude/rate bounds, dissipative
power check and timestep convergence. Recorded settled roll was about 0.03-0.05
degrees in the exercised cases. Tests also preserve finite-stroke cable release
and immediate hook-up release rather than making the stronger cable permanent.

The approved native layout/arrest review completed in 36.72 seconds of its
one-minute budget and restored the previous session. Both `wire-layout` and
`arrest` snapshots were inspected. The latter follows six seconds of ordinary
physics, shows WIRE 1 CAUGHT, approximately zero ship-relative speed, and an
upright supported aircraft. Displayed absolute airspeed near 13 m/s includes the
carrier's 25-knot motion. Local evidence: `tests/carrier-wire-review/`. These are
scripted snapshots, separate from manual flight review; no soak was run.
The existing Mac app was rebuilt and passed deep/strict ad-hoc signature
verification. Relaunch to load the new executable.
