# Tu-160 exterior

Original asset: **Lowpoly TU-160**, by **SIpriv** ([author](https://sketchfab.com/S1Priv), [model](https://sketchfab.com/3d-models/lowpoly-tu-160-6cdcef13c1a244978cba2957e0394c88)).

Licensed **Creative Commons Attribution 4.0 International**: <https://creativecommons.org/licenses/by/4.0/>. Attribution, license, and source are embedded in the supplied original GLB metadata. `source/original.glb` is the unchanged user-downloaded file; its SHA-256 is recorded in `inspection.json`.

Nord Flight modifications: metre-scale normalization to 54.1 m length, body-frame conversion, fixed airborne configuration, removal of two loose source assemblies, material batching, roughness adjustment, four original bay panels under independent longitudinal hinges, and three distance LODs. No source animation players are retained. Original paint/UVs are preserved. This is a low-poly visual representation, not a validated Tu-160 flight or weapons simulation.

Length reference: [Bank of Russia Tu-160 technical description](https://cbr.ru/cash_circulation/memorable_coins/coins_base/ShowCoins/?cat_num=5109-0108), 54.1 m length and 35.6–55.7 m span. A single swept-wing airborne pose (about 43.0 m span in the converted asset) remains fixed; no variable-sweep, gear, or control-surface animation is enabled.

Rebuild: `blender -b -t 2 --python-exit-code 1 --python scripts/tu160/build_asset.py` from the project root. Validate: `blender -b -t 2 --python-exit-code 1 --python scripts/tu160/validate_asset.py`. LOD0 preserves the source triangles; medium and distant LODs simplify the fixed hull while preserving full bay-door geometry. See `inspection.json` for counts, bounds, and hinge positions.
