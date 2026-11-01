# Imported Su-27P fighter exterior

The user-supplied larger GLB is the textured Su-27P Flanker-B by Rhine_Lab_Muelsyse, CC BY 4.0. Source, attribution, checksum and reproducible Blender conversion are included under assets/aircraft/su27 and scripts/su27. Both downloads contain the same 39 source meshes; the larger copy retains the higher-resolution main texture.

All live combat fighter slots now use the imported exterior in target practice, dogfight, BVR and Carrier Defense. Naval bomber slots retain the Tu-160. The replacement is a visual change: AI, flight dynamics, weapon inventories, seeker logic and collision/damage physics are unchanged. BVR friendly aircraft share the same exterior as the existing common combat-model path did previously. Legacy procedural combat roots remain hidden; the old naval fighter meshes are removed.

Three shared scene assets now provide 7,708 / 2,731 / 824 triangles (previously 28,374 / 9,930 / 2,837). Switching distances are 800 and 3,000 metres from the camera with 10% hysteresis. Only one LOD per fighter is visible. Source textures/UVs are retained, with no runtime model animation player. The exterior is a static airborne pose; this change does not add animated gear or control surfaces to enemies.

Conversion uses 21.9 m length, source -Y nose to simulator -Z nose, and +Z source up to simulator +Y. The source radome was raised: an original closed ogive replaces that portion, and the canopy is rotated into a closed airborne pose. Gear, dampers, open gear hatches, bombs, loose source stores and pylons are removed. These are visual repairs rather than a claim of CAD-accurate reconstruction. The original GLB is retained for future refinements.

Original import validation: 736 automated tests passed (including new fighter/bomber filtering and LOD hysteresis tests). The standalone asset validator checks all GLB headers, finite bounds, physical length/span, decreasing triangle counts, absence of source animation clips and attribution. Native fighter-follow captures were inspected for textures, orientation and assembly repairs. This used the remaining previously approved bounded review window and restored the user's running session. Snapshot FPS is not a before/after performance benchmark; no measured performance gain is claimed. Mac app rebuilt and strict/deep ad-hoc signature checked.

## Lower-poly mesh and enemy hit-display repair (2026-11-01)

The replacement nose previously had inward-facing side polygons and an incorrect vertical fit. Its winding is corrected and its base is fitted to the imported fuselage opening. A separate procedural radome is generated for each LOD (24/16/8 circumferential segments), preserving the nose independently of fuselage decimation. The source textures and CC BY attribution remain intact. Close-mesh triangle count is reduced by 72.8%; no FPS gain or texture-memory reduction is claimed.

Enemy hit snapshots now preserve bomber identity even after target selection changes or destruction. Fighters use the Su-27 damage silhouette and control-surface names; Tu-160 bombers use a swept-wing silhouette with paired nacelles, wing/engine/subsystem damage colours, and bomber-specific status text. Bomber hull health is normalized to its existing two-point health capacity. Existing aggregate engine-health channels are propagated to both displayed engine groups, and the bomber tail reads the stabilator damage channels. This changes presentation only; hit physics and durability are unchanged.

Offline review: inspected Blender workbench renders of close/far LODs with back-face culling enabled, plus actual HUD raster output for both damage silhouettes. The asset validator enforces mesh budgets and outward radome winding in every LOD, alongside the original envelope/licensing checks. No live-flight interruption was used for this repair. Reproduce model views with `Blender -b --python scripts/su27/render_review.py`; local images are in `tests/enemy-model-review/`.

Repair validation: all 753 game tests passed in 84.45 seconds, including nine hit-display tests. All three GLBs passed geometry-envelope, triangle-budget and outward-radome-face checks. Current offline preview images are `su27-lod0.png`, `su27-lod2.png`, `fighter-damage.png`, and `bomber-damage.png` in the local review folder.

After the final engine/tail display-channel correction, all nine focused effects/display tests passed again (0.16 seconds).

The existing Mac app was rebuilt after the final correction and passed strict/deep ad-hoc signature verification.
