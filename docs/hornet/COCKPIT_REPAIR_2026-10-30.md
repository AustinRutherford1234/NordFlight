# Hornet cockpit fit, detail and canopy-reset repair

The reported areas were the front instrument panel, side consoles, an extra canopy support ring, and a canopy that stayed missing after ejection/reset.

## Repairs

The generic canopy bows, posts and sill rails were being generated in addition to the fitted tandem Hornet structure. The Hornet now uses its own framing. Other aircraft retain their existing framing.

The generic front backing was narrower than the three live displays, and the display glass was separated from its supporting boxes. A fitted wide panel, recessed display cases, frame rails and captive fasteners now support the existing glass positions. The split glare shield leaves a supported opening for the central FCS display. Vent strips, edge trim and lower-panel fittings add shape to the previously plain blocks.

Side-console decks now meet the existing switch faces. Front dummy switch rows overlapping those functional controls have been removed. Extended console tops, inset plates, screw heads, subdued placards, sidewall service panels, conduits, sill seals and latch bases give the cockpit a coherent finish. The floor is recessed, with pedal supports and access covers. Forward bulkheads, cheek returns and an extended floor close the footwell gaps exposed when the overlapping external roof was removed. Seat shells have softened edges and attached seat-back fittings. Rear repeater glass and frame rails share the backing panel's tilt and a common placement helper.

The enlarged external nose-wheel well roof intersected the seat area in first-person view. External bay, gear and strut geometry now follows the same first-person exterior-culling convention as the imported fuselage, leaving the fitted tub responsible for the cockpit interior. Switching outside restores that geometry. This is a camera presentation correction, not a new physical wheel-well/cockpit packaging validation.

The canopy pivot was explicitly hidden during ejection but never explicitly restored in the normal state. Its visibility is now assigned on every update. Reset restores the same pivot and glazing entities; cockpit-camera glazing culling remains separate from ejection visibility.

The [U.S. Navy F/A-18F photograph](https://www.dvidshub.net/image/6340532/sailor-cleans-cockpit-f-18f-super-hornet-flight-deck-aboard-nimitz) was inspected for the tandem canopy framing and finish. Generated interior fittings are representative game geometry; the existing functional display and switch positions remain the interface reference.

## Runtime cost and compatibility

Decorative parts use the existing rigid-geometry batching path. One static 512-by-128 RGBA label atlas adds 256 KiB of pixel data; there are no additional live instrument pages or per-frame decorative animations. Small hardware uses 12-sided cylinders. Rear display sharing and HUD angular calibration remain covered by regressions. No FPS or memory-footprint improvement is claimed without a separate benchmark.

## Automated evidence

`cargo test --locked`: 680 passed, zero failed. Coverage includes:

- Repeated real ejection starts and scenario resets on the same canopy entities, including carrier scenarios and cockpit/external visibility transitions.
- Front screen sampling checks: no fitted casing in front of the tested live glass points and supporting casing within 16 mm.
- Opaque forward footwell coverage below both panels.
- Finite generated meshes with consistent vertex attributes.
- Shared rear repeater materials and unchanged HUD angular scale.
- Existing ejection, aircraft, carrier, HUD and physics regressions.

## Native review and delivery

The six final views were captured and inspected: `cockpit-panel`, `cockpit-left`, `cockpit-right`, `cockpit-floor`, `cockpit-rear` and `cockpit-reset-canopy`. Earlier review exposed the wheel-well/seat overlap and the forward footwell opening; those were corrected and the final captures refreshed.

The reset preset starts an ejection after the installed scene has loaded, verifies that the installed canopy pivot becomes hidden, resets the flight in that same process, then verifies that the same pivot is inherited/visible again before the screenshot. The checker follows the installed aircraft's ancestor chain, excluding the detached escape prop with matching node names. The final image shows the restored canopy. Previous user app sessions were resumed after each diagnostic batch.

Local evidence is retained in `tests/hornet-cockpit-repair/`; the tracked runner is `scripts/fa18f/capture_cockpit_repair.py`. No application errors or assertion failures were found in the final logs. These are scripted visual checks, separate from the user's flight review.

`./build-mac.sh dev` updated Nord Flight.app. Strict deep signature verification passed. Build and packaged ARM64 UUIDs match: `40FB2593-1EF8-37F6-9858-9B7F732C0716`. Relaunching the previously running app is needed once to load this update; subsequent ejection/reset cycles restore the canopy in-session.
