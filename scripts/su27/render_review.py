"""Cull-enabled fitted-part review from both sides and across all runtime LODs."""
import bpy
from pathlib import Path
from mathutils import Vector
root=Path(__file__).resolve().parents[2];out=root/'tests/su27-repair';out.mkdir(parents=True,exist_ok=True)
for i in range(3):
 bpy.ops.wm.read_factory_settings(use_empty=True);bpy.ops.import_scene.gltf(filepath=str(root/f'assets/aircraft/su27/lod{i}.glb'))
 # Workbench opaque inspection also exposes missing backfaces in transparent parts.
 for m in bpy.data.materials:
  if m.name.startswith('Su27Glazing'):m.diffuse_color=(.08,.23,.30,1.)
 scene=bpy.context.scene;scene.render.engine='BLENDER_WORKBENCH';scene.display.shading.light='STUDIO';scene.display.shading.color_type='MATERIAL';scene.display.shading.show_shadows=True;scene.display.shading.show_cavity=True;scene.display.shading.show_backface_culling=True;scene.display.shading.background_type='WORLD';scene.world=bpy.data.worlds.new('ReviewWorld');scene.world.color=(.13,.16,.2);scene.render.resolution_x=1200;scene.render.resolution_y=800;scene.render.resolution_percentage=100
 views=[('whole',(19,27,13),(0,1,0),27),('side',(15,6,.5),(0,6,.2),10),('opposite',(-15,6,.5),(0,6,.2),10),('nose',(7,19,4),(0,8,0),10),('canopy',(5,9,4),(0,5.2,.5),5.7)]
 for tag,p,target,scale in views:
  bpy.ops.object.camera_add(location=p);camera=bpy.context.object;camera.rotation_euler=(Vector(target)-camera.location).to_track_quat('-Z','Y').to_euler();camera.data.type='ORTHO';camera.data.ortho_scale=scale;scene.camera=camera;scene.render.filepath=str(out/f'lod{i}-{tag}.png');bpy.ops.render.render(write_still=True)
