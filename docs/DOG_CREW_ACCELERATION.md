# Dog acceleration reactions — 26 October 2026

The existing twenty emotional clips now have a continuous, three-axis acceleration layer. Fifteen additional previews expose different directions and intensities. Directions describe the aircraft’s acceleration; the pilot leans the opposite way.

| Preview | Motion | Native frame 1 | Native frame 2 |
| --- | --- | --- | --- |
| Zero G | Head and ears float only while airborne and unloaded. | [View](../tests/crew-loads/01-g-zero-1.png) | [View](../tests/crew-loads/01-g-zero-2.png) |
| +2.5 G | Subtle seat compression and focused eyes. | [View](../tests/crew-loads/02-g-positive-light-1.png) | [View](../tests/crew-loads/02-g-positive-light-2.png) |
| +5 G | Stronger compression, squint, breathing and grip. | [View](../tests/crew-loads/03-g-positive-strong-1.png) | [View](../tests/crew-loads/03-g-positive-strong-2.png) |
| +9 G | Bracing, deep squint and a small strain tremor. | [View](../tests/crew-loads/04-g-positive-extreme-1.png) | [View](../tests/crew-loads/04-g-positive-extreme-2.png) |
| −0.5 G | Uneasy brows and gently rising ears. | [View](../tests/crew-loads/05-g-negative-light-1.png) | [View](../tests/crew-loads/05-g-negative-light-2.png) |
| −1.5 G | Stronger grimace and floating ears. | [View](../tests/crew-loads/06-g-negative-strong-1.png) | [View](../tests/crew-loads/06-g-negative-strong-2.png) |
| −3 G | Pronounced lift, ear spread and jaw tremble. | [View](../tests/crew-loads/07-g-negative-extreme-1.png) | [View](../tests/crew-loads/07-g-negative-extreme-2.png) |
| Left 0.5 G | Gentle pilot lean to the aircraft’s right. | [View](../tests/crew-loads/08-g-left-light-1.png) | [View](../tests/crew-loads/08-g-left-light-2.png) |
| Left 2 G | Stronger rightward lean with lagging ears. | [View](../tests/crew-loads/09-g-left-strong-1.png) | [View](../tests/crew-loads/09-g-left-strong-2.png) |
| Right 0.5 G | Gentle pilot lean to the aircraft’s left. | [View](../tests/crew-loads/10-g-right-light-1.png) | [View](../tests/crew-loads/10-g-right-light-2.png) |
| Right 2 G | Stronger leftward lean with lagging ears. | [View](../tests/crew-loads/11-g-right-strong-1.png) | [View](../tests/crew-loads/11-g-right-strong-2.png) |
| Forward 0.5 G | Light press back into the seat. | [View](../tests/crew-loads/12-g-forward-light-1.png) | [View](../tests/crew-loads/12-g-forward-light-2.png) |
| Forward 2 G | Seat press, chin lift and backward-trailing ears. | [View](../tests/crew-loads/13-g-forward-strong-1.png) | [View](../tests/crew-loads/13-g-forward-strong-2.png) |
| Backward 0.5 G | Gentle lean toward the harness. | [View](../tests/crew-loads/14-g-backward-light-1.png) | [View](../tests/crew-loads/14-g-backward-light-2.png) |
| Backward 2 G | Forward pitch, harness brace and forward-swinging ears. | [View](../tests/crew-loads/15-g-backward-strong-1.png) | [View](../tests/crew-loads/15-g-backward-strong-2.png) |

Live intensity interpolates between the preview values. The body responds with a 55 ms time constant and the ears with a 140 ms time constant. G strain, horizontal inertia, gaze and bracing combine with explosion and warning expressions. The portrait eases back slightly when negative G spreads the ears, keeping them inside the frame.

The acceleration controls are below the emotion previews in **DOG CREW**. The preview and Done button stay fixed while controls scroll. Scroll position survives preview/wardrobe changes. [Native editor check](../tests/crew-loads/editor-negative-3g.png).

Validation: the complete suite passed **469 tests**, followed by **21 crew tests** after the final UI adjustment. Checks cover signed accelerometer axes under rotated attitudes, monotonic G intensity, mirrored lateral/longitudinal motion, body/ear response timing, load reversal, combined explosion/load expressions, airborne-only zero G, actual rig transforms in acceleration previews, all 30 breed/equipment meshes, and existing combat HUD clearance. Native screenshots cover all fifteen previews at two times each.

The added acceleration values are telemetry only; flight forces, integration and controls are unchanged. The rig reuses cached meshes and the existing portrait render target/camera, with no per-frame mesh rebuilding or extra render view.

## More dramatic floppy ears

Negative G now lifts hanging ears past horizontal, with a soft spring overshoot, asymmetric swaying/twisting, and a brief settling flop as the load releases. The additional lift grows with negative-G intensity; upright-ear breeds retain the ordinary response. The portrait's existing negative-G pullback keeps the expanded poses in frame. This animation updates joint transforms on the same cached meshes.

The focused crew suite passed 22 tests, including a new check for stronger lift, continuing sway, bounded rotations, and settling to neutral after release. Native checks of the −1.5 G and −3 G poses are saved in `tests/crew-floppy-ears/`.
