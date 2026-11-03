# SR-71 static preview validation — November 2, 2026

- Full `cargo test --locked`: **819 passed, 0 failed, 1 intentionally ignored**, 26.27 seconds.
- New mesh test checks finite positions and valid indices. The existing all-aircraft missile regression now explicitly checks that SR-71 firing is rejected rather than assuming every aircraft is armed.
- SR-71 uses the existing aircraft list, slug parser, hangar selection and root transform/visibility integration. Its renderer creates no moving-surface, wheel-spin, door, nozzle or plume animation components.
- Source regression testing is not flight-envelope validation. No SR-71-specific trim, maximum-speed, stability or performance calibration has been performed.
- Generic collision/contact geometry is provisional and is not derived from the new visual mesh. Fixed visible gear does not follow the simulation's gear command. There is no bespoke cockpit. Use external views for this preview.
- Live-app visual-check approval was requested separately. No additional running flight was interrupted without that approval; absent approval, native appearance, camera framing and flight handling remain unverified.
- Build/signature results are recorded below after packaging. `output/` is unrelated user work and was preserved.

## Packaged app

`./build-mac.sh dev` succeeded (49.67 seconds). Strict deep signature verification succeeded. Packaged executable SHA-256: `1458e84cfa1aa21d92b8b742973e38b07634e12695707269b3a15bf3b667befd`.

## Approved native visual review

Three packaged-app views (top, side, front) captured in 44.49 seconds of the approved two-minute window; existing flights restored by the capture handler. Images inspected: static aircraft is visible and selected, but the initial orbit distance crops the large airframe. SR-71-only default orbit distance was doubled and the app rebuilt/signature checked. Corrected framing has not yet been recaptured. Geometry remains a rough authored preview. Local evidence: `tests/sr71-visuals/`.

A subsequent search identified manilov.ap's CC BY 4.0 SR-71, 31.4k triangles, with GLB download options: https://sketchfab.com/3d-models/sr71-908985d8ec544638bcd661bc315597ad . Browser download was attempted but no retrievable file was obtained; this online model is not installed.
