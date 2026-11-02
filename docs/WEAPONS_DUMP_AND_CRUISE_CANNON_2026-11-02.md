# Guarded weapons dump and cannon interception of cruise missiles

## Pilot controls

Hold either Option/Alt key and press W three distinct times within 1.5 real-time seconds. This uses the same bounded gesture window as the existing ejection shortcut, with separate counters. Holding W does not accumulate presses. Scenario generation changes, menu/hangar entry, editing warp settings, a non-aircraft vehicle, crash/ejection and an unmodified W press invalidate the partial gesture.

The command clears carried missile inventory, mounted weapon-presence flags and queued bay launches. Existing aircraft mass, drag and mounted-store presentation consume those flags. It preserves cannon rounds, fuel, external tanks and already airborne missiles. Position, velocity and angular rates are unchanged at removal. Status text confirms the dump, or reports that no carried weapons remain. Option+W does not also command pitch or map panning. Plain W remains available for both normal functions. The binding appears in the pause Controls page and in-flight help.

The handler runs after combat scenario initialization, including noncombat flights, and resets its gesture while the main menu is open. It does not depend on a target lock or master-arm state.

## Cannon collision

The player cannon previously checked enemy aircraft but not the naval mission's cruise-missile collection. It now sweeps player projectiles against moving live anti-ship missile bodies. The collision surrogate uses an ellipsoid with the existing representative ordnance mesh's 6.2 m length and 0.52 m diameter, with 0.01 m bullet-radius padding radially. A conservative relative-segment AABB rejection precedes the ellipsoid check. Both missile translation and the full bullet segment participate, rather than checking only an endpoint or using an aim proximity cue.

The naval weapon solver runs after cannon integration, so the collision query predicts missile translation over that same interval. This is a bounded constant-velocity approximation during the physics substep. Missile dimensions and one-hit destruction of the fragile missile body are gameplay estimates, not validated warhead/airframe vulnerability data.

A computed hit consumes the bullet, retires the missile and its live render state/track, increments interception and cannon-strike counts, and uses existing impact/flash and blast-sound effects. It cannot award a second hit on an already destroyed missile. Hostile bullets do not use the new player-cannon interception path. The query uses the existing capped missile collection and does not allocate per projectile query.

## Validation

Automated tests exercise the actual control system and combat tick. Coverage includes distinct triple presses with both Option keys, holding/timeouts, reset and menu boundaries, plain-W cancellation, weapon/tank/gun separation, queued-launch cancellation, preserved flight kinematics, pitch input isolation, moving cruise-body hits at 60/120/240 Hz, near misses, hostile bullets, bullet consumption and prevention of duplicate interceptions.

No live user flight is interrupted by this task. Automated simulation evidence is separate from a native visual review or real-world ballistics validation.

## Recorded results

Final `cargo test --locked`: **804 passed, zero failures, one opt-in timing diagnostic ignored**, in 23.15 seconds. Both weapon-dump tests, the Option+W pitch-isolation regression and both cruise-cannon tests passed. The cruise-hit regression exercises 60, 120 and 240 Hz; near-miss and hostile-bullet checks leave the cruise missile alive.

The existing `Nord Flight.app` was rebuilt with `./build-mac.sh dev` and passed `codesign --verify --deep --strict`. Executable SHA-256: `6d293414e4f73e81618130e0c8197d34d8608a759ac0d766b34ec251d12e80e5`. The current flight was not stopped or restarted; relaunch the app to load the new executable.
