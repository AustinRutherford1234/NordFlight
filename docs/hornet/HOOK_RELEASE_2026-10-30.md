# Arresting-wire release on hook retraction

The arrested flag previously survived a hook-up command. The force solver and wire renderer continued using the retracting tip, and crossing detection could catch another wire while the animated hook remained down.

Engagement now requires both the hook-down command and a deployed tip angle above 0.6 rad. Physics load evaluation and post-step crossing detection clear an incompatible attachment before applying cable loads or considering another catch. The wire renderer uses the same eligibility rule. Release clears active payout/speed telemetry; dissipated cable energy remains event history until the next catch resets it. Existing arrest-force, suspension and hook-animation parameters are unchanged.

Regression coverage includes hook retraction at 0, 40 and 70 m/s and several animation angles, comparison with unattached force/moment output, post-step release, prevention of immediate recatching, and subsequent engagement when commanded down again. Validation results are recorded below. No interactive app review was run for this change.

- `cargo test --locked carrier:: -- --nocapture`: 30 passed.
- `cargo test --locked`: 672 passed, zero failed, including the carrier roll-stability regressions.
- `./build-mac.sh dev`: rebuilt the installed Mac app; strict deep signature verification passed. Packaged and build ARM64 UUIDs match: `0510400E-40CA-3890-9B59-E17FB208F271`.
