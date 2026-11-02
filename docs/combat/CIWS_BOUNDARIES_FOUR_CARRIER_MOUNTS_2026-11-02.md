# Friendly-ship CIWS boundaries and four carrier mounts

## Targeting boundary

CIWS excludes targets inside any friendly ship's imported collision-mesh bounds expanded by 25 m in each direction. This intentionally simple ship-local box follows the moving, listed, trimmed or sinking hull through the same ship transforms as collision and rendering. It is not an infinite-height cylinder or a fleet-wide exclusion sphere.

The test applies during candidate selection, so an invalid nearby missile cannot hide an eligible threat farther away. It also applies to the predicted intercept point at the projectile's estimated arrival time. Existing radar detection, range, elevation, traverse and ship-obstruction checks remain active. The boundary inhibits new fire; existing ballistic projectiles are not recalled or repositioned.

## Carrier installation

The carrier previously had one simulated mount. The requested four-mount carrier configuration now uses four independent aim/elevation states, firing schedules and finite magazines. Each keeps the existing 3,100-round game allocation, giving the carrier 12,400 total; each escort retains its existing one simulated mount and 3,100 rounds. These are simulation supply allocations, not sourced real-ship magazine specifications.

All four reuse the existing CIWS geometry, six rotating barrels, muzzle flashes and ballistic tracer presentation. Geometry and effects use the same mount identities/attachment coordinates as firing. Carrier mounts cover their outboard hemispheres; candidate selection rejects threats outside those arcs before choosing the closest target. Predicted lead remains subject to the arc check. Static mount geometry is also visible in ordinary carrier scenarios outside Carrier Defense.

| Carrier mount | Local x (m) | Local y (m) | Local z (m) |
|---|---:|---:|---:|
| Port forward | -20 | 24.15 | -80 |
| Starboard forward | 22 | 24.15 | -80 |
| Port aft | -32 | 24.15 | 110 |
| Starboard aft | 32 | 24.15 | 110 |

These are mesh-fitted game attachment estimates, not a surveyed installation for a named hull/refit. Downward collision queries verify that every pedestal foot is supported by the imported carrier deck within 0.1 m. The four-mount configuration is the requested game configuration; real Nimitz-class armament varies by hull and refit. The [Navy Phalanx fact file](https://www.navy.mil/DesktopModules/ArticleCS/Print.aspx%3FPortalId%3D1%26ModuleId%3D724%26Article%3D2167831) describes its close-in defense role and 4,500-round/min aircraft/anti-ship-missile firing rate. The existing implementation retains 75 rounds/s in three-round packets at 25 Hz per active mount.

## Bounds and optimization

The fixed ballistic-packet pool is increased from 256 to 512 for six total simulated fleet mounts. Six mounts firing 25 packets/s for the full three-second packet lifetime require at most 450 packets. Slots are reused; the cap remains finite. Ship-boundary queries use cached imported mesh bounds and the shared ship frame cache. Cheap range/arc rejection precedes boundary and detailed visibility checks. Models/materials are shared and muzzle flashes remain preallocated; this change adds no point lights or per-shot entities. No measured FPS improvement or unchanged-FPS claim is made.

## Validation

New tests cover all three moving/damaged ship boundaries and the 25 m edge, rejection of an in-hull target without masking a valid target, all four carrier guns firing with separate aim and ammunition, bounded packets, aggregate magazine accounting and pedestal support against the installed collision mesh. Existing CIWS cadence/interception, naval missile, damage, flight and reset tests remain in the full suite.

Automated simulation/geometry evidence is separate from native visual review. This task leaves the running user flight untouched.

## Recorded results

Final `cargo test --locked`: **808 passed, zero failures, one opt-in timing diagnostic ignored**, in 20.72 seconds. All four boundary/mount regressions passed, including each carrier pedestal's support against the imported mesh. The previous single-mount 75 rounds/s cadence tests at 60/120/240 Hz also remain passing.

## Delivery

Rebuilt the existing `Nord Flight.app` with `./build-mac.sh dev`; it passed `codesign --verify --deep --strict`. Executable SHA-256: `2d3511e1e1e8d851cbfd7f3e317d13759390fff11c808229238189365aac440b`. Quit and reopen the app to load the update. The current user flight was not interrupted, and no new native visual/performance diagnostic was run.
