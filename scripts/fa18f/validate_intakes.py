"""Reject blanking surfaces that hide the modeled fan from a forward view.
Run with Blender in background mode and --python-exit-code 1.
"""
import bpy,math
from pathlib import Path
from mathutils import Vector
from mathutils.bvhtree import BVHTree
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=str(Path(__file__).resolve().parents[2]/'assets/aircraft/fa18f/rigged.glb'))
bpy.context.view_layer.update()
verts=[];polygons=[];names=[]
for o in bpy.data.objects:
 if o.type!='MESH' or (o.parent and any(key in o.parent.name for key in ['F18-cockpit','F18-landingOnLight'])):continue
 offset=len(verts);verts.extend(o.matrix_world@v.co for v in o.data.vertices)
 polygons.extend(tuple(offset+i for i in p.vertices) for p in o.data.polygons);names.extend([o.name]*len(o.data.polygons))
tree=BVHTree.FromPolygons(verts,polygons)
for side in (-1,1):
 cam=Vector((40.6,side*9,-3.2));visible=0;hits={}
 for y in range(-4,5):
  for z in range(-4,5):
   if y*y+z*z>16:continue
   target=Vector((-12.35,side*(6+y*.6),1.3+z*.6));direction=(target-cam).normalized()
   hit,_,i,_=tree.ray_cast(cam,direction)
   name=names[i] if hit else 'none';hits[name]=hits.get(name,0)+1
   visible+=int('HornetFan' in name)
 print('Side',side,'visible fan samples',visible,hits)
 if visible<5:raise RuntimeError('Fan still obscured from forward intake view')
