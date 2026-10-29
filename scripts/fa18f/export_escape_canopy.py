"""Extract the exact moving canopy derivative for the escape visual."""
from pathlib import Path
import bpy
root=Path(__file__).resolve().parents[2]
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=str(root/'assets/aircraft/fa18f/rigged.glb'))
bpy.ops.object.select_all(action='DESELECT')
canopy=bpy.data.objects['HornetCanopyPivot']
for obj in [canopy,*canopy.children_recursive]:obj.select_set(True)
frame=bpy.data.objects.get('HornetCanopyFrame')
if frame:
 for obj in [frame,*frame.children_recursive]:obj.select_set(True)
bpy.ops.export_scene.gltf(filepath=str(root/'assets/aircraft/fa18f/escape_canopy.glb'),export_format='GLB',use_selection=True,export_animations=False)
