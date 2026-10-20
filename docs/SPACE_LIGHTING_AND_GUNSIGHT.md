# Space lighting and cockpit gun director — October 17, 2026

## Space Sun

The solar mesh was hidden unless an obsolete camera index was selected. Its
real-distance position was also beyond the ordinary flight camera's far plane.
The unlit StandardMaterial path bypassed its emissive setting, leaving only a
dim base colour even when the mesh was displayed.

`src/celestial.rs` supplies a render-only solar proxy on the actual eye-to-Sun
ray. Distance and radius scale together, retaining apparent angular size inside
the camera range. It stays behind nearby Earth/Moon geometry, allowing depth
occlusion rather than drawing the Sun through a planet. Physical Sun position,
directional lighting inputs and orbital calculations are unchanged. The mesh
becomes visible in vacuum flight and maps, uses exposed emissive radiance and
natural bloom, and cannot cast a shadow on its own directional sunlight. Its
fog flag is disabled; atmospheric rendering supplies the low-altitude disk.

Native checks include a Sun-facing external flight view and its night-side
counterpart. The first shows the bright disk/glare; the second shows Earth
covering the disk. Diagnostic logs record proxy pose, visibility and camera
alignment. The cockpit panel can physically obstruct the disk, so the external
view is the unobstructed visibility check.

![Visible Sun in orbital flight](../tests/space-lighting/release/earth-flight-220-sun-external.png)

## Sweden and the Nordic region

The old high-resolution Nordic image occupied a separate alpha-blended globe
patch. It could composite above the cloud shell, leaving Sweden/Scandinavia
cloud-free while adjacent parts of the globe remained cloudy. It also replaced
the broad albedo colours with a different regional mosaic, exaggerating the
boundary's appearance.

The shipping GPU renderer now samples Nordic detail in the opaque globe's own
fragment shader. It creates no second regional mesh or transparent regional
draw. Global mosaic colour/brightness remains the broad albedo; bounded local
detail from the original full-resolution Nordic image is feathered into it.
The same interpolated solar shade then applies everywhere. Clouds and haze
remain separate layers above the entire globe. The global/Nordic source images
and globe geometry are retained. The developer-only `NORD_EARTH_CPU` historical
reference still uses its original overlay; it is not the shipping path.

Matched native orbital captures show the restored coverage:

![Before: regional patch covers clouds](../tests/space-lighting/before/earth-flight-220.png)

![After: continuous globe and cloud coverage](../tests/space-lighting/release/earth-flight-220.png)

Additional captures check a Nordic-centered map and the wider geography view.
Screenshots establish appearance; their displayed FPS includes capture overhead
and is not a performance benchmark.

## Cockpit two-ring gun director

The cockpit funnel is replaced with the requested F/A-18-inspired presentation:

- Small dim ring: current gun bore direction, projected at selected-target range
  when a valid solution is available, otherwise at the existing 2 km reference.
- Larger bright ring and central dot: predicted round arrival relative to the
  selected target. **Place this ring over the actual target aircraft**, then
  fire with **U**. It includes gravity, inherited player velocity, constant
  target velocity and the actual game's 4,250 m/s muzzle speed.
- A perimeter tick indicates target range. Normal cockpit view includes range
  and time of flight; simplified view retains both rings. `SHOOT` requires an
  aligned solution and an available powered gun with ammunition.
- Rearward, destroyed, out-of-range and unreachable targets have no computed
  impact ring. The bore reference remains available. External aiming symbology
  and the external funnel retain their existing behavior.

The director computes a centreline round fired now at the solved intercept
time. Subtracting target motion and translating the predicted miss back to the
target's current position makes the ring-on-aircraft aiming convention work.
It assumes constant target velocity and does not promise a hit against a target
that maneuvers during the round's flight. This is the requested game gunsight,
not a calibrated reproduction of a Hornet fire-control computer. The user's
requested two-circle behavior defines this implementation.

![Cockpit director](../tests/space-lighting/release/air-hud-cockpit.png)

## Validation and delivery

All **304 application tests passed**. New regressions cover solar projection
direction/size/far-plane clearance, planet-occlusion ordering, single opaque
surface conversion, director lead/drop/rear-target rejection, and a numerically
integrated round intercept when the director is aligned. Existing cannon
availability/audio, flight, combat, terrain, lunar and rendering tests also pass.
Native captures inspect normal/simplified cockpit views, orbital Sweden, the
Nordic/wider maps, the visible Sun and night-side occlusion. No aircraft,
missile or orbit dynamics were changed.

Final release packaging and native capture verification are recorded in
`tests/space-lighting/delivery.json`. The previously installed executable is
preserved at `tests/space-lighting/baseline`. Existing running flights are not
discarded during installation; restart that app instance to load the new build.

The final optimized release compiled successfully in 5 minutes 28 seconds,
was stripped/ad-hoc signed, and passed strict/deep signature verification.
All eight native checks of the packaged executable completed successfully:
orbital Sun, night-side occlusion, orbital Sweden, Nordic map, 99/101 km views,
and normal/simplified cockpit directors. Images were inspected after capture.
Installed SHA256:
`60c63aa96ea2d0143ccd38ed7c1a35c89076fd2da2bf622296adf7099c7b3dac`.
Build UUID: `3ECB0BB3-3DFE-344F-A3F9-8F8E0E8E04AE` (arm64).
