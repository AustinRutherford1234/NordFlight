# Arleigh Burke Flight III model

Base model: **USS Jack H. Lucas DDG-125**, by **SB-129** (`hrd4588`).

- Source: https://sketchfab.com/3d-models/uss-jack-h-lucas-ddg-125-1c9229b49915454c8ff22605f8cc7b66
- Author: https://sketchfab.com/hrd4588
- License: **Creative Commons Attribution 4.0 International (CC BY 4.0)** — https://creativecommons.org/licenses/by/4.0/

`source.glb` is the unmodified free 2K GLB downloaded through Sketchfab's download control. Its embedded metadata identifies the source, author and CC-BY-4.0. Preserve this attribution/license/source link and indicate changes when redistributing the derivatives. No endorsement by the original creator or the U.S. Navy is implied.

`lod*.gltf`, their binary buffers and shared `textures/` are modified derivatives prepared for Nord Flight. Changes include metre alignment, a waterline origin, helicopter removal, normal/material refinement, geometry for handrails, deck fittings/tie-downs/static sailors, bridge window inserts, navigation/deck lamps, per-ship paint/number/name/aviation-deck markings, and four levels of detail. `collision.bin` is a derived skin with visual paint/crew overlays excluded. The asset retains static source weapon fittings. Carrier Defense now attaches representative radar/SAM/CIWS gameplay through `src/combat/naval/`; those estimated systems and added mounts are project changes, not capabilities supplied or endorsed by the source model author.

The model is used for representative **DDG 125 / USS Jack H. Lucas** and **DDG 128 / USS Ted Stevens** instances. Small fittings and waterline/lighting setup are simulation/art estimates; these are not certified replicas of a dated operational configuration. Shared mesh/texture assets are used across both ships; individual markings and mooring-coil arrangements distinguish them.

Reproduce derivatives using Blender 5.x:

```
blender --background --python-exit-code 1 --python scripts/burke/build_asset.py
```

`inspection.json` records source/collider hashes, geometry counts, texture sharing, light placements, and the removed helicopter components. The editable intermediate Blender project is saved locally in `tests/escort-model-review/prepared.blend`; runtime uses only the exported glTF assets.
