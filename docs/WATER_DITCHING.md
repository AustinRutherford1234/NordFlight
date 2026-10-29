# Water impact, ditching and sinking

Water contact now enters `src/water.rs` rather than the dry-land crash/suspension path. It applies to the existing lake and carrier sea region, excludes supported carrier-deck contacts, and works with the shared flight loop and dedicated Hornet plant.

## Estimated behavior

A horizontal contact speed of at most 85 m/s, descent rate at most 6 m/s, bank below 20 degrees and pitch between -5 and +18 degrees permits an initially intact ditch. This is a gameplay-oriented engineering estimate, not a certified aircraft ditching envelope or real-world guidance. Exposed gear can break above 18 m/s; contact above 60 m/s or 3.5 m/s descent can break a wing. Faster, steeper or poorly oriented contact enters the existing breakup damage state, extinguishes fire and continues through the water solver.

Contact shuts engines down, cancels carrier attachments and disables autopilot. Water drag decelerates the hull; immersion-dependent buoyancy initially supports a gently ditched aircraft. Flooding progressively removes buoyancy, faster with broken-wing damage, and the aircraft sinks. Vertical and horizontal drag use bounded exponential updates. The model has no waves, detailed hull hydrodynamics, watertight compartments or seabed contact. Scenario reset creates a fresh water state.

The renderer emits upward blue-white spray with ballistic fall, an outward foam patch and fading mist-sized droplets. Spray height scales with contact speed and descent. Water impact suppresses the player fireball, flame/smoke emission and crash-blast sound. Detached parts can still produce debris. The pool remains capped at 256 particles and pauses with simulation time.

## Validation

619 tests passed in the final full regression run (18.61 seconds). Tests cover gentle float-then-sink, excessive-energy breakup, gear/wing damage, carrier-deck exclusion, 60/120/240 Hz consistency and spray/foam without fireball, followed by particle expiry. The sound change excludes water impacts from the existing blast event; 26 focused sound tests also passed. Reset and underwater-restart checks are included.

Native captures can be reproduced with `python3 scripts/capture_water.py`, after `./build-mac.sh dev`. Evidence is local under ignored `tests/water/`. Capture states are gentle impact, high-speed impact, four seconds after contact and 35 seconds after contact. These are deterministic native renders rather than a manual flight review.

The four native renders were inspected. A foam-plane orientation problem found in the first render was corrected, and spray opacity was reduced before recapturing. The gentle case remained intact at the surface, was partially submerged after four seconds and was about 46 m below sea level after 35 seconds. The high-speed case generated spray and debris without a fireball.
