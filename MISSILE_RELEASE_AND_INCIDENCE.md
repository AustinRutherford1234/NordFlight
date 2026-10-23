# Missile release and incidence dynamics

Missiles now inherit the launcher's world attitude and angular velocity at
release. World release velocity includes `omega × station_offset` in addition
to aircraft velocity and the existing rail/ejection impulse. Player close-combat,
BVR, allied and hostile launch paths use this behavior. Internal-bay releases
retain the deployed launcher's orientation, including the AIM-9X rail roll.

Previously most missiles initialized their nose along velocity, and each flight
update pulled the nose toward velocity. The aerodynamic controller also applied
normal acceleration directly from fin demand. That produced almost no sustained
angle of attack and prevented proper rotational inheritance.

The controller now demands incidence through a damped, limited body-rate
response. The body rotates independently of the velocity vector. Aerodynamic
normal force depends on body incidence and dynamic pressure; incidence adds
drag. Existing motor, mass, fin travel/rate, structural acceleration limits,
seeker behavior, lofting and active-missile support are retained. AIM-9X thrust
vectoring remains available while powered. This is a tuned game model, not a
calibrated missile inertia/aerodynamic database or full rigid-body torque model.

Rendering preserves the integrated attitude and roll. Smoke emission starts at
the nozzle along the body axis, so a missile flying at incidence does not leave
its emitter displaced along the velocity direction.

## Validation

- **438 regression tests passed**, zero failures, in
  `tests/missile-incidence/tests-delivery.log`.
- New release tests cover both missile types and both F-22 combat launch paths
  on a rotating carrier, including release attitude, world angular rate and
  launch-point velocity.
- A new incidence test checks finite initial attitude response, inherited rates,
  nonzero incidence-dependent aerodynamic load and quaternion/body agreement.
- Existing long-range trajectories, fin authority, density dependence,
  countermeasures, timestep sensitivity and combat regressions pass.
- The deterministic missile engagement study completed; its output is preserved
  in `tests/missile-incidence/engagement-study.log`. This is simulation evidence,
  not real-weapon validation. No new native visual capture is claimed.

The updated native release is packaged in `Nord Flight.app`. Restart a running
instance to load the new executable. Signature and build records are in
`tests/missile-incidence/`.
