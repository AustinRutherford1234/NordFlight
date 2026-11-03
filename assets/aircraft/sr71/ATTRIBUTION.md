# Sr71 — manilov.ap

Original model: https://sketchfab.com/3d-models/sr71-908985d8ec544638bcd661bc315597ad
Author: manilov.ap — https://sketchfab.com/manilov.ap
License: Creative Commons Attribution 4.0 International (CC BY 4.0)
https://creativecommons.org/licenses/by/4.0/

User supplied source.glb, downloaded from the above model page. Source preserved unchanged.
Nord Flight modifications: aligned longitudinal axis to game forward (-Z), centered longitudinal/lateral bounds, uniform scale to 32.73 m length, aligned tyre bottom 2 m below aircraft origin, combined static objects, consolidated duplicate materials, re-exported as static.glb without animations. Textures and triangle detail preserved. Work does not imply endorsement by the original author.

Preparation: Blender 5.2, scripts/sr71/prepare.py. Measurements in preparation.json. Shared flight/contact physics remain provisional; this model does not add animation or aerodynamic validation.

Animated derivative: rigged.glb and sr71-editable.blend preserve source surfaces and textures, with separated/reparented gear, wheels, fins, elevons, inlet cones, nozzles and canopy shells. Added source-mesh-derived ram/hinge/stiffener details, recessed well liners, and seven small fitted closure skins because the original model has open wheel wells. Hinges, rates, clearances and spike/nozzle schedules are visual engineering estimates. Runtime actuator code and scripts are in src/aircraft/sr71.rs, src/aircraft/visual/sr71.rs and scripts/sr71/.
