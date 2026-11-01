# Low-speed Hornet inlet operation

The previous inlet-distortion estimate used angle of attack and sideslip alone. When parked facing aft on the 25-knot carrier, the actual air-relative velocity points into the rear of the aircraft. This produces an incidence near 180 degrees at only about 100 Pa dynamic pressure. The angle-only rule accumulated its 1.2-second distortion timer and falsely flamed out both engines.

The new estimate computes incidence directly from body-frame air-relative velocity and multiplies it by a smooth dynamic-pressure factor. Directional freestream distortion is zero below 250 Pa, transitions continuously with a cubic smoothstep, and reaches the previous full effect at 4,000 Pa. The same estimate drives inlet pressure recovery, flameout accumulation and restart eligibility. Carrier velocity is not subtracted from ambient airflow: the engines still see the actual wind. No ground-only immunity is applied.

These pressure bounds are engineering estimates for a reduced-order game model, not an F414 compressor map or validated tailwind limit. Low-speed engine-induced intake flow is represented by reducing the relevance of freestream incidence. Strong distortion at flight speed still accumulates flameout exposure. Damage, pilot shutdown, fuel exhaustion and existing APU/windmill restart requirements remain active.

Primary background: [NASA inlet performance](https://www.grc.nasa.gov/www/k-12/airplane/inlet.html) describes inlet operation, engine airflow demand, and distortion sensitivity at high incidence. [NASA inlet model discussion](https://www.grc.nasa.gov/www/k-12/airplane/tunmodinlt.html) describes distorted-flow compressor stalls and the effects of angle of attack and yaw. Neither supplies the numerical calibration used here.

Regression cases exercise eight headings on the moving carrier at 60/120/240 Hz, static operation, 10/20/30 m/s head/cross/tail winds at idle and full power, high-speed distortion flameout and APU restart, independent engine damage, shutdown, fuel exhaustion, and pressure-blend boundary continuity. These are deterministic engine bench tests; full game regressions cover integration. No live-flight interruption is needed for this repair.

Validation: four new engine regression tests passed, followed by the complete game suite: 757 passed, zero failures (48.52 seconds). The carrier-heading bench cases span 60/120/240 Hz; all healthy engines maintain positive thrust/fuel flow and hydraulic authority. No claim of measured F414 crosswind/tailwind certification limits is made.
