# F-22 elastic surfaces

The F-22 has a reduced-order elastic model for its wings, vertical fins,
stabilators and moving aerodynamic panels. It is enabled only for the F-22;
the structural solver and state are separate from the aircraft-specific
configuration so that another aircraft can supply its own geometry and modes.
All dimensions, masses, frequencies, damping ratios and load-allocation
fractions in this model are simulation estimates, not F-22 engineering data.
The parent wing bending mode is tuned stiffer (6 Hz) than the prior 4.5 Hz
estimate, keeping the nominal tip slope near 10 degrees at the modeled 15-G
break load. Vertical fins and tail planes are also stiffer, and moving panels
have higher bending and torsion modes to keep their load-driven flex subtle.
Deflection scales smoothly with load below failure.

The solver represents bending and twist with damped modes. The modal update is
the exact constant-load solution for each actual simulation step; aerodynamic
loads and rigid-body motion are updated between steps. To resolve the estimated
fast control-surface modes, the F-22 advances its coupled flight and structural
physics in substeps no larger than 1/240 second. Forces include an
estimated share of the existing whole-aircraft aerodynamic lift and side force,
the existing incremental control-panel forces, and distributed inertial loads
from specific acceleration and aircraft rotation. The elastic load estimates
are not added to the rigid-body force a second time. Elastic shape and velocity
also affect the estimated aggregate aerodynamic incidence and panel loads.
Fly-by-wire control probes use the same frozen elastic state and do not advance
the structural solver.

The aircraft-level polar does not identify an actual pressure distribution.
The allocation assigns estimated fractions and locations to the lifting
surfaces; remaining aggregate force and moment continue to act in the existing
rigid-body model. In particular, baseline moment and the real wing, fuselage
and tail load split are not reconstructed. Hinge and parent-elastic angular
acceleration are omitted from the independent panel inertia estimate. Parent and panel modes are a
low-cost approximation rather than a coupled finite-element model; all-moving
stabilator support and panel compliance are separate approximate modes, not
measured structural components. Existing overload and damage rules remain in
charge of failure. No fatigue, plastic deformation, validated flutter boundary,
or certified structural limit is predicted.

## Estimated wing breaking strength

Each wing has a separate signed aerodynamic root bending stress, available in
the F10 aerodynamic display. The simulation sums estimated aerodynamic force
moments about that wing root and divides by an effective section modulus.
Thrust, gravity, landing gear and inertial-relief forces do not enter this
failure calculation. An intact wing fails immediately above an assumed 300 MPa
in either bending direction. The assumed section modulus is approximately
0.01191 m³: it is calibrated using the default 25,700.74 kg F-22 loadout,
equal left/right loading, neutral panels, and a synthetic sea-level 15-G
body-normal aerodynamic force. Air density affects the force; the section
strength remains fixed at other altitudes and aircraft masses. Different load
distributions and asymmetry mean that actual failure need not occur at exactly
15 indicated G. This stress is an estimated bending stress, not a complete
stress tensor or measured F-22 material strength.

Existing wing health scales the effective section modulus and hence remaining
strength. A broken wing loses its fixed skin, collision pieces and attached
leading flap, flaperon and aileron; the stabilator remains. A simplified debris
fragment is emitted in normal flight and combat. The other wing can fail
independently. The existing aircraft-wide overload, pressure and thermal damage
remain active. Their positive/negative G damage rates are reduced tenfold so a
short high-G excursion can reach the wing limit; sustained overload can still
destroy the aircraft first. Assisted controls continue to limit commanded G.

The F-22 procedural surfaces use unique deformable meshes. Wings, fins,
controls, borders and attached lights follow the elastic state; segmented wing
collision pieces and locally posed panel/fin hulls approximate the same shape.
The collision skin remains coarser than the rendered mesh, and the existing
missile-proximity/damage distance queries still use the undeformed skin. The solver state is
per aircraft and is reset on aircraft selection; other aircraft retain their
existing flight and visual paths.

The following deliberately exaggerated posed view helps inspect continuity; it
is a rendering diagnostic, not a prediction of aircraft deflection:

```sh
NORD_AIRCRAFT=f22 NORD_FLEET_CAPTURE=flex-test \
  target/debug/gripen-flight --visual-check engine-flexcheck
```

Verification should include the structural and aircraft tests, the F-22 flight
and fleet studies, and native views. Test results establish numerical and game
behavior; real-aircraft fidelity would require suitable structural and
distributed aerodynamic reference data.
