# Beyond-visual-range operations

Open **BVR SETUP** at the upper right. Select a force size, difficulty and mission,
then choose **START MISSION**. All choices are visible together on a dedicated
setup screen. Opening it pauses the flight; **Cancel** or **Escape** restores
the previous pause state. Starting a mission begins the new flight.
Existing target-practice and close-range combat
scenarios remain separate. **R** restarts using the current BVR setup selections.

| Force configuration | Friendly aircraft, including you | Hostile aircraft |
| --- | ---: | ---: |
| Solo intercept | 1 | 2 |
| Two versus two | 2 | 2 |
| Outnumbered pair | 2 | 4 |
| Flight engagement | 4 | 8 |

Each configuration offers **Training, Balanced and Hard**, independently of its
**Elimination** or **Intercept / Defend** mission. This gives 24 selectable
combinations. Flights start approximately 125 km apart at 11–12 km altitude.

The inland theater covers roughly 200 × 350 km of additional rendered land,
east of the original airport area. Mountain ridges and valleys share the same
height function for rendering, collision, terrain clearance and radar/missile
line of sight. Earth curvature remains part of the simulation; radar visibility
is not guaranteed simply by being inside the selected range. The original
airfield and its nearby terrain are preserved.

## Missions and difficulty

Elimination completes when all hostile aircraft are destroyed. Intercept/defend
protects the marked home sector: a surviving attacker entering its 18 km boundary
fails the mission. Holding the sector for 20 simulated minutes or eliminating the
hostile flight completes it. Attack elements press toward the sector; escorts
engage interceptors. Player destruction fails either mission.

Difficulty changes AI observation/control accuracy, reaction delay, launch timing,
launch-range discipline, defensive timing and target allocation.
Hard pilots distribute targets more strongly across their flight. Friendly AI
uses the same selected difficulty. The player and AI share missile propulsion,
guidance, terminal-seeker and damage models; difficulty does not alter BVR
warhead damage probabilities or give the AI a faster missile.

AI pilots fly the same six-degree-of-freedom aircraft plant as the player and
use the same engine, fuel, aerodynamic forces, actuators, FBW and component
failures. They search through the modeled radar beam, maintain uncertain contact
memory, share fresh paired-wingman reports, distribute targets, support missiles,
crank, notch or drag away from detected threats and command stall/terrain
recovery. All aircraft retain full physics frequency. These are game tactical
controllers, not a reproduction of a specific air force's doctrine or real
aircraft avionics. See [AI.md](AI.md) for architecture and diagnostics.

## Radar and tactical map

### Cockpit radar warning receiver

The Gripen's **left MFD** is a passive radar warning receiver (RWR), available
in every scenario and in simplified view. The center display carries engine
and system data; the right display carries the radar. Look down and left and
use the mouse wheel to zoom; Home resets the view. Flight speed, altitude,
attitude and navigation cues remain on the HUD and attitude ball.

The RWR is aircraft-relative: **12 o'clock is ahead**, 3 is right, 6 is behind
and 9 is left. Symbols identify received emissions:

| Symbol | Meaning |
| --- | --- |
| Green S | Search-radar sweep |
| Amber L | Tracking radar or radar supporting a missile toward you |
| Red M with flashing brackets | Hostile active radar missile seeker |

Rings indicate **priority, not range**: searches lie on the outer ring,
tracking emitters on the middle ring and missile seekers on the inner ring.
Leader lines preserve the true bearing when labels need spreading. Contacts
are held for five simulated seconds, dim after one second without an update,
and clear on reset or loss of battery/avionics. Pause holds the display and
flashing state. The receiver is independent of your radar's power switch.

This game approximation receives hostile airborne radar beams within 240 km,
subject to emitter health, pointing and terrain visibility. BVR tracking and
missile-support states come from the existing AI; close-range tracking comes
from the pilot's radar acquisition state. Active radar missile emissions need
an illuminated bearing within 35 km. Passive midcourse missiles and IR seekers
do not appear as RF emitters. Existing missile-approach warnings remain separate.
The receiver prioritizes a bounded sixteen contacts, displaying the top eight.

The principle follows the USAF descriptions of [passive RF warning](https://www.edwards.af.mil/News/Article/396199/new-c-130j-radar-warning-receiver-put-to-test/)
and [bearing, emitter type and missile warnings](https://www.nationalmuseum.af.mil/Visit/Museum-Exhibits/Fact-Sheets/Display/Article/197445/USAFmuseum/radar-homing-and-warning-the-key-to-the-wild-weasels/).
This is a Gripen-inspired game display, not a calibrated receiver, RF propagation
solver or exact reproduction of a Gripen avionics suite.

### Radar control display

The other Gripen panels use the same **Option/Alt + click** interaction:

Both side-console/kneeboard surfaces also carry functional controls. Look down
and left/right with mouse drag, zoom with the wheel, then hold Option/Alt to click.
The left console has BAT, LIGHTS, APU, ENGINE, GEAR, FLAPS, AIRBRK and FBW. A gap
around the throttle preserves its full travel. The right console has RADAR,
R-MODE, TARGET, WEAPON, FLR/CH, APPR, SIMPLE and HELP. Mechanical levers move to
reflect the actual switch state; pushbuttons depress and highlight on hover.
They operate the same systems as the front-panel controls and keyboard inputs.
Home returns the view to the forward cockpit position.

| Location | Physical switches |
| --- | --- |
| Left of RWR | BAT, APU, ENG, LTS |
| Left of engine display | GEAR, FLAP, ABRK, FBW |
| Below displays | TARGET, WEAP, CM, SIMP, AP, HELP |

BAT toggles battery power. APU requires the battery; ENG uses the same battery/APU
interlock as the keyboard. LTS toggles exterior lights. FLAP cycles up/takeoff/
landing, and AP toggles approach assistance. TARGET cycles targets, WEAP changes
IR/radar missile selection, and CM uses the existing combined flare/chaff
dispenser with its cooldown/inventory checks. SIMP and HELP control display
clutter. Cyan illumination shows active switches, and the hover hint names each
action. These controls use the same flight state as the existing keyboard inputs.

In every Gripen scenario, the **right multifunction display** is the radar. Hold
**Option / Alt** and click a track diamond to select that hostile aircraft.
The selected track is amber and bracketed; its ID, range, altitude in kilometres,
track age, bearing (`AZ`) and closure speed in metres/second (`VC`) appear around
the scope. Positive VC means closing. The lower line gives estimated maximum
and no-escape ranges in kilometres, with acquisition, launch or inhibition cues.
`SUP` counts your airborne missiles still waiting for autonomous seeker guidance.

With Option / Alt held, click the physical buttons surrounding the display:

The radar bezel has raised metal sockets and backlit, labelled keys. Cyan lighting
indicates radar power, STT lock, TWS return, or the selected tactical page. Holding
Option / Alt highlights the hovered key; pressing it depresses the cap. The
bottom hint spells out its action. Click targets cover the visible button faces,
and the side keys sit above the right console so LOCK stays visible.

| Buttons | Action |
| --- | --- |
| PWR | Toggle radar power; standby removes player midcourse updates |
| RNG+ / RNG− | Increase/decrease scan range: 40–240 km in BVR, 5–40 km otherwise |
| MODE | Cycle Search, TWS and STT |
| EL+ / EL− | Raise/lower scan elevation by 5 degrees |
| NEXT | Cycle tracked hostile aircraft |
| LOCK | Enter STT on the selected aircraft |
| UNLK | Return to TWS |
| PAGE | Toggle forward range/azimuth scope and heading-up tactical scope |

Select a track, choose **LOCK** (or TWS), keep it within radar coverage, and wait
for acquisition. **SHOOT** appears when the selected weapon's conditions and
dwell are satisfied; **Space** launches. `OUT RNG`, `SET TWS`, `STALE` and
`NOSE ON` explain common inhibited launches. These are game-model cues rather
than a calibrated real radar. Both pages use sensed/coasting hostile tracks;
they do not inherit the tactical map's reveal-all setting.

Release Option / Alt to use either mouse button for camera dragging. Look down
and right to bring the display into view; use the **mouse wheel to zoom** for
readable instruments. **Home** recenters the cockpit view and resets its zoom.
The hover hint identifies the control under the cursor. Cockpit and map radar
controls operate the same sensor and selected target.

The physical cockpit HUD also shows sensed radar tracks: green hostile diamonds,
cyan friendly boxes and a larger amber selected-target diamond. `T` identifies
the designation; `L` identifies a fresh selected STT track. Gray-green symbols
and **COAST** distinguish stale extrapolated tracks; expired tracks disappear.
The selected target receives **STEER** or **AFT** cues at the HUD edge when it is
outside the combiner field. Full view shows contacts within the HUD; simplified
view retains only the selected contact. Radar standby/failure removes these
symbols. The selected range and closure use the same coasting estimate as the
symbol, rather than undisclosed target position.

The clean BVR-style physical HUD and radar symbols are shared by runway,
approach, target-practice and close-range combat flights. Outside BVR, a separate
local scan maintains measured/coasting tracks with power, range, elevation,
Search/TWS/STT, terrain visibility and signal checks. It does not create extra
aircraft in ordinary flights. The existing close-range seeker and missile
guidance model remains in use. Radar standby inhibits radar missile launches.

Both the radar MFD and cockpit HUD show a vertical **launch-zone scale** for the
selected sensed hostile track. **MAX** marks estimated maximum range; the thick
lower band ends at **NEZ**, the estimated no-escape range. A chevron moves with
the target's measured/extrapolated range. Values are kilometres; short distances
use one decimal place. **IN NEZ EST** appears inside the zone. Coasting estimates
are muted and do not assert this cue. The LR-AAM uses the existing BVR envelope;
IR and ordinary radar missiles use their shorter range and a bounded
altitude/speed/closure adjustment. These are energy/geometry gameplay estimates,
not a guaranteed-hit boundary. Seeker lock, minimum range, ammunition and radar
support remain independent launch requirements.

The MFD also shows **blue triangles for friendly missiles** and **red triangles
for nearby closing threats**, with a color legend and visible missile count.
Friendly missiles use own/datalink positions. Hostile missile visibility is a
warning-model approximation limited to 35 km, closing motion and clear terrain
line of sight. Both forward and tactical pages use their current scale, so a
missile behind the aircraft is visible on the tactical page when within range,
while the forward scope retains its ±60-degree limits. Expended missiles vanish.

**M** opens the aircraft tactical map; spacecraft still use their orbital map.
Flight, pause and time-warp settings continue to apply. Map controls:

- **Right-drag:** pan. **Mouse wheel:** zoom from a 20 km to 600 km span.
- **CENTER ON PLAYER:** recenter without changing radar settings.
- **Click a contact:** select it. **Tab:** cycle live tracked hostile contacts.
- **Enter:** switch to STT on the selected contact.
- **RADAR RANGE:** cycle 40 / 80 / 160 / 240 km scan limits.
- **ELEVATION:** move the scan center in 5-degree increments, within ±45 degrees.
- **MODE:** cycle Search, track-while-scan (TWS), and single-target track (STT).
- **RADAR POWER / STANDBY:** toggle the same radar used by the cockpit display.
- **SENSORS / REVEAL ALL:** switch the map's information rules.

Search sweeps a ±60-degree azimuth sector with a 24-degree elevation band. TWS
interleaves maintenance updates for previously acquired tracks and supports
multiple missiles. STT revisits the selected contact and supports that target's
missiles. Search alone does not authorize a radar-missile launch.

Sensor mode shows acquired tracks, friendly positions and friendly sensor reports.
Unobserved tracks coast from their last measured position and velocity, receive a
COAST label after three seconds, and expire by 18 seconds. Initial contacts are
shown as unknown until a second report. The simplified IFF model then identifies
the scenario's hostile aircraft. Friendly sensor reports carry a DL label.
Reveal-all mode shows truth positions for training; switching it on does not
bypass radar tracking or weapon-launch requirements.

Map labels show identity, range and altitude. Missile labels include guidance
state, estimated time to the tracked position and Mach number. Friendly missiles
are visible; hostile missile display is limited to nearby active seekers unless
reveal-all is enabled. Time-to-go assumes current relative motion and is not a
guaranteed intercept time. The terrain image is cached until pan/zoom changes;
positions use double precision until map or local-render projection.

## Weapons

**O** selects IR or the BVR **LR-AAM** active-radar profile. **Space** launches
after tracking requirements and dwell are satisfied. The BVR load is four LR-AAMs
and two IR missiles; each AI aircraft carries four LR-AAMs. Existing close-range
scenarios retain their original radar missile. **6** follows/cycles airborne
missiles; **7** restores the preceding view. **V** remains paired flare/chaff;
**Backspace** is chaff-only. The cannon remains available on **U**.

The LR-AAM uses a 190 kg initial mass, 70 kg propellant, seven-second 27 kN boost
and seventeen-second 5.5 kN sustain. It shares the existing navigation filter,
proportional-navigation guidance, acceleration controller, aerodynamic authority,
drag, gravity and seeker limits. A bounded loft command is applied at longer
ranges. Datalink support is required before autonomous terminal acquisition near
18 km. Radar support can be lost through track aging, power/system failure,
terrain masking, notching or turning outside the support cone. The missile
coasts its estimated track when measurements disappear; active radar can later
reacquire within its own constraints.

**RMAX EST** and **NEZ EST** are gameplay estimates based on launch altitude,
launch speed and target radial closure. They are not lookup tables calibrated
against a real missile, and NEZ does not guarantee a hit. A cold target or a
defensive maneuver can exhaust a missile that could reach a hot target at the
same initial distance. Missile life is capped at 210 simulated seconds, and the
scenario-wide missile pool is bounded at 64 entries.

## Diagnostics

- `cargo test --locked`: scenario/difficulty combinations, radar gating and aging,
  launch inhibition, mission/reset/pause behavior, motor profile and shared
  regressions.
- `--study-ai`: six 60-second control/failure profiles and 28 integrated cases
  using the shared plant for both the scripted player and every AI aircraft;
  results are in `tests/ai-flight-study.json`, `tests/ai-combat-study.json` and
  corresponding flight CSVs. This is not a human combat win-rate study.
- `cargo run --release --locked -- --study-bvr`: 24 tactical engagement cases and
  isolated long-range weapon trials in `tests/bvr-study.json`. The engagement
  study uses an explicitly kinematic player path and is not a six-degree-of-freedom
  flight validation or a win-rate benchmark.
- `--visual-check bvr-map`, `bvr-reveal`, `bvr-flight`, `bvr-setup`,
  `bvr-cockpit`, `bvr-cockpit-tactical`, `bvr-cockpit-missiles`,
  `bvr-hud-missiles`, `air-radar`, `air-radar-hud`: disposable native captures
  of the largest force configuration.

All radar, missile, AI and vulnerability parameters here are explicit game-model
assumptions. Current measured results and native UI evidence belong in the
repository's validation record, rather than being inferred from compilation.
