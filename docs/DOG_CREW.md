# Dog crew: first playable milestone

## Agreed design

All ten concept-sheet breeds are selectable. **08 Dachshund / Otto** is the default and the first detailed character. Use the stylized 3D concept-sheet look: an oversized canine head on a small humanoid body, a complete seated pilot, sized for the existing cockpit. Player appearance follows the player between vehicles. Individual named dogs have distinct reaction sensitivities.

The player can customize coat palettes, markings, suit color, and equipment. All three face-equipment options are available: open helmet/headset, headset alone, and helmet with an oxygen mask. Aircraft use flight suits; pressure suits belong to the subsequent spacecraft fitting.

Show one compact, movable and resizable crew portrait. It follows the viewed aircraft and hides during missile follow. The portrait faces the dog, with glances toward nearby threats/explosions. Its background uses the actual aircraft cockpit geometry. AI crews draw from all ten breeds.

Use separate signed vertical and lateral accelerometer loads. Positive-G strain begins at 2–3 G, is obvious around 5 G and reaches full intensity at 8–9 G. Follow loads immediately; negative G produces an uncomfortable grimace and rising ears. Keep head movement subtle and ears expressive. Blend explosion surprise with G strain rather than replacing it.

Nearby explosions matter more when seen or when damaging the aircraft. Include missile warning, stall, hard landing, damage, engine failure and successful landing reactions. Permit restrained crew death, comic nonverbal chatter, and blinking/idle motion while paused. Polish a single aircraft before extending the fitting to the rest of the fleet and spacecraft.

## Implemented first fitting

The initial cockpit fitting is the **F-22**, the home screen's default aircraft. The home screen and flight view have a **DOG CREW** button. The editor exposes all ten breeds, saved appearance options, chatter, and expression previews. The portrait top bar is its drag handle; its lower-right corner is its resize handle. Settings live in `.nord-flight/crew.json` alongside the existing local game preferences. `NORD_FLIGHT_CREW_SETTINGS` overrides that path for tests.

Geometry is authored in `src/crew/model.rs`: a full bent-leg seated body, gloves, boots, suit seams, pockets, zip, shoulder patches, webbing, harness adjusters, buckles, a continuous sculpted skull/cheek/muzzle surface, separate hinged lower jaw, teeth and tongue, curved eyes with irises/pupils/highlights, independent upper/lower eyelids and brows, whisker follicles, articulated thick ears, a fitted open helmet, earcup hardware, microphone and optional oxygen mask/hose. Static detail is merged by material and articulation group. The animated portrait and exterior pilot share the same cached model assets.

The live portrait renders at 640 × 640 with its own front-facing camera, key/fill lighting, a single self-shadow cascade, and three local accent lights. Its seat, tub, consoles and rear bulkhead come from the same F-22 interior builder used by the aircraft. The player's exterior pilot is excluded from the first-person camera to prevent looking through its head. F-22 AI pilots use the same outside-visible rig and watched-aircraft reaction state.

Reactions read flight state without changing physics or simulation RNG. Signed lateral G comes from the same specific-acceleration measurement as the existing vertical G meter. Catastrophic airframe destruction currently marks that aircraft's crew lost; this is a game rule, not an injury model. Starting/resetting a mission restores the crew. Chatter uses short synthetic nonverbal syllables through the game's normal audio mixer and mute/volume controls.

Other airframes and spacecraft do not yet have a fitted pilot or portrait. Their player appearance selection remains saved. Breed variants share the detailed rig and have distinct skull, muzzle, ears, jowls and coat palettes; further individual sculpting beyond the Dachshund is a later polish pass. Crew selection is currently one pilot per vehicle; multi-seat selection can be added when a supported multi-crew cabin is fitted.

## Repeatable inspection

Use the existing native capture workflow, e.g.:

```sh
NORD_AIRCRAFT=f22 NORD_CREW_CAPTURE=calm target/debug/gripen-flight --visual-check dry
```

`NORD_CREW_CAPTURE` accepts `calm`, `positive`, `negative`, `side`, `boom`, `lost`, `flight` and `exterior`. `NORD_CREW_BREED` selects a zero-based breed index (7 is the Dachshund); `NORD_CREW_EQUIPMENT` selects 0=open helmet, 1=headset, 2=oxygen mask. These overrides use disposable in-memory choices and do not save over the player's preferences. Portrait PNGs are saved under `tests/crew-*.png`; the existing native window capture also produces `tests/visual-dry.png`.

## Validation — 25 October 2026

- Eight crew tests passed, including live signed-load/explosion blending, crew-loss reset, follow-camera routing, blinking on wall time while paused, selection of all ten breeds and three equipment choices, saved-setting bounds, and finite mesh attributes/indices across all **30 breed/equipment combinations**.
- The existing 24 audio tests and 18 physics tests passed (50 tests across these three suites).
- Native macOS GPU captures were inspected for the calm, positive-G, negative-G and explosion expressions, the oxygen-mask option, the editor and the outside-visible F-22 pilot. Cap normals, eyelid occlusion and thin-ear rendering were refined from those captures.
- The Mac app was rebuilt and its ad-hoc signature verified. Windows runtime verification remains outstanding.
- These checks cover the first fitting and selected poses; they are not a long-duration performance or memory study.

## Performance correction

The first crew release caused the sky-light renderer to repeat environment filtering for the portrait camera. This has been corrected without reducing model detail or portrait quality. See [the measured regression fix](DOG_CREW_PERFORMANCE_FIX.md) for the cause, changes, benchmark reports and tests.

## Combat HUD clearance

With a crew portrait visible in third-person combat, weapon/target information and the defense/countermeasure panel dock together in a column beside the portrait. The threat-bearing compass and incoming-missile caption use that same reserved column, with the caption beside the compass. Weather stays in its own upper-right slot. The column chooses a clear side when the portrait is moved or enlarged. Hidden portraits restore the original combat-panel anchors; simplified HUD warnings retain their original position when it is clear.

Twelve crew/layout tests passed, including default, resized and moved portrait clearance, weather/weapon separation, actual panel anchoring and no repeated layout invalidation on unchanged frames. The final layout was inspected using the native `air-hud-threat` scene with an incoming IR missile in third-person view.

## Model and animation overhaul — 26 October 2026

Otto now uses a continuous organic face generated once from smooth implicit surfaces, with recessed eye sockets, shaped cheeks, a long Dachshund muzzle, a smaller wet canine nose, fitted spherical eyelids, a compact articulated jaw, teeth, tongue and thick tapered ears. The helmet has a continuous flush stripe and a shaped rubber seal. Flight gear includes stitched flat webbing, frame buckles, a zipper, pockets, sleeve paw patches, gloves and boots. The same model is visible in the portrait and through the canopy. The other nine breed variants retain their own proportions, ears, palettes and equipment choices on the shared upgraded rig.

Twenty named animation clips are available under **DOG CREW → EXPRESSION PREVIEW**. Selecting a clip also displays a short description of the feeling and motion. Live flight chooses clips from telemetry and events; clips crossfade over 0.22 seconds. Signed G, explosion surprise, threat glances, blinks and chatter layer onto the performance. Wall-time idle movement and blinking continue during pause. All motion updates joint transforms without rebuilding meshes or allocating per bone each frame.

| State | Performance / live context |
| --- | --- |
| Content | Relaxed breathing, gentle gaze and ears during quiet flight or ground idle. |
| Focused | Forward lean, narrowed eyes and instrument checks, including fast low flight. |
| Curious | Head tilt, one raised brow and a listening ear flick during idle moments. |
| Excited | Bright eyes, eager pant and ear bounce during takeoff acceleration. |
| Proud | Chin lift and relaxed shoulders after a successful landing. |
| Nervous | Eye darts, pinned ears and tense breathing with a badly damaged aircraft. |
| Afraid | Wide eyes, retreat into the seat and jaw tremble after a nearby explosion. |
| Startled | Fast recoil, open jaw and cautious recovery on a new explosion. |
| Bracing | Harness hunch, tense arms and squeezed eyes at extreme positive G. |
| Positive G | Seat compression, squint and clenched jaw with increasing positive load. |
| Negative G | Uncomfortable grimace, lifting head and floating ears. |
| Side G | Signed sideways lean and lagging ears. |
| Missile alert | Threat scanning, raised brows and quick breathing with an incoming missile. |
| Stall worry | Instrument glance, uneven squint and a tense mouth during a stall. |
| Engine concern | Side check and questioning brow on engine failure. |
| Hurt | Restrained asymmetric wince and shoulder flinch when damage is taken. |
| Dazed | Slow blinking and an unsteady head after a hard landing. |
| Relieved | Exhale, relaxed ears and slowly reopening eyes when a missile warning clears. |
| Celebrating | Happy pant, tongue, head bob and buoyant ears on a successful landing. |
| Sleepy | Droopy eyelids, slow nod and yawn during engine-off ground idle. |

Crew loss remains a separate restrained terminal pose, outside the twenty emotional clips.

For repeatable animation inspection, `NORD_CREW_CAPTURE=gallery` renders two native 640 × 640 frames per clip to `tests/crew-v2/`, then exits. Individual clip slugs (for example `curious`, `bracing`, `engine-concern`) also work with `NORD_CREW_CAPTURE`. The diagnostic gallery changes only in-memory preview settings. The original portrait is retained at `tests/crew-v2/before-portrait.png` for comparison.

Final validation and screenshot links are in [the native visual review](DOG_CREW_VISUAL_REVIEW.md). The full suite passed 462 tests; the final crew rerun passed all 15 tests. The Mac app was rebuilt and signed. The clean live F-22 timing sample averaged 59.42 FPS (17.74 ms p95); the review also records a later contended run with another game instance running.

## Three-axis acceleration reactions — 26 October 2026

The rig now layers continuous signed loads over the existing twenty emotional clips. Body-right load comes from +X and body-forward load from −Z, measured from the same specific acceleration as the vertical G meter. Gravity and rotating-frame terms are removed before this measurement. The added longitudinal value is read-only telemetry; flight forces, controls and integration timing are unchanged.

The occupant leans opposite the aircraft's acceleration: right acceleration pulls the pilot left; forward acceleration presses the pilot back; backward acceleration/braking pulls the pilot toward the harness. Chest, head, gaze and ears all respond. The body follows with a short 55 ms response; ears trail with a 140 ms response. Positive strain adds compression, squint, bracing and an extreme-load tremor. Negative load lifts the head/ears and increases the grimace and jaw tremble. Near-zero load produces gentle head/ear floating only while airborne and unloaded on all three axes. Combined loads remain additive during explosions, warnings and other emotional events.

**DOG CREW → ACCELERATION / CONTINUOUS IN LIVE FLIGHT** adds fifteen previews, bringing the editor to 35 emotional/acceleration choices (plus live and crew loss):

| Preview | Aircraft load |
| --- | --- |
| Zero G | 0 G on all axes, airborne |
| Positive: light / strong / extreme | +2.5 / +5 / +9 vertical G |
| Negative: light / strong / extreme | −0.5 / −1.5 / −3 vertical G |
| Left: light / strong | −0.5 / −2 lateral G |
| Right: light / strong | +0.5 / +2 lateral G |
| Forward: light / strong | +0.5 / +2 longitudinal G |
| Backward: light / strong | −0.5 / −2 longitudinal G |

These are sample points, not discrete animation jumps. Live intensity interpolates continuously through positive-load landmarks of 1.5, 3, 5 and 9 G and negative-load landmarks of 0, −0.5, −1.5 and −3 G. Horizontal reactions start with smaller loads and increase through the 2 G previews. Strong negative, sideways and fore/aft loads can also trigger the existing cooldown-limited nonverbal chatter. The compact portrait caption identifies the dominant load/direction. Takeoff excitement remains available while the inertial layer responds to acceleration. The preview image and Done button stay fixed while the controls scroll, and preview changes preserve scroll position.

Run `NORD_AIRCRAFT=f22 NORD_CREW_CAPTURE=load-gallery target/debug/gripen-flight --visual-check dry` to capture two native frames of each acceleration preview under `tests/crew-loads/`. Individual slugs such as `g-forward-strong`, `g-backward-strong`, `g-left-strong`, `g-positive-light` and `g-negative-extreme` work with the existing one-pose capture command. These overrides do not overwrite player settings.

See [the acceleration screenshot review](DOG_CREW_ACCELERATION.md) for the fifteen previews, direction checks and final test results.

## G-LOC and recovery

Detailed sustained-G pass out, waking/recovery, an optional start-menu control consequence, and positive-G grey-out / negative-G red-out / cockpit blackout are implemented. See [the behavior and native review](DOG_CREW_PASSOUT.md).
