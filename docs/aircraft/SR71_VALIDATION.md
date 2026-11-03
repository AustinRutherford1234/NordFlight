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
