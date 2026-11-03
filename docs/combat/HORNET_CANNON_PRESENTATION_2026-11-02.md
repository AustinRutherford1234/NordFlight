# Hornet cannon, GUN HUD and servicing

## Delivered behavior

- Hornet ammunition is 2,060 rounds, five times the previous 412-round gameplay load. This is a gameplay allocation, not a claim about the real magazine. Rate of fire, projectile speed and hit physics are unchanged.
- F7 cycles NAV → A/A → GUN → A/G; the cockpit HUD MODE control uses the same cycle. U fires the cannon. GUN displays ammunition, availability, a ballistic funnel, a 30 m reference and available selected-aircraft lead information. Finite HUD projections include eye/muzzle parallax. Selecting GUN does not acquire or lock a target.
- Actual ammunition expenditure drives a pulsing three-dimensional muzzle flash, warm local illumination, layered tracers and short smoke wisps. Effects freeze with the simulation, clear on reset and do not produce reload flashes.
- Backslash then 1 instantly refuels/rearms a stopped aircraft on a friendly paved runway or usable carrier deck. Idle throttle (at most 5%), landed state and surface-relative speed at most 1 m/s are required. Launch/arrest attachments must be released. Damaged/unusable carrier decks reject service.
- Service restores cannon ammunition, missiles, countermeasures and the selected tank configuration without resetting battle time, enemies or fleet ammunition.

## Resource bounds

Three persistent visual meshes, one shadowless light, at most 24 smoke puffs and 128 player tracer representations are used. Buffers are reused; empty effects skip mesh updates. Existing smoke textures are reused. Distant smoke/tracers are culled. No frame-rate improvement or measured performance neutrality is claimed; this task did not include a performance comparison.

## Validation

The automated suite passed 818 tests with no failures and one intentionally ignored timing test. Coverage includes GUN mode cycling/rendering, 2,060-round initial capacity, runway radio input consumption, service preserving battle state, carrier replenishment, damaged-deck restrictions, and flash/mesh guards. The final saved-source suite was rerun before delivery.

Native packaged-app external and cockpit firing captures were inspected. Both use actual cannon fire: six rounds expended, leaving 2,054. An intentionally offset cruise missile remains alive, so these images validate presentation rather than interception. The separate earlier cannon-versus-cruise regression remains in the suite. The native review used 37.75 seconds of the approved two-minute window and restored suspended flights.

Local evidence (not committed): `tests/cannon-firing-visuals/defense-cannon-firing-external.png`, `tests/cannon-firing-visuals/defense-cannon-firing-cockpit.png`, and the review JSON/logs in that directory. Reproduce with `scripts/fa18f/capture_cannon_fire.py` after obtaining any required live-flight diagnostic approval.

`./build-mac.sh dev` rebuilt the existing Mac application. `codesign --verify --deep --strict 'Nord Flight.app'` succeeded. Executable SHA-256: `ed61bfd297eb1b1195ef2c89f3278fe9a7dec08d641be229cfef9fda8c6f7257`.

These are automated and scripted visual checks; the user's interactive flight review remains separate.

## Follow-up: HUD-only sight and direct shortcuts

Removed the external-view gun funnel, lead director, aiming cross and 30 m marker. Ballistic calculations remain available to the cockpit HUD. In the Hornet, 8 selects GUN and 9 selects NAV. Option+8 retains the Hard Combat scenario shortcut and Option+9 opens audio settings; other aircraft retain their existing 8/9 controls. F7 mode cycling remains available. This follow-up uses automated HUD/control checks and a rebuilt signed app; no additional live-flight diagnostic was run. The executable hash above identifies the earlier presentation build, not this follow-up build.
