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

## Imported user-supplied model

The user supplied `/Users/Austin/Downloads/sr71.glb`; source copied byte-for-byte to `assets/aircraft/sr71/source.glb` (SHA-256 `2506ee33cbef686afc49f033f114e4777447f38d75371328a03d98ea3edade01`). Reproducible Blender preparation produces `static.glb`, 31,413 triangles, 24 textures, no animation. Material consolidation reduces 75 source material slots to 26 without decimation or texture resizing. CC BY 4.0 author/source/modification attribution is bundled with the asset.

The focused imported-asset test passed on the final normalized model: embedded images, valid GLB, no animations, 32.73 m length, approximately 17.03 m span and tyre-bottom registration. The previous full 819-pass suite predates this asset replacement; it is not claimed as a fresh full-suite result for the import.

Native imported top/side/front captures exposed reversed facing after Blender/glTF axis conversion, plus an orbit-camera path missed by the earlier distance change. Both corrected. The final front capture verifies forward-facing orientation and uncropped framing. Earlier top/side captures are inspection history and predate these corrections. Final imported gear deployment/contact matching was not exhaustively reviewed. Cumulative approved live runtime: 108.60 seconds, restoration handler completed. No performance benchmark or calibrated flight test was performed.

Final packaged app rebuilt successfully (22.93 s); strict deep codesign verification succeeded. Executable SHA-256: `77d5104d5aaed300ccbd25778d6646927ee5af001532cf241fa595478bcb0b53`. Evidence: `tests/sr71-visuals/imported-front.png`, source logs and review.json. Capture script: `scripts/sr71/capture.py` (requires live-test approval; tracks cumulative runtime). Flight model, cockpit and collision remain provisional.

## Aircraft-selection button fix

The setup screen generated buttons from the six-entry aircraft list, but its action handler still matched only IDs 10–14. The SR-71 button (ID 15) was therefore ignored. Dispatch now derives the accepted range from `Aircraft::ALL.len()`. A regression clicks every aircraft button and then starts the selected SR-71, checking aircraft/damage identity, menu closure and resumed simulation. This fixes setup-menu selection; carrier scenarios remain Hornet-only by design.
