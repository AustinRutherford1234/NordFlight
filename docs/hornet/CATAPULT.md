# Carrier rail alignment and launch stability

The launch now follows the port bow rail in Grant's imported Nimitz mesh. This is asset alignment, not a surveyed real-carrier geometry or a validated operational catapult model. The source asset and license remain unchanged.

## Geometry evidence

`scripts/nimitz/inspect_catapult.py` imports the GLB into Blender without editing it and exports deck-strip measurements and an orthographic image. The rail borders in Object_6 give source centre X 226.9602203, start Y -79.0487442, end Y -199.3674011, height Z 21.2127075. Applying the same centering, rotation and 333/303.842285 scaling as the app gives ship X -5.2965719 m, start Z -30.3627807 m, end Z -162.2276156 m and height 23.2483494 m. The modeled stroke is 131.8648349 m. Mesh precision does not imply real-hardware accuracy.

`src/carrier/catapult.rs` shares those coordinates among aircraft placement, joint forces, progress, shuttle drawing and release. The duplicate hardcoded track overlay was removed. Previously the simulation used X=10 m and a 105 m stroke, unrelated to this visible rail.

## Dynamics

The previous lateral spring measured CG displacement and velocity but applied its force at the nose attachment. Rotation therefore moved the attachment without entering the damping measurement. The replacement constrains the attachment using its ship-relative position and velocity, including angular velocity crossed with its CG-relative arm. Damping uses effective point mass from the full inertia tensor. Nose steering targets neutral while attached or launching. Normal six-degree-of-freedom integration remains active.

Engineering estimates: lateral stiffness 350 kN/m, holdback stiffness 2 MN/m, damping ratio 1.1, lateral cap 250 kN, holdback cap 500 kN. Launch energy targets 78 m/s relative to the ship and subtracts actual forward engine force. The force schedule uses a 5 m entrance ramp and 8 m taper, normalized for their area, capped at 800 kN. Downforce remains 38% of launch pull. These are simulation tuning parameters, not manufacturer specifications. Engine shutdown, throttle below 0.5, elapsed launch time above 8 s or attachment deviation above 1.5 m abort the pull; release occurs at the rail endpoint.

## Automated evidence

On this Mac, all 607 tests passed (16.31 s). The 14 carrier tests include moving-deck transport, arrest, bolter, launch, pilot rotation and five new rail/joint tests. Logs and CSV traces are under the ignored `tests/carrier-catapult/` directory.

| Case | Peak attachment lateral error | Peak CG lateral error | Peak yaw | Release speed | Stroke time |
| --- | ---: | ---: | ---: | ---: | ---: |
| Light | 0.048 mm | 0.360 mm | 0.0056° | 76.68 m/s | 3.642 s |
| Heavy | 0.095 mm | 0.486 mm | 0.0074° | 76.92 m/s | 3.704 s |
| Three tanks | 0.120 mm | 0.542 mm | 0.0094° | 76.89 m/s | 3.750 s |
| Crosswind, tanks | 3.49 mm | 28.88 mm | 0.505° | 76.87 m/s | 3.750 s |
| Initial 3° yaw and 1 m/s lateral velocity | 0.189 m | 0.220 m | 3.56° | 76.80 m/s | 3.683 s |

The same disturbed start before this change peaked at 0.777 m CG excursion and 8.28° yaw. The replacement reduces CG excursion by about 72%; small damped reversals still occur. Nominal 60/120/240 Hz runs agree within the 2% release-speed/time requirement (76.89265 m/s and 3.75 s at each rate).

Reproduce with `cargo test --locked carrier:: -- --nocapture` and `cargo test --locked`. The tests also verify that the force evaluator does not overwrite motion state.

## Native app inspection and delivery

`scripts/nimitz/capture_catapult.py` runs the actual app at start, 25%, 50%, 75% and release, plus a close-up of the nose gear, both with and without tanks. All 12 captures completed and were visually inspected: the nose gear follows the visible port rail at each sampled position, with release at its forward end. These deterministic native renders inspect alignment throughout the stroke; they are not a manual pilot review or continuous-video flight test.

The optimized development app was rebuilt with `./build-mac.sh dev` and its ad-hoc signature verified with `codesign --verify --deep --strict`. Binary SHA-256: `97fabcdaae09845e2a4ead092e44e31398a8fba01d15f2fab7c8fa088dbbea2f`.

User flight review remains pending. Controls remain Shift for full power, Enter for launch and Alt+C for stopped-aircraft repositioning.
