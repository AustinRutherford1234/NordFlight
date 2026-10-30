"""Render the downloaded candidate without integrating it into the game."""
import bpy,math
from mathutils import Vector
from pathlib import Path
root=Path(__file__).resolve().parents[2];out=root/'tests/escort-model-review'
bpy.ops.wm.open_mainfile(filepath=str(out/'inspection.blend'))
scene=bpy.context.scene
scene.render.engine='CYCLES';scene.cycles.samples=12
scene.cycles.use_denoising=True;scene.render.resolution_x=1440;scene.render.resolution_y=900;scene.render.resolution_percentage=100
scene.render.image_settings.file_format='PNG'
scene.world=bpy.data.worlds.new('Review World');scene.world.use_nodes=True
scene.world.node_tree.nodes['Background'].inputs['Color'].default_value=(0.5,0.5,0.5,1.)
scene.world.node_tree.nodes['Background'].inputs['Strength'].default_value=0.8
scene.view_settings.view_transform='AgX';scene.view_settings.exposure=0.8
objects=[o for o in scene.objects if o.type=='MESH']
pts=[o.matrix_world@v.co for o in objects for v in o.data.vertices]
lo=Vector([min(v[i] for v in pts) for i in range(3)]);hi=Vector([max(v[i] for v in pts) for i in range(3)])
center=(lo+hi)/2;length=hi.x-lo.x
# Normalize only the standalone review scene; the untouched download remains source.
scale=155.3/length
bpy.ops.object.select_all(action='DESELECT')
for o in objects:
 matrix=o.matrix_world.copy();o.parent=None;o.matrix_world=matrix
 o.location=(o.location-center)*scale;o.scale*=scale
bpy.context.view_layer.update()
bpy.ops.object.light_add(type='SUN',location=(0,0,100));sun=bpy.context.object;sun.rotation_euler=(.5,-.4,-.5);sun.data.energy=3
bpy.ops.object.camera_add();cam=bpy.context.object;scene.camera=cam;cam.data.lens=45;cam.data.clip_end=2000
views=[('bow-quarter',(-125,-115,67),(0,0,0),170),('stern-quarter',(120,100,68),(0,0,0),170),('port-profile',(0,-170,10),(0,0,0),174),('deck-detail',(-12,-42,48),(-12,0,6),75)]
for name,eye,target,ortho in views:
 cam.location=eye;direction=Vector(target)-cam.location;cam.rotation_euler=direction.to_track_quat('-Z','Y').to_euler()
 cam.data.type='ORTHO';cam.data.ortho_scale=ortho
 scene.render.filepath=str(out/(name+'.png'));bpy.ops.render.render(write_still=True)
