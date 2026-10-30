# Carrier operations — first playable version

From the flight setup menu choose **CARRIER APPROACH** or **CARRIER CATAPULT**. Both use the F/A-18F. The Nimitz model by Larcenie Grant / k26_k29 is licensed CC BY 4.0; see `assets/ships/nimitz/README.md`.

The carrier runs a constant straight course at 25 knots in a sea corridor around x=20 km, starting at z=-16 km in the existing local terrain. The visual deck and the convex contact outline were measured from the imported asset. The 333 m scale, angled landing lane and wire stations remain simulation estimates. A collision skin exported from the imported mesh now covers the hull, deck, island and equipment, using swept contacts in the moving ship frame; see [carrier collision validation](hornet/CARRIER_COLLISION_2026-10-29.md). The active catapult follows the measured port bow rail in the imported model; this is not a surveyed real-carrier layout.

## Controls

- **H**: lower/raise hook. The approach starts with it selected down.
- **G**: landing gear. Both starts use gear down.
- **Alt+G**: toggle the optional approach gates and lineup/glidepath readout.
- **Shift**: full throttle; **Z/X** adjust power; **Ctrl** cuts power.
- **W/S**: pitch. S commands nose-up and W nose-down. Apply a gentle nose-up input after catapult release, then relax it. Launch rotation remains pilot-commanded with the Hornet FBW.
- **Enter**: fire the catapult when attached, engine running, fuel available, gear down and throttle above 85%.
- **Alt+Shift+C**: while stopped on the deck, move to the catapult and attach. This is a convenience reset for this first version, not a taxi/tow animation.
- **R**: restart the selected carrier start.
- **Shift+H**: help.

Approach guidance follows an estimated 3.5-degree angled lane and displays deck-relative speed. It does not fly the airplane. A hook-down crossing of a wire inside the forgiving catch envelope triggers arrest; hook-up landings roll through. Catapult and arresting-wire loads act through the normal aircraft physics solver at their attachment points. Wires stretch visually toward the aircraft and the shuttle follows the catapult stroke. Normal flight dynamics remain active throughout launch and arrest. Hard touchdowns, island impacts, missed deck edges and water impacts remain hazardous.

## Scope and validation

No pitch/roll sea motion, taxi director, launch-bar mesh animation, detailed hydraulic catapult, deck crew, weapons servicing, ship combat or damage model is included. Arrest, catapult and tyre/caster parameters are engineering estimates registered in `docs/hornet/PARAMETERS.md`. Only this mode spawns the carrier visual; other flight scenarios retain their own behavior.

Tests cover moving-frame transforms, deck boundaries, parked transport, catapult release, hook-dependent arrest, integrated touchdown/bolter behavior and scenario reset. Existing physics and collision regression tests also pass. `NORD_CARRIER_CAPTURE=deck-close` or `approach`, together with `--visual-check dry`, provides reproducible app-render captures without advancing flight. Additional presets `launch-release`, `arrested` and `relaunch` simulate the physical sequence first, then render the resulting state. Launch inspection presets include an explicit pilot pitch input after release. These are inspection presets, not normal controls.

The [Hornet FBW](hornet/FBW.md) provides basic stabilization and trim with manual flaps. It does not fly the approach or manage throttle. T selects normal/direct control; failed hardware can remove available control authority in either selection.

The [catapult alignment and stability update](hornet/CATAPULT.md) documents the measured rail, attachment-point damping, comparison traces and validation. The former duplicate track overlay has been removed; the model rail and movable shuttle share the physics geometry.
