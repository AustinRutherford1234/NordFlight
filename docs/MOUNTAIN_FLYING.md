# Mountain Flying

In Flight Setup, select **Mountain Flying** beside Atmospheric Flight and
Spaceflight. The tab contains **Free Flight** and **Close Combat**. Its live
background uses the selected aircraft, gear retracted, over installed Norwegian
terrain. A slow cinematic pass runs independently of paused flight simulation.

Close Combat offers these friendly-versus-hostile force sizes:

| Option | Friendly force | Hostile force |
| --- | --- | --- |
| 1 × 1 | Player | 1 AI |
| 1 × 2 | Player | 2 AI |
| 1 × 4 | Player | 4 AI |
| 2 × 2 | Player + AI wingman | 2 AI |
| 2 × 4 | Player + AI wingman | 4 AI |

Each force size supports three entries and Training, Balanced, or Hard difficulty:

- **Valley Merge:** opposing formations approach, initially about 3.5 km apart.
- **Enemy on Your Six:** enemies start about 2.2 km behind the player.
- **Valley Pursuit:** enemies start about 1.8 km ahead, maintaining their initial
  course for ten seconds before engaging; terrain and defensive responses remain active.

The selected aircraft keeps its standard missile inventory and cannon capacity.
Existing aircraft configuration, including the F-22 external-stores option,
remains available. AI wingmen and enemies can use radar missiles, short-range IR
missiles, cannon, and countermeasures. Normal sensor visibility, terrain masking,
weapon guidance, collision, and damage rules apply.

Terrain avoidance previews the current flight path and an intended turn using
body-fixed, double-precision terrain queries. The forecast includes speed,
descent, roll recovery time, and water elevations. It checks 32 intervals over a
speed-dependent 14–22 second horizon. Threatening paths trigger climb/roll
recovery; steep obstacles also prompt checks of side corridors. Queries remain
CPU-resident and never wait for rendering or disk access.

Each AI caches its preview, normally refreshing at 8 Hz and at most 16 Hz for
rapid retasking, with immediate invalidation after a teleport or simulation-time
reset. Flight controls and the full aircraft physics continue at their usual
frequency. Close-combat pilots extend to regain energy before resuming tight
turns. Stall recovery unloads incidence before attempting a terrain climb.

The menu owns one fixed terrain patch (about 17,000 vertices) and reuses the
world's texture arrays and geographic masks. Its material instance has independent
opacity so opening the tab from an orbital flight still shows solid mountains.
Switching aircraft rebuilds the preview vehicle, not the terrain patch. Menu
scroll position is retained while changing scenario settings.

Scenario variants have stable IDs encoding force size, entry, and difficulty.
Restart retains the selected variant.
The existing scene-loading gate prepares destination rendering before flight
simulation resumes.

Validation commands:

```sh
cargo test --locked --bin gripen-flight
NORD_ENVIRONMENT=earth NORD_MOUNTAIN_SECONDS=120 cargo test --locked --bin gripen-flight real_pack_mountain_survival_study -- --ignored --nocapture --test-threads=1
./build-mac.sh release
python3 tests/mountain-combat/visuals.py
```

The real-pack survival study runs in an isolated test process because the active
environment is immutable. Weapons are disabled in that phase to distinguish
terrain survival from combat damage. A diagnostic pilot flies the player through
the real aircraft control interface; it is not a human flight review. Runtime
reports and native captures are saved under `tests/mountain-combat/`.

## Recorded validation

The final delivery passed **906 tests**, with two diagnostic studies intentionally
ignored by the default test command. All 45 Earth combinations (five force sizes,
three entries, three difficulties) completed separate two-minute weapons-disabled
terrain-survival studies without AI losses or player crashes. The lowest recorded
AI clearance was 194.1 m. Separate sci-fi Earth and packaged-release survival
checks also completed. An armed 2 × 4 study launched 11 missiles, retained a live
player, and recorded one AI combat loss.

Native captures verified the F-22 and Hornet previews, menu/HUD separation, and
mountain combat. The signed release application is `Nord Flight.app`.

The final native 2880 × 1800 menu sample on AC power, after 20 seconds of warm-up,
recorded 84.2 mean FPS, 25.84 ms p95, and 26.10 ms p99 across 20 seconds. This
sample showed uneven frame pacing and missed the 18/25 ms percentile targets.
These are menu measurements, not a live-combat FPS result. Detailed evidence and
the exact executable hash are in
[`validation.json`](../tests/mountain-combat/validation.json).
