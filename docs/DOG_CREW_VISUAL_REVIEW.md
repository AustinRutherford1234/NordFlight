# Dog crew visual review — 26 October 2026

Native game captures from the upgraded F-22 dog rig. Otto / 08 Dachshund remains the default. Each state was captured at two different times; these are renderer screenshots, not concept art.

The screenshot passes drove corrections to eye-socket contrast, jaw thickness, iris/eyelid occlusion, mouth-curve endpoints, tongue/teeth placement, helmet stripe continuity, and shadow bias. The outside view checks the seated body against the existing cockpit and canopy.

## Model comparison

[Original portrait](../tests/crew-v2/before-portrait.png) · [Updated portrait](../tests/crew-v2/01-content-1.png) · [Through the canopy](../tests/crew-v2/exterior-canopy.png) · [Combat HUD](../tests/crew-v2/combat-hud.png)

## Twenty animation states

| State | Earlier frame | Later frame |
| --- | --- | --- |
| 01 Content | [View](../tests/crew-v2/01-content-1.png) | [View](../tests/crew-v2/01-content-2.png) |
| 02 Focused | [View](../tests/crew-v2/02-focused-1.png) | [View](../tests/crew-v2/02-focused-2.png) |
| 03 Curious | [View](../tests/crew-v2/03-curious-1.png) | [View](../tests/crew-v2/03-curious-2.png) |
| 04 Excited | [View](../tests/crew-v2/04-excited-1.png) | [View](../tests/crew-v2/04-excited-2.png) |
| 05 Proud | [View](../tests/crew-v2/05-proud-1.png) | [View](../tests/crew-v2/05-proud-2.png) |
| 06 Nervous | [View](../tests/crew-v2/06-nervous-1.png) | [View](../tests/crew-v2/06-nervous-2.png) |
| 07 Afraid | [View](../tests/crew-v2/07-afraid-1.png) | [View](../tests/crew-v2/07-afraid-2.png) |
| 08 Startled | [View](../tests/crew-v2/08-startled-1.png) | [View](../tests/crew-v2/08-startled-2.png) |
| 09 Bracing | [View](../tests/crew-v2/09-bracing-1.png) | [View](../tests/crew-v2/09-bracing-2.png) |
| 10 Positive G | [View](../tests/crew-v2/10-positive-g-1.png) | [View](../tests/crew-v2/10-positive-g-2.png) |
| 11 Negative G | [View](../tests/crew-v2/11-negative-g-1.png) | [View](../tests/crew-v2/11-negative-g-2.png) |
| 12 Side G | [View](../tests/crew-v2/12-side-g-1.png) | [View](../tests/crew-v2/12-side-g-2.png) |
| 13 Missile alert | [View](../tests/crew-v2/13-missile-alert-1.png) | [View](../tests/crew-v2/13-missile-alert-2.png) |
| 14 Stall worry | [View](../tests/crew-v2/14-stall-worry-1.png) | [View](../tests/crew-v2/14-stall-worry-2.png) |
| 15 Engine concern | [View](../tests/crew-v2/15-engine-concern-1.png) | [View](../tests/crew-v2/15-engine-concern-2.png) |
| 16 Hurt | [View](../tests/crew-v2/16-hurt-1.png) | [View](../tests/crew-v2/16-hurt-2.png) |
| 17 Dazed | [View](../tests/crew-v2/17-dazed-1.png) | [View](../tests/crew-v2/17-dazed-2.png) |
| 18 Relieved | [View](../tests/crew-v2/18-relieved-1.png) | [View](../tests/crew-v2/18-relieved-2.png) |
| 19 Celebrating | [View](../tests/crew-v2/19-celebrating-1.png) | [View](../tests/crew-v2/19-celebrating-2.png) |
| 20 Sleepy | [View](../tests/crew-v2/20-sleepy-1.png) | [View](../tests/crew-v2/20-sleepy-2.png) |

## Validation

The full Rust suite passed: **462 tests**. The final crew-specific rerun passed **15 tests**, covering all 30 breed/equipment mesh combinations, finite geometry, distinct moving clips, transition continuity, live flight events, signed G/explosion layering, paused blinking, camera ownership, saved settings, breed/equipment selection, and combat HUD clearance.

The Mac app was rebuilt and its ad-hoc signature verified. Performance measurements and native screenshots were taken with the real Metal renderer on Apple M1 at 1440 × 900, with the crew portrait enabled. See the raw [live performance report](../tests/crew-v2/performance-vsync.json). The portrait retains its environment-filter opt-out; the live sample reported zero multi-camera cache bypasses.

The clean live sample used ordinary VSync, 12 seconds of warmup and a 12.02-second sample: **59.42 FPS mean, 16.66 ms median, 17.74 ms p95**. Four frames exceeded 25 ms and one exceeded 50 ms. This is a short single-aircraft check, not a claim about every scene.

A later [timing sample](../tests/crew-v2/performance-vsync-final.json) averaged 29.85 FPS while a separate `Nord Flight.app` process was also running. Process inspection additionally showed a busy system ANE compiler. That sample is retained as contended evidence, rather than used for a model-only comparison. The user's running game was left open. Both samples retained zero environment-filter multi-camera bypasses.

[Animation editor screenshot](../tests/crew-v2/editor.png) shows all twenty preview buttons and their description area. The combat screenshot includes a real incoming IR missile; the defense panel and inbound warning remain clear of the portrait.
