# Aircraft damage and destruction

Player, opposing aircraft, allies and practice targets share persistent structure, propulsion, radar, avionics, hydraulic, fuel and control-surface damage and the same six-degree-of-freedom flight dynamics. The component strengths, missile odds, collision threshold and fire hazard are gameplay parameters. AI pilots command the physical aircraft through normal controls; see [AI.md](AI.md).

## Missile outcomes

Each successful detonation samples seeded randomness once. Both missile types and all difficulties use the same categories. The nearest point on the weapon's path relative to the moving aircraft locates the burst. Distance is measured to the compound convex airframe skin, including wings, and divided by the weapon's fuse radius. These probabilities are smoothly interpolated:

The following original profile now applies to the player and allied aircraft. Enemy aircraft use the more lethal profile below.

| Burst distance / fuse radius | Minimal | Critical | Destruction |
|---|---:|---:|---:|
| Direct airframe contact: 0 | 5% | 25% | 70% |
| Mid-distance: 0.5 | 25% | 50% | 25% |
| Outer fuse edge: 1 | 55% | 40% | 5% |

Enemy missile hits have an immediate destruction probability of `0.97 - 0.17 × normalized burst distance`: **97% at contact, 88.5% halfway to the fuse limit, and 80% at the fuse edge**. The remaining probability is split between minimal and critical damage in the original profile's relative proportions. Both missile types and every difficulty use this enemy profile. Surviving hits still resolve localized failures and accumulated damage can also destroy an aircraft. Player/allied vulnerability is unchanged.

Cannon hits against enemies now apply **4× the normal component and hull dose**, with the existing per-hit dose cap and impact-speed scaling. Hits on the player or allies keep their existing strength. Damage remains localized and cumulative; firing speed, projectile speed and ammunition counts are unchanged.

Minimal damage reduces hull integrity slightly and may add a fuel leak, without newly disabling a critical system. Critical damage disables one system nearest the burst and damages adjacent regions: engine, hydraulics, radar, avionics, wing structure or a control surface. Surface-critical hits have an equal chance of jamming or detaching that surface. Destruction causes immediate breakup. All damage accumulates, so another hit can finish an aircraft already near structural failure. Cannon rounds retain shaped swept collision and cumulative damage, scaled by relative impact speed squared.

## Flight and surface failures

The nine moving surfaces are left/right canards, inner/outer elevons, airbrakes and rudder. Each has an operational, degraded, jammed or detached state. A jam preserves its angle at impact and its fixed aerodynamic loads, while removing its response to control commands. Detached surfaces disappear from the attached model, generate a moving fragment, lose their loads, and add exposed-edge drag. Damaged wing structure removes lift and produces roll imbalance. FBW allocates commands against remaining authority, including the constant moment from jams.

Engine failure removes thrust. Hydraulic failure freezes actuator response. Radar and avionics failures affect weapon support and flight assistance. Opponents experience those same force, moment and actuator failures through the shared plant, with finite fuel and leakage. Missile jams capture their actual surface deflections. Scenario restart restores the aircraft and clears effects, pilot memory and contact state.

## Fire, effects and collisions

Fire is primarily visual and does not continuously reduce ordinary hull health. Engine shutdown lets an engine-fed fire decay, while fuel exhaustion extinguishes fuel-fed fire. A fuel-fed fire above intensity 0.3 can cause a delayed explosion after five seconds, with a 0.1% hazard per simulated second. An exponential waiting time provides the same stochastic behavior across timesteps; extinguishing the fire clears the pending hazard.

Shared effects use a pool capped at 256 particles. Flame/smoke billboards originate at damaged regions; breakup emits a flash and major fragments that retain aircraft momentum, then experience gravity, drag and terrain contact. Effects follow simulation time and pause. Fragment shapes are approximate debris rather than a complete structural fracture model.

Player–enemy and enemy–enemy collisions use continuous separating-axis checks between convex airframe pieces, with distance bounds before narrow-phase checks. Relative contact speed of at least 25 m/s destroys both aircraft. Slower contact applies local damage and velocity changes, then separates the skins. Contact is debounced until separation and destroyed aircraft are counted once. Orientation is fixed within each 1/120-second contact sweep; there is no continuous angular rigid-body collision solver.

## Damage display and controls

The small top-left schematic always shows the player's aircraft in full, cockpit and simplified flight views. Cyan means healthy, amber degraded, magenta jammed, and red failed/detached. A flame icon marks fire. The compact failure list shows three prioritized faults and a count for additional faults. The orbital map hides it.

A confirmed player missile or cannon hit on an enemy also opens a matching schematic, labeled with that enemy's number and HIT CONFIRMED or DESTROYED. In full view it sits beside your own schematic; in simplified view it occupies the upper right below the view-status badge. It records component health and failures at impact, holds for **3 simulated seconds**, then fades over **0.65 seconds**. The latest hit replaces the previous report and restarts its timer. Changing selected targets or destruction does not discard the snapshot. Pause freezes it; scenario restart clears it. It remains visible in cockpit/simplified flight views and is hidden on maps. This is hit feedback, not a continuously updated enemy-health sensor.

V deploys one flare and one chaff charge, subject to their independent inventory and cooldown. An available dispenser still works if the other is empty or cooling. Backspace remains chaff-only, and separate mouse buttons remain available. Countermeasures influence seekers and cannot guarantee survival.

## Verification and physical limits

Tests cover probability anchors and seeded distributions, survivable and catastrophic outcomes, local failures, actual player dynamics with jammed/detached surfaces, bounded effects and pause/reset, paired dispensers, swept and low-speed contacts, enemy–enemy collisions, and delayed-fire timing. Native captures inspect failures, smoke/fire, breakup, collisions and the cockpit schematic. `--study-air-combat` exercises enemy weapons against actual player dynamics.

NASA's [damaged-aircraft flight-dynamics study](https://ntrs.nasa.gov/citations/20120014568) supports modeling incremental aerodynamic asymmetry and degraded controls/thrust. It does not supply this game's component strengths, probabilistic lethality or fire rates. No calibrated Gripen vulnerability, fragmentation, armor, aeroelasticity or structural fracture fidelity is claimed.

## Surface damage detail

Eight surface-fitted cosmetic regions on player and AI aircraft respond to wing,
engine/tail, nose/radar/avionics and central/fuel/hydraulic damage. Charred paint,
fragment pits and exposed-metal flecks become more opaque as the corresponding
components deteriorate; resetting component damage clears them. They are visual
marks rather than geometric penetration holes or a new fragmentation solver.

Burning regions emit sparks in addition to flame and smoke. Smoke darkens near
the source and fades to ash grey, and breakup fragments use folded, jagged metal
geometry. The existing 256-particle pool, simulation-time aging, pause behavior,
damage probabilities and delayed-explosion rules remain in force.

