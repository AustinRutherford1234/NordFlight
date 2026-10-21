# AIM-120D radar missile visual refinement

The standard and long-range radar missiles now use detailed AIM-120D-inspired
presentation meshes, on the aircraft and in flight. These are visual models for
the game's existing generic radar/BVR weapons. Aircraft labels such as Meteor
and R-77 retain their existing gameplay meaning; this update does not claim a
separate accurate exterior or flight model for every named weapon.

The F-22 radar weapon label now reads AIM-120D. The body uses a rounded ceramic-colored radome, 64 radial segments, section
joints, identification bands, proximity-sensor windows, a tapered dorsal wiring
cover, swept solid mid-body wings and a recessed exhaust throat. Overall length
is 3.66 m with a roughly 0.178 m body diameter, using the public AMRAAM envelope
in the [USAF fact sheet](https://www.af.mil/About-Us/Fact-Sheets/Display/Article/104576/aim-120-amraam/).
The clipped-wing C/D planform and gray body/ceramic radome treatment use
[Raytheon's public AIM-120D-3 imagery](https://www.rtx.com/raytheon/what-we-do/air/amraam-missile)
as the appearance reference. Panel positions, minor details and surface finishes
are illustrative.

Four solid tail fins hinge independently in an X arrangement. They resolve the
existing rate-limited controller demand into local shaft rotation, with an
estimated 20-degree visual limit. Mid-body wings and nozzle housing stay fixed.
There are no invented radar-missile jet vanes or deflecting nozzle animations.
Both radar profiles use the same AIM-120D clipped-wing exterior. The existing
boost/sustain behavior still distinguishes their exhaust during flight.

The hot core, shock-cell modulation and translucent plume sheath reuse shared
assets from the AIM-9X visual work, sized for the radar missile. Exhaust remains
axial, starts inside the throat, reacts to the actual boost/sustain thrust and
disappears at motor shutdown. Soft smoke uses the shared textured-lobe renderer,
with a broader footprint, simulation-time expansion/fading and persistent buffers.
The old thin debug trail is suppressed for radar missiles.

No motor, guidance, seeker, damage, physics timestep or aircraft performance
parameters were changed. Pause freezes actuation and plume phase; camera movement
still reorients the smoke cards. Existing 64-missile and 48-point trail pool bounds
remain in effect.

Validation includes the full regression suite plus geometry/index/envelope tests,
fixed tail-hinge checks and axial plume/boost-sustain checks. Native close-ups:

```sh
python3 tests/capture_aim9x.py --weapon radar --output tests/radar-visual-dev
```

The shared capture driver inspects body detail, off-axis fin response, exhaust
and smoke using the actual native renderer. These diagnostic setups are visual
validation, not calibrated missile performance measurements.
