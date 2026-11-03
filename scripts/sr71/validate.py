"""Offline triangle-level wheel/closed-skin clearance sweep; no Nord Flight process."""
from pathlib import Path
import sys,json
sys.path.insert(0,str(Path(__file__).parent))
from review import *
from mathutils.bvhtree import BVHTree
rig=load();wheels=[o for o in bpy.data.objects if o.type=='MESH'and o.name.startswith('Torus')];doors=[o for o in bpy.data.objects if o.type=='MESH'and o.name.startswith('Sr71Closure')]
def bvh(o):return BVHTree.FromPolygons([o.matrix_world@v.co for v in o.data.vertices],[list(p.vertices)for p in o.data.polygons])
collisions=[];strokes={l['barrel']:[1e6,0.]for l in rig['links']}
for step in range(41):
 e=step/40.;d=0. if step==0 else 1.;pose(rig,e,d)
 wb=[(o,bvh(o))for o in wheels];db=[(o,bvh(o))for o in doors]
 for w,a in wb:
  for door,b in db:
   if a.overlap(b):collisions.append(dict(extension=e,wheel=w.name,door=door.name))
 for l in rig['links']:
  a=Vector(l['base']);b=G(bpy.data.objects[l['parent']].matrix_world@B(l['local']));distance=(b-a).length
  strokes[l['barrel']][0]=min(strokes[l['barrel']][0],distance);strokes[l['barrel']][1]=max(strokes[l['barrel']][1],distance)
report=dict(samples=41,wheels=len(wheels),door_skins=len(doors),wheel_skin_intersections=collisions,actuator_attachment_distances=strokes)
(ROOT/'tests/sr71-rig/clearance.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
if collisions:raise RuntimeError('Wheel/skin interference detected; inspect clearance.json')
