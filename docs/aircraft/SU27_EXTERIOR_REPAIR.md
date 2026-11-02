# Su-27P nose and canopy repair

## Cause and corrected build

The previous bake discarded the source radome and substituted a narrow cone that
did not follow the original forward fuselage. The original is reusable: its
maintenance-pose rotation closes by 125 degrees around the authored object origin.
The restored nose retains the original contour, pitot probe, UVs and paint.

The canopy bake used a guessed common hinge and put the lid too far down/forward.
The revised bake closes the frame and main glazing by 40 degrees around their
actual authored origins, retaining the fixed windshield. Close views were used to
adjust the windshield/lid fit. The seat is lowered 0.20 m in Blender's baked up
axis so its headrest no longer protrudes through the closed glazing.

The source glass alpha was about 0.157. The exported glazing now has a blue-gray
coated surface at alpha 0.64, roughness 0.19 and metallic 0.22, with two-sided
rendering. It is still translucent in the game; opaque Workbench renders inspect
surface continuity independently of transparency. No new moving parts are added.

Critical parts are decimated independently instead of being merged into the hull.
At medium/far distance, a cut from the known-good close forward fuselage replaces
the aggressively collapsed cockpit/nose region. The 40-degree canopy remains
separate at all distances. Source normals, materials and UVs are preserved.

## Detail and limits

Final triangle counts: close 7,894, medium 4,122, far 1,484. The previous counts
were 7,708 / 2,731 / 824. Attempts to retain the smaller medium/far budgets left
broken collars and stretched skin; additional forward geometry was retained to
correct this. The far model is deliberately faceted and has a slightly reduced
wingtip envelope. These changes are not a measured FPS improvement claim.

The source remains the user-provided Rhine_Lab_Muelsyse CC BY 4.0 Su-27P; its
original GLB and source hash are retained. Attribution is in the asset folder.
Reference silhouette: US military air-to-air Su-27 photograph, August 26, 1988,
DefenseImagery DD-ST-88-09314, reproduced at
https://commons.wikimedia.org/wiki/File:Su-27_05.jpg
The model represents the single-seat Flanker-B, not the longer two-seat Su-27UB.

## Verification

- `scripts/su27/build_asset.py` regenerates all three static GLBs in Blender.
- `scripts/su27/validate_asset.py` checks GLB structure, finite positions, indices,
  dimensions, required parts, nose and glazing envelopes, seat containment,
  canopy/windshield proximity, glazing alpha/two-sidedness and triangle budgets.
- `scripts/su27/render_review.py` renders both sides, a whole-aircraft view, nose
  and canopy for each LOD with culling enabled. Local captures: `tests/su27-repair/`.
- All 771 game tests passed in 65.17 seconds. Game flight/weapon behavior was not
  changed; added Rust paths only prepare opt-in frozen inspection cameras.

The approved native review completed in 61.99 seconds of its 120-second window,
with the restoration handler resuming the previous flight. Four frozen in-app
views (right, left, front-quarter nose and close canopy) were inspected; captures
are local at `tests/su27-repair/native/`. The actual game renderer shows the
original paint/panel texture, continuous nose/pitot, closed framing and translucent
coated glazing. These are scripted static inspection views, not manual flight or
a performance benchmark. A fresh launch is required to replace already-loaded
scene handles in the user's older running session.

The current Mac app was rebuilt after the final asset export and its ad-hoc
signature passed `codesign --verify --deep --strict`. Asset hashes were compared
between the workspace and app bundle before delivery.
