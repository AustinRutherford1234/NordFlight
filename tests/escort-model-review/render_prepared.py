import bpy
from pathlib import Path
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[2];OUT=ROOT/'tests/escort-model-review'
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=str(ROOT/'assets/ships/burke/lod0.gltf'))
bpy.ops.import_scene.gltf(filepath=str(ROOT/'assets/ships/burke/markings128.gltf'))
scene=bpy.context.scene;scene.render.engine='CYCLES';scene.cycles.samples=12;scene.cycles.use_denoising=True
scene.render.resolution_x=1440;scene.render.resolution_y=900;scene.render.resolution_percentage=100
scene.world=bpy.data.worlds.new('World');scene.world.use_nodes=True
scene.world.node_tree.nodes['Background'].inputs['Color'].default_value=(.5,.5,.5,1);scene.world.node_tree.nodes['Background'].inputs['Strength'].default_value=.8
bpy.ops.object.light_add(type='SUN');bpy.context.object.rotation_euler=(.5,-.4,-.5);bpy.context.object.data.energy=3
bpy.ops.object.camera_add();cam=bpy.context.object;scene.camera=cam;cam.data.type='ORTHO';cam.data.clip_end=1000
for name,eye,at,width in [('prepared-starboard',(110,130,75),(0,0,10),170),('prepared-hull-number',(28,64,8),(0,64,6),18),('prepared-stern',(-70,-120,50),(0,-30,8),140)]:
 cam.location=eye;cam.rotation_euler=(Vector(at)-cam.location).to_track_quat('-Z','Y').to_euler();cam.data.ortho_scale=width
 scene.render.filepath=str(OUT/(name+'.png'));bpy.ops.render.render(write_still=True)
