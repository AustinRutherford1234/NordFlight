# Larger naval missile smoke

## Visual references and interpretation

The [USS Higgins SM-2 launch photograph](https://www.dvidshub.net/image/8326257/uss-higgins-live-fire-missile-launch) was inspected directly. It shows a dense, lumpy launch cloud spreading around the VLS, a rising white exhaust column and brief warm illumination near the motor. This motivates a distinct launcher cloud in addition to the moving exhaust trail. The [NZDF air-launched Harpoon reference](https://www.nzdf.mil.nz/air-force/air-force-news/rnzaf-p-8a-poseidon-successfully-launches-harpoon-missiles/) was also inspected; its published still shows the weapon separating from the aircraft, rather than a VLS-like launch cloud. Aircraft launches therefore do not spawn a deck cloud.

These are visual references, not measurements of plume size, opacity or motor chemistry. Images are not bundled with the game. The game's representative air-to-ship missile receives the requested fuller trail as an artistic approximation; it is not a claim that every jet-powered cruise missile produces a solid-rocket-like smoke column.

## Presentation

- Standard/ESSM-class ship missiles use a wider, billowing trail: nominal width is 5.5 times the previous radar-missile trail, with asymmetric lobe displacement, rising drift, denser opacity and a shaded smoke texture.
- Air-to-ship cruise missiles use a separate 3.4-times-width profile. Ordinary air-to-air missile profiles remain unchanged.
- Smoke begins closer to the exhaust, with a smooth fade-in from 4 to 14 m behind the nozzle. The default air-to-air clearance remains 16 to 30 m.
- Ship launches create a separate cloud that expands, inherits the ship velocity initially and relaxes toward ambient wind with an estimated three-second response and fades over 12 simulation seconds. A brief warm tint fades rapidly to soft gray-white. Cloud lifetime is independent of the live missile, so a quick interception does not instantly remove the launch cloud.
- The appearance comes from layered, shaded, camera-facing smoke cards. It is not a fluid simulation or ray-marched participating volume.

The current CPU retained-mesh trail path handles naval profiles in both normal operation and the experimental GPU-smoke configuration. The experimental GPU path remains available for existing air-to-air trails; it does not simultaneously render a duplicate naval trail.

## Bounded rendering work

Trails retain the existing 64-sample history. Close views keep six cards per segment; naval trails reduce to three above 1.8 km and two above 6 km, increasing each card's length to preserve coverage. Vertex/index storage is reused. Existing 35 km naval-trail distance culling remains active.

Launcher clouds use one shared material and one dynamic mesh with a fixed pool of at most 32 clouds, and no per-launch entities or lights. Each cloud uses 12 cards nearby, six beyond 2.5 km, and is culled beyond 12 km. Maximum close cloud geometry is 1,536 vertices / 2,304 indices, reserved at startup. Empty cloud pools skip mesh mutation/upload work. Cloud slots/launch identities reset with the scenario and expire without accumulating mission history. The only added texture is a procedural 128 x 128 RGBA smoke image.

The larger smoke increases transparent pixel coverage in close views. No unchanged-FPS or performance-gain claim is made; the visual diagnostic window is not a throughput benchmark.

## Automated evidence

`cargo test --locked`: **810 passed, zero failures, one opt-in timing diagnostic ignored** (44.23 seconds). New checks verify larger finite smoke bounds, alpha bounds, distance simplification and buffer reuse, one launcher cloud per stable launch identity, pool saturation, expiration and reset. Existing CPU/GPU air-to-air smoke geometry checks and naval/flight regressions remain passing.

Native review uses three bounded snapshots: early ship launch/cloud, ship missile trail, and air-to-ship cruise trail. These checks are distinct from a performance/soak test and the user's final visual review.

## Visual refinement and focused recheck

The initial native views exposed two presentation issues: the enlarged trail was still too transparent over ocean glare, and the initial 20% ship-velocity inheritance left the launcher cloud behind the bridge too quickly. The final profiles use the widths above, denser alpha with a slower naval fade, a lighter shaded texture and full initial ship-velocity inheritance that relaxes toward the air over an estimated three-second timescale. Cloud radius grows from 2.5 m at 1.25 m/s before per-lobe spreading. The final inspection view shows the cloud over the launcher rather than concealed behind the moving bridge.

After the final visual/mesh-storage refinements, `cargo test --locked smoke` passed all **seven focused tests**, with zero failures (0.11 seconds). The earlier full 810-test run is listed separately above. Native still captures verify the launch cloud and both trail profiles at selected times; they do not constitute continuous-motion, post-burnout or throughput validation.

## Native review and delivery

The approved diagnostic window consumed **167.02 seconds** across 11 scripted captures, remaining below three minutes. The restoration handler resumed the prior flight after every invocation. The final ship-launch image visibly shows the cloud spreading over the VLS deck, and the ship/air-to-ship trail views show the wider, denser, billowing profiles. A final packaged-app launch capture also verified the preallocated mesh setup. Latest images and logs are under `tests/naval-volume-smoke/`; its `review.json` records the capture sequence and restoration status.

The existing `Nord Flight.app` was rebuilt with `./build-mac.sh dev` and passed `codesign --verify --deep --strict`. Executable SHA-256: `65b95af5b98840132a57cb9e753ddb527c995ffe81cab04e18c89a6e00850c87`. Quit and reopen the app to load this build; the preserved running flight still uses its original executable.
