"""Headless geometry checks of all shipped Tu-160 LODs, including hinge sweeps."""
from pathlib import Path
import bpy,math,json
from mathutils import Quaternion,Vector
ROOT=Path(__file__).resolve().parents[2];HERE=ROOT/'assets/aircraft/tu160';results=[]
for level in range(3):
 bpy.ops.wm.read_factory_settings(use_empty=True);bpy.ops.import_scene.gltf(filepath=str(HERE/f'lod{level}.glb'));bpy.context.view_layer.update()
 doors=[bpy.data.objects.get(f'BomberBay{i}')for i in range(4)];assert all(doors)
 assert not any(o.animation_data for o in bpy.data.objects),'Unexpected runtime animation'
 meshes=[o for o in bpy.data.objects if o.type=='MESH'];points=[o.matrix_world@v.co for o in meshes for v in o.data.vertices]
 assert all(math.isfinite(x)for p in points for x in p)
 length=max(p.y for p in points)-min(p.y for p in points);assert abs(length-54.1)<(.01 if level==0 else .25),length
 count=0
 for o in meshes:o.data.calc_loop_triangles();count+=len(o.data.loop_triangles)
 for i,d in enumerate(doors):
  child=[o for o in meshes if o.parent==d];assert child
  # Material grouping may add one object transform, but export should retain direct mesh child.
  closed=[o.matrix_world@v.co for o in child for v in o.data.vertices];z0=sum(p.z for p in closed)/len(closed)
  for amount in [0.,.25,.5,.75,1.]:
   d.rotation_mode='QUATERNION';d.rotation_quaternion=Quaternion((0,1,0),(1 if i%2==0 else -1)*math.pi/2*amount);bpy.context.view_layer.update()
   opened=[o.matrix_world@v.co for o in child for v in o.data.vertices]
   assert all(math.isfinite(x)for p in opened for x in p)
   if amount==1:assert sum(p.z for p in opened)/len(opened)<z0-.5,'Door opens upward'
  d.rotation_quaternion=Quaternion();bpy.context.view_layer.update()
 results.append({'level':level,'length_m':length,'triangles':count,'door_sweep':'pass'})
print(json.dumps(results,indent=2))
