# High-speed FBW, neutral trim and missile cleanup

## Reproduction and physical correction

The reported condition was clean flight near Mach 1.2 with neutral-control trim drift. Extended deterministic tests reproduced severe oscillation that the older 250 m/s response matrix missed. At 3,000 m and 400 m/s, the old model reached roughly ±20 g and angular rates above 7 rad/s during a small-input/release trial.

The tail's reduced-order torsion model used an estimated stiffness of 110,000 N m/rad. The combination of tail lift slope, area and elastic-incidence feedback implied static positive-feedback divergence at approximately 72 kPa, inside the model's nominal 90 kPa pressure envelope. Raising the engineering estimate to 300,000 N m/rad moves that approximate static boundary to 195 kPa. This is an internal model-consistency correction, not measured Super Hornet stiffness or validated flutter prediction. Damage, actuator limits and physical force integration remain enabled.

## Controller and trim

- Normal-load feedback and its integrator scale down with airspeed using clamp(180 / V, .35, 1). The normal-load target is unchanged.
- Inner angular-rate gains have a smooth dynamic-pressure schedule, from unity below 15 kPa to .5 at 140 kPa. No attitude/rate snapping or direct force compensation was added.
- Trim records achieved stabilator bias rather than chasing instantaneous allocator output. All-axis neutral input, low rates, small load error and small pitch allocation residual must persist for .75 s. The tail average is filtered over 1.5 s and transfers at .003 rad/s maximum.
- Neutral control remains load/rate stabilization, without altitude hold, automatic wings-level recovery or approach guidance. A banked aircraft can descend with controls released.

These gains, timing thresholds and stiffness are documented simulation estimates. They require user flight review rather than being represented as production F/A-18F software or flight-test calibration.

## Automated evidence

Final full regression: **783 passed, zero failed, 60.81 s** on this Mac. `cargo test --locked` includes the existing aircraft, carrier/deck, combat, reset and HUD regressions, plus:

1. Sixteen clean high-speed trials at 3,000 / 10,000 m, 300–500 m/s: neutral and pitch/rudder/roll pulse/release. Approximate Mach 1.2 start points are 394 m/s at 3,000 m and 359 m/s at 10,000 m. Each trial runs 20 simulation seconds with physical damage enabled.
2. Full-throttle acceleration with neutral controls and with a two-second pitch .4 / roll .3 pulse. Low-altitude observation ends at 40 s, before an intentionally uncorrected bank can reach terrain; high-altitude observation runs 60 s. This is not an autopilot test.
3. Mach 1.2 maneuver/release runs partitioned at 60 / 120 / 240 Hz, requiring final positions within .1 m and angular rates within .001 rad/s.
4. SAM motor energy/turning, spent/missed ballistic transition, survival at 299 s, expiry after 300 s, supported-target intercepts and frame-partition checks. Cruise-missile long-range regressions remain enabled.

Sampled telemetry from the matched 20 s, 3,000 m / 400 m/s small-input trial:

| Quantity | Before | After |
|---|---:|---:|
| Peak angular-rate magnitude, rad/s | 7.84 | .393 |
| Load range, g | -19.97 to 20.46 | .085 to 2.066 |
| Angular-rate magnitude at end, rad/s | 6.07 | .0174 |

At 10,000 m / 359 m/s, the neutral trial ends near Mach 1.199 and .9999 g, with angular-rate magnitude about .00014 rad/s. Neutral steady-trim cases at 300, 350 and 450 m/s require less than .001 rad trim change over 20 s. These are numerical stability and trim checks, not evidence of altitude hold or real-aircraft envelope accuracy.

The no-power-trim cases use explicitly labeled approximate initial trim and full throttle. Their initial load transient and subsequent deceleration are not claimed as sustained level-flight solutions. Long acceleration trials, sampled JSON, test logs and native screenshots remain local under `tests/hornet-fbw/` and `tests/fbw-highspeed-review/`.

## Delivery and review

The native review presets `fbw-highspeed-cockpit` and `fbw-highspeed-release-cockpit` simulate 30 s at a trimmed Mach 1.2 start, assert intact structure and settled rates, then freeze the resulting state for the normal application renderer. The latter includes the same short all-axis pulse. These scripted rendered states are separate from continuous hands-on piloting and the user's final flight review.


Delivery completed with `./build-mac.sh dev`; `codesign --verify --deep --strict` passed. Executable SHA-256: `1cd84958fdc67f29c2da6713d726be8a65f9fb4c9d1ea26678385e237abd8703`.

The approved native review completed in **38.56 s**, captured both requested views, and ran its session-restoration handler. Both images were inspected. At 30 s the neutral trial reported Mach 1.1990, .99995 g, intact structure and rate magnitude .000142 rad/s. The maneuver/release trial reported Mach 1.2820, 1.0010 g, intact structure and .0246 rad/s rate magnitude in the banked descending turn. Neither crashed. The latter is a physically banked, descending flight state, not level-flight trim. These are scripted physics-and-render snapshots, not continuous manual flight validation. Relaunch the existing app to load the rebuilt executable; an already-running restored process retains its previous executable.
