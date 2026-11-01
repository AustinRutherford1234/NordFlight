import bpy, math
from pathlib import Path
from mathutils import Vector
root=Path(__file__).resolve().parents[2];out=root/'tests/enemy-model-review';out.mkdir(parents=True,exist_ok=True)
for i in [0,2]:
 bpy.ops.wm.read_factory_settings(use_empty=True);bpy.ops.import_scene.gltf(filepath=str(root/f'assets/aircraft/su27/lod{i}.glb'))
 bpy.ops.object.camera_add(location=(19,27,13));camera=bpy.context.object;camera.rotation_euler=(Vector((0,1,0))-camera.location).to_track_quat('-Z','Y').to_euler();camera.data.type='ORTHO';camera.data.ortho_scale=29
 scene=bpy.context.scene;scene.camera=camera;scene.render.engine='BLENDER_WORKBENCH';scene.display.shading.light='STUDIO';scene.display.shading.color_type='MATERIAL';scene.display.shading.show_shadows=True;scene.display.shading.show_cavity=True;scene.display.shading.show_backface_culling=True;scene.render.resolution_x=1000;scene.render.resolution_y=800;scene.render.resolution_percentage=100;scene.render.filepath=str(out/f'su27-lod{i}.png');bpy.ops.render.render(write_still=True)
