# Hornet missiles, external tanks and actuator response

2026-10-28. The F/A-18F carries **two AIM-9X and six AIM-120** in the existing missile inventory. An optional loadout adds **three 480-US-gallon fuel tanks**, one centreline and a symmetric inboard wing pair.

## Controls

Open **F6 → F/A-18F → EXTERNAL LOADOUT: TOGGLE**. The readout shows whether the next scenario start includes tanks. Choose **START / RESET SCENARIO** to apply. Tanks are optional; a clean missile loadout remains the default. The selected tank configuration survives R/scenario resets, which restore fresh tanks and fuel. Changing aircraft clears its dynamic tank/drop state.

- **O** selects AIM-9X or AIM-120, **Tab** selects a target, and **Space** requests launch using the existing radar/seeker, lock-time, gear and ammunition checks.
- **Option/Alt+J** drops the Hornet's external tanks while retaining missiles.
- **Option/Alt+Shift+J** jettisons tanks and all missiles.
- The Hornet's labelled **TANK JETT** cockpit button drops tanks only.
- Other aircraft retain their previous jettison controls and missile behavior.

The hangar applies the loadout as an explicit scenario reset. It is not a new in-flight refuelling or carrier servicing menu.

## Missile integration

The Hornet's IR stations now use the existing detailed AIM-9X body, seeker model, countermeasure discrimination, thrust-vectoring guidance, animated in-flight fins and motor presentation. Its existing estimated 10 km acquisition range and 60-degree nose-cone checks match the project's F-22 AIM-9X settings. These are gameplay estimates, not claimed operational envelopes or a new helmet-mounted sight.

The wingtip missiles launch along their rails with inherited aircraft motion and body-rate velocity, without the generic downward ejection used for rack-launched stores. Both ordinary and BVR player release paths use this distinction. AIM-120s retain the existing radar-missile model and standard/BVR guidance profiles, detailed exterior, radar support and active terminal behavior. No proprietary missile performance database is claimed; [MISSILES.md](../MISSILES.md) and [BVR.md](../BVR.md) describe their existing limitations.

The HUD now reads **9X**, common weapon readouts identify **AIM-9X**, and missile inventory controls mounted-store visibility. The six radar stores occupy the outer/mid wing pylons and two fuselage shoulder stations; the latter were moved inboard so they do not occupy the tank stations. Attached and released stores use the same station coordinates. Wingtip missiles and rails follow the existing fold pivots; launch is inhibited while the wings are folded. Missile mass is 84.37 kg for AIM-9X and 152 kg for AIM-120 in the Hornet mass model.

## Tank model

Tank bodies are original code-generated meshes with curved noses/tails, circumferential seams, stabilizing fins and mounting detail. A volume check requires space for the nominal fuel capacity plus at least 5% for shell, equipment and ullage. Attached and released instances share the same geometry. Released tanks retain aircraft velocity and rotational-point velocity, receive an estimated 2 m/s downward separation impulse, and then integrate gravity, drag and slow tumbling independently. The aircraft receives the opposite linear/angular release impulse. Rendering uses the actual dropped-store state, not a fade or a disappearing attached mesh.

Each tank holds 480 US gal × 3.785411784 L/gal × an estimated .8 kg/L density = **1453.60 kg of fuel**. Estimated dry mass is **130 kg**. Three full tanks add **4750.79 kg**, including **4360.79 kg of fuel**. The fuel density, dry mass, dimensions, drag, impulse and tumble are simulation estimates. Empty tank shells remain mounted until released.

The engine model draws external fuel first, proportionally across the installed tanks, then internal fuel. Tanks can continue supplying the engines when internal fuel is zero. The existing infinite-fuel option suppresses consumption. Jettison discards each tank's remaining fuel; it does not transfer that fuel to the aircraft. Total usable fuel is reflected in the Hornet HUD, main telemetry, fuel gauge, low-fuel warning and engine sound availability, with a separate external-fuel readout in the cockpit.

Mass, centre of gravity and inertia include each installed tank's dry mass and actual remaining fuel at its station. Tank drag uses estimated CD .006 per tank referenced to the aircraft wing area, applied at the individual station with local rotational airflow. The aggregate aircraft drag excludes that contribution before these forces are added, preventing double counting. Asymmetric release changes mass properties and aerodynamic moments. Released records are bounded to three, expire after 45 seconds, and stop at terrain/water/carrier deck contact. They are visual/physical debris, not new damage-producing weapons.

Airborne scenario setup increases initial airspeed with the square root of the loaded mass ratio, then solves trim for that loadout. Runtime jettison does not impose a new speed or attitude.

## Twice-as-fast actuators

At the user's request, Hornet flight-control actuator time scale is doubled. Unloaded rate increases from **.9 to 1.8 rad/s**; target-error gain increases **10 to 20 per second**; acceleration increases **4 to 16 rad/s²**, which scales the acceleration ramp consistently with the 2x time scale. The FBW allocator shares the new rate limit. Travel, pressure/loading, hydraulic health, power and jam limits remain active. Gear, canopy, hook and other aircraft mechanisms retain their previous speeds.

## References

[NAVAIR's Super Hornet fact sheet](https://www.navair.navy.mil/product/FA-18EF-Super-Hornet) identifies AIM-9X/AIM-120 compatibility and three 480-gallon tanks in the ferry configuration. [Fleet Readiness Center Southwest](https://frcsw.navair.navy.mil/Public-Affairs-Office/News/Article/2613535/frcsw-shop-services-hornet-fuel-tanks/) independently describes the Super Hornet's 480-gallon tank. These support aircraft/store compatibility and nominal capacity; they do not validate our mass distribution, drag, release trajectories or handling estimates.

Validation and delivery evidence: [STORES_VALIDATION.md](STORES_VALIDATION.md).

| Parameter | Value / units | Evidence and uncertainty |
| --- | --- | --- |
| Tank capacity | 480 US gal each | Public Navy nominal capacity; high confidence for the tank class |
| Fuel density | .8 kg/L | Engineering estimate; ±5% sensitivity range |
| Tank dry mass | 130 kg | Unverified engineering estimate; ±50% |
| Tank length / body diameter | 4.75 / .864 m | Reference-style geometry estimate; ±15% |
| Tank drag increment | .006 per tank on 46.5 m² reference | Engineering estimate; ±50%; not flight calibrated |
| Separation speed | 2 m/s body-down relative to the rack | Engineering estimate; ±50%; opposite aircraft impulse applied |
| Tank tumble rates | .13 / .04 / .08 rad/s | Presentation estimate; ±100%; no validated released-store attitude model |
| Control actuator rate / gain / acceleration | 1.8 rad/s / 20 per s / 16 rad/s² | User-requested 2x time scale of the existing estimates; travel/authority limits retained |

These ranges describe possible sensitivity studies, not completed sweeps or statistical confidence intervals.
