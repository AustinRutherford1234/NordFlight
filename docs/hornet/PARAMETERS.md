# F/A-18F physics parameter register

2026-10-28. SI throughout; angles are radians. These are reduced-order simulation parameters, not an operational flight manual. Uncertainty ranges below describe sensitivity-study bounds, not statistical confidence intervals. They have not all been swept. A generic engineering estimate applies to the simulated F/A-18F configuration only; it is not a measured Super Hornet derivative.

## Evidence

- [NAVAIR F/A-18E/F overview](https://www.navair.navy.mil/product/FA-18EF-Super-Hornet): conventional twin F414 installation, two-seat F variant, 22,000 lbf thrust class per engine, Mach 1.8+ and 50,000 ft+ advertised envelope. These are broad specification checks, not a thrust deck or aerodynamic database.
- [GE F414 family data sheet](https://www.geaerospace.com/sites/default/files/datasheet-F414-Family.pdf): sea-level standard-day thrust class 98 kN. This is a class rating, not installed net thrust at every speed/altitude.
- [NASA, Modeling Flight, SP-2009-4519](https://www.nasa.gov/wp-content/uploads/2023/04/sp-4519.pdf): describes the Super Hornet's redesigned leading-edge extensions and extensive configuration-specific wind-tunnel development. Used as qualitative support for including vortex lift and configuration dependence, not as coefficient values. Legacy F/A-18/HARV results are not silently substituted for F/A-18E/F measurements.
- Geometry: current imported, articulated `assets/aircraft/fa18f/rigged.glb`; transform and measured wheel/nozzle locations documented in the renderer and rig scripts. Geometry conformity is limited by this asset.

## Register

| Parameter / implementation | Nominal and units | Provenance / confidence | Sensitivity bounds |
| --- | --- | --- | --- |
| Reference wing area / span / chord | 46.5 m² / 13.62 m / 3.76 m | Existing aircraft dimensions; medium; chord is reference value | ±2%, ±1%, ±10% |
| Empty aircraft, excluding added crew and pylons | 14,550 kg | Existing project estimate, low | ±10% |
| Crew and pylons | 180 kg / 160 kg | Estimated two occupants and aggregate pylon hardware, low | 140–220 / 100–250 kg |
| Removable wing / tail / fin component masses | 750 / 150 / 150 kg per side, included in empty mass | Engineering estimate, low | ±40% |
| Central inertia diagonal (pitch/yaw/roll) | 140,000 / 175,000 / 20,000 kg m² | Estimated central dry body, low; component/tank/store parallel-axis terms added separately | ±40% |
| Fuel distribution | 35% forward, 25% aft, 20% each wing | Estimated tank representation, low; total remains current fuel | Each allocation ±0.1, renormalized |
| Fuel locations | (0,0,-1.4), (0,0,1.8), (±2,0,0) m | Geometric engineering estimate, low | ±0.5 m |
| Stores | 84.37 kg AIM-9X at first two stations; 152 kg AIM-120 at remaining occupied stations | Existing representative AIM-9/AIM-120 loads, medium for these simulated stores only | ±10%; no bomb/tank mass claim |
| Wing strip geometry | 4 strips/side, 4.8 m² each | Reduced-order allocation, low; not additional aggregate wing lift | ±15% area allocation |
| LEX / stabilator / fin contributions | 3 / 4 / 3.8 m² per side | Effective aerodynamic element estimates, low | ±25% |
| Wing lift tables | α = 0…π; CL = 0, .48, .92, 1.28, 1.55, 1.2, .55, 0, 0 | Engineering estimate, low; shape-preserving bounded smooth interpolation | ±25% attached lift; ±50% separated lift |
| Mach lift factor | 1 at low Mach, 1.12 at .9, .85 at 1.1, .5 at 2.2 | Engineering estimate, low | ±25% |
| Vortex contribution | .3 sin(2α) with bounded incidence envelope | Qualitative NASA support; numerical fit estimated, low | 0–2 times nominal |
| Separation onset / full separation | .30 / .65 rad, LE droop increases onset | Engineering estimate, low | ±.08 / ±.15 rad |
| Flow lag / separation / reattachment | .12 / .08 / .35 s | Engineering estimate, low | 0.5–2 times nominal |
| Wing parasite CD / fuselage aggregate CD | .018 / .008, with separate gear/store/brake increments | Engineering estimate, low; no generic polar added | ±40% |
| Induced drag | aspect ratio 4, efficiency .78 | Effective reduced-order estimates, low | efficiency .6–.9 |
| Wave drag increment | 0 below .75 Mach, peak .024 near 1.1 | Engineering estimate, low | ±50% |
| Fuselage normal force | three 2.5 m² elements, .6 sin(2α) | Engineering estimate, low | ±50% |
| Tail / fin slopes | 3.2 / 2.3 per rad | Engineering estimate, low | ±30% |
| Actuator travel / unloaded rate / acceleration | ±.55 rad / 1.8 rad/s / 16 rad/s² | User-requested 2x actuator time scale; engineering estimate, low; hinge-load proxy q/100,000 Pa reduces rate | ±30% |
| Hydraulic pressure | engine-powered 1; APU .4; 3 s decay/recovery | Estimated normalized authority, low | APU .2–.6, lag 1–6 s |
| Per-engine dry / wet rating | 62,500 / 97,860 N | Dry estimated, low; wet is public 22,000 lbf class, medium | dry ±15%, installed wet ±10% |
| Engine locations | (±.475,.48,8.202) m thrust line | Measured model nozzle locations, medium | Mesh-dependent, not manufacturer engine-CG data |
| Engine spool rise / fall | 1.4 / .9 s | Engineering estimate, low | ±50% |
| Thrust lapse / ram | bounded density and Mach tables in `engine.rs` | Engineering estimate, low; no public installed thrust deck used | ±25% |
| Fuel coefficients dry / added AB | 21e-6 / 65e-6 kg/(N s) | Engineering estimate, low | ±30% |
| Engine thermal time constant | 12 s; damaged-engine excess temperature can degrade health above 1120 K | Engineering estimate, low | ±50%; failure threshold ±100 K |
| Inlet-distortion flameout / restart | sustained severe α/β distortion >1.2 s; restart below 12 km with APU or >80 m/s | Bounded engineering estimate, very low | Thresholds ±50%; not an operational envelope |
| Wing / tail / fin bending | 6 / 8 / 10 Hz, effective masses 210 / 65 / 80 kg | Engineering estimates, low | frequencies ±30%, modal masses ±40% |
| Wing / tail / fin torsion | 12 / 15 / 16 Hz; stiffness 4.4e6 / 110,000 / 140,000 N m/rad | Engineering estimates, low; chosen to avoid artificial static divergence in tested range | ±30% frequency, factor 2 stiffness |
| Bending / torsion damping | .18 / .22 damping ratio | Engineering estimate, low | .08–.3 |
| Load damage thresholds | +7.5 / -3 g equivalent distributed wing load; 90 kPa; Mach 1.85 | Simulation limits, low; no fatigue-life validation | ±20% |
| Skin thermal state | recovery .85, lag 40 s, degradation above 450 K | Engineering estimate, very low | ±50% lag; threshold ±50 K |
| Nose/main wheel radii | .285 / .41 m | Measured rig geometry, medium | ±.02 m |
| Nose/main suspension travel | .40 / .48 m | Engineering estimate, low | ±.1 m |
| Nose/main spring and damping | 700,000 / 470,000 N/m; 45,000 / 45,000 N s/m | Estimated load-response model, low | ±40% |
| Bump stop / load cap | 2e6 N/m / 900,000 N per wheel | Engineering estimate, low | ±50% |
| Tyre friction / rolling / brake | lateral μ .65 land / .35 deck; 14,000 N s/m cornering stiffness; rolling .016; added brake μ .55 | Engineering estimate, low | lateral .4–.9, rolling .01–.03 |
| Carrier speed / outline / deck altitude | 12.86 m/s; measured polygon; 23.23 m nominal local height | Existing mode specification and Grant's mesh, medium | Outline is coarse; not a deck survey |
| Hook pivot / shank / lowering angle | body (0,.15,5.16) m; 3.5 m; .65 rad | Imported pivot plus shank estimate, medium/low | tip location ±.2 m |
| Arrest reaction point | Hook pivot (0,.15,5.16) m; tip remains the crossing/payout geometry | Reduced-order articulated-hook estimate, low; explicit hinge/damper moments omitted | Pivot ±.2 m; joint torque model unvalidated |
| Launch attachment | (0,-1.4,-3.61) m | Nose gear rig / launch-bar estimate, low | ±.2 m |
| Nose wheel steering | 2 rad/s steering/caster rate, ±.5 rad caster limit above 20 m/s with released steering | Engineering approximation, very low; not a verified F/A-18F NWS law | Rate ±50%; threshold 10–30 m/s |
| Catapult | 131.865 m modeled-rail stroke, 78 m/s design deck-relative speed; force cap 800 kN; down-component ratio .38 | Rail length measured from the imported asset; energy/stroke schedule estimated, low; mass, drag, engines integrated normally | ±20%; no proprietary catapult curve |
| Arresting absorber | 180 kN base, 2,500 N/m payout, 2,000 N s/m damping, 700 kN cap | Engineering estimate, low | ±40% |
| Cable limits | 120 m effective payout, 90 MJ dissipated work | Engineering estimate, low | ±30% |

No parameters above establish certified departure behavior, engine restart procedures, fatigue life, flutter boundaries, carrier operating limits, or weapons-release safety.

Optional Hornet external tanks and their estimated parameters are documented in [STORES.md](STORES.md).

Updated catapult joint and geometry parameters are in [CATAPULT.md](CATAPULT.md).
