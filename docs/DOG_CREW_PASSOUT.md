# F-22 dog crew: G-LOC, recovery and cockpit vision

## Unconscious dream

With Cockpit G Effects enabled, the player's fitted F-22 dog sees a one-shot
dream only while fully unconscious in cockpit view. Framed family portraits
drift away and fade as a paw reaches toward them. The paw's independently moving
finger sections stay visible after the portraits vanish and fade during waking.
The waking fade uses the existing recovery timing. The portraits
follow the selected breed and receive a subtle coat-palette tint; the paw also
uses the selected coat palette. The art is illustrative rather than an exact
render of the selected rig or markings. Flight physics, control-loss rules,
and unconscious timing are unchanged.

The Dachshund's drifting memories show the family at the beach, the family
watching a Texas Longhorns football game from the stands, and the couple's
wedding without the puppies. Other breeds retain their original family image.

Synthesized music is raised in the mix to twice its previous dream level while
all other sound remains audible at five percent of its normal level. The music continues through the unconscious hold and follows the existing
effects volume, master volume, mute, and pause settings. The full-screen dream
covers the flight HUD, but does not capture input. Switching camera or turning
off Cockpit G Effects restores the normal view. This feature currently applies
to the selected player crew, not AI aircraft.

For a posed native inspection, set `NORD_AIRCRAFT=f22`,
`NORD_CREW_CAPTURE=vision-faint-out` and `NORD_DREAM_TIME=1.4`, then run
`target/debug/gripen-flight --visual-check cockpit`. `NORD_CREW_BREED` accepts
the existing zero-based breed selection. These variables are diagnostic only.

## Gameplay behavior

Sustained signed vertical G builds a per-pilot exposure burden. Brief spikes produce strain without instant unconsciousness. Positive exposure starts above +4.5 G and negative exposure below −1.3 G; at constant +9 G, collapse begins after about 2.4 seconds, and at constant −3 G after about 3.3 seconds. These are gameplay tuning values measured in simulation seconds. Horizontal loads still drive inertial motion and affect whether recovery conditions are settled.

The state machine proceeds through **alert → struggling → fading → collapsing → unconscious → waking → recovering → alert**. A collapsing pilot loses consciousness over 1.15 seconds. Unconsciousness lasts at least four seconds and waking requires a settled load (−0.8 to +3 G, modest horizontal loads) for 0.8 seconds. Waking takes 3.2 seconds, followed by four seconds of recovery. Renewed severe exposure can cause a relapse. Pausing freezes exposure and stage progression while idle breathing continues. Mission reset restores consciousness. Catastrophic crew loss retains its separate restrained presentation.

## Animation detail

- Progressive eyelid droop and flutter, an unfocused gaze, small corrective head nods and a failing brace precede collapse.
- The eyes close slightly out of sync, the final head correction fades, and the dog slumps against the harness with a small sideways head roll.
- Shoulder motion, independent wrist joints and finger joints let the hands lose tension asymmetrically and relax off the controls.
- During unconsciousness, closed eyes stay closed even during explosions, chatter stops, the jaw remains slack, breathing continues, and ears move passively. Negative-G ear lift is retained under ongoing negative load.
- Waking includes uneven eye opening, a second blink, searching gaze, a small swallow/jaw motion, and returning hand tension. Recovery adds a tired long blink, steadier posture and a deeper exhale.
- The portrait eases back during the sequence to reveal the hands and harness. The same articulated rig drives the outside-visible pilot. Cached meshes are reused throughout.

Seven previews are available in **DOG CREW → G-LOC / PASS OUT & RECOVERY**, including a full live-timed sequence. The preview and Done button remain fixed while controls scroll.

## Start-menu option

**G-LOC CONTROL LOSS** appears on atmospheric launch setup and in **Settings → PILOT G-LOC / F-22 CREW**. It defaults **OFF** and is saved with the existing crew preferences.

**COCKPIT G EFFECTS** is a separate saved option in the same two locations. It defaults **ON** to preserve existing behavior. Turn it **OFF** to disable cockpit grey-out, red-out, tunnel vision and blackout; switching it off immediately restores the scene even if the dog is unconscious. Older preference files keep the existing ON behavior.

Animation and crew status operate with either choice of both options. With control loss ON, manual pitch/yaw/roll authority fades with consciousness and returns during recovery; throttle and autopilot mode are held during unconsciousness. Autopilot can continue flying, and the user retains camera, pause and reset access. This first fitting applies to the player F-22 crew; AI crew animations retain individual exposure states.

## Cockpit vision

Positive-G exposure progressively desaturates the actual cockpit scene and adds grey contrast loss and tunnel vision. Negative-G exposure produces a red wash and tunnel vision. Collapse smoothly reaches an opaque, fully black cockpit image. Waking opens vision gradually and recovery restores normal color and visibility. The crew portrait remains visible throughout. Switching away from cockpit view, entering a menu or resetting restores the normal camera grading and hides the overlay.

The effect uses the existing flight camera’s color grading and four inexpensive UI layers, including one generated vignette texture. It does not create another render view or change flight forces. The existing portrait environment-filter opt-out remains in place.

## Native review

| Stage | Pilot | Cockpit |
| --- | --- | --- |
| Grey-out | [Portrait](../tests/crew-passout/01-faint-grey-1.png) | [View](../tests/crew-passout/01-faint-grey-1-cockpit.png) |
| Red-out | [Portrait](../tests/crew-passout/02-faint-red-1.png) | [View](../tests/crew-passout/02-faint-red-1-cockpit.png) |
| Collapsing | [Portrait](../tests/crew-passout/03-faint-collapse-1.png) | [View](../tests/crew-passout/03-faint-collapse-1-cockpit.png) |
| Unconscious | [Portrait](../tests/crew-passout/04-faint-out-1.png) | [View](../tests/crew-passout/04-faint-out-1-cockpit.png) |
| Waking | [Portrait](../tests/crew-passout/05-faint-wake-2.png) | [View](../tests/crew-passout/05-faint-wake-2-cockpit.png) |
| Recovery | [Portrait](../tests/crew-passout/06-faint-recover-1.png) | [View](../tests/crew-passout/06-faint-recover-1-cockpit.png) |

[Full native animation preview](../tests/crew-passout/passout-sequence.gif) (160 frames; GIF scaled to 480 px, raw captures retained at 640 × 640) · [Control loss OFF](../tests/crew-passout/menu-option-off.png) · [Control loss ON](../tests/crew-passout/menu-option-on.png)

## Validation

The cockpit G-effects option follow-up passes **494 tests**. Extended checks
verify immediate restoration during unconsciousness, re-enabling the blackout,
independent control-loss behavior, and backward-compatible preference defaults.
Native Settings captures use isolated preferences: [OFF](../tests/cockpit-g-option/settings-off.png)
and [ON](../tests/cockpit-g-option/settings-on.png). The updated Mac bundle is
ad-hoc signed and passes strict signature verification.

The full suite passed **477 tests** and the focused crew rerun passed **29 tests**. New checks cover sustained versus brief signed-G exposure, recovery conditions and timings, pause/reset behavior, finite poses and closed unconscious eyes, red/grey selection, opaque cockpit blackout and restoration on camera change, saved option defaults, and opt-in control authority/throttle holding. All 30 breed/equipment mesh combinations remain finite with the new hand joints.

Native captures cover both colored precursors, collapse, full blackout, wake/recovery, menu OFF/ON states, and a full motion sequence. The Mac app was rebuilt and its ad-hoc signature verified. Runtime checks were on macOS/Metal.

## Diagnostic capture commands

`NORD_AIRCRAFT=f22 NORD_CREW_CAPTURE=faint-vision-gallery target/debug/gripen-flight --visual-check cockpit` captures paired portraits/cockpit views of six stages. `NORD_AIRCRAFT=f22 NORD_CREW_CAPTURE=faint-sequence-gallery target/debug/gripen-flight --visual-check dry` records the full portrait sequence at eight samples per second under `tests/crew-passout/sequence/`. Individual preview slugs are `faint-grey`, `faint-red`, `faint-collapse`, `faint-out`, `faint-wake`, `faint-recover`, and `faint-sequence`; prefixing with `vision-` presents the effect in cockpit view for a single capture. Diagnostic selections do not overwrite the player’s saved crew settings.
