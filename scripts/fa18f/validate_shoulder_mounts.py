"""Actual AIM-120/adapter mesh against every authored gear door across its sweep.
First run the Rust shoulder mesh-export test. Execute with Blender --background.
"""
import runpy,json,math
from pathlib import Path
from mathutils import Vector,Matrix
from mathutils.bvhtree import BVHTree
root=Path(__file__).resolve().parents[2]
g=runpy.run_path(str(root/'scripts/fa18f/validate_gear_sweep.py'))
bpy=g['bpy'];data=g['data'];tree=g['tree'];fail=[]
geometry=json.loads((root/'tests/shoulder-mount-review/geometry.json').read_text())
def source(p):return Vector(((2.56-p[2])*10.,-p[0]*10.,(p[1]-.35)*10.))
for index,store in enumerate(geometry):
 at=store['station']
 for part in ['missile','adapter']:
  mesh=store[part];vertices=[source([v[k]+at[k]for k in range(3)])for v in mesh['vertices']];ix=mesh['indices'];polys=[ix[k:k+3]for k in range(0,len(ix),3)];solid=tree(vertices,polys)
  if part=='adapter':
   body=max((o for o in bpy.data.objects if o.type=='MESH'),key=lambda o:len(o.data.vertices))
   skin=tree([body.matrix_world@v.co for v in body.data.vertices],[tuple(p.vertices)for p in body.data.polygons])
   assert solid.overlap(skin),('adapter must intersect fuselage skin',index)
  for obj in bpy.data.objects:
   if obj.type!='MESH' or not obj.name.startswith('Hornet') or 'Door'not in obj.name or not obj.name.endswith('Mesh'):continue
   name=obj.name[:-4];side=-1 if name.endswith('L')else 1
   hinge=obj.parent.matrix_world.translation.copy();v=[obj.matrix_world@p.co for p in obj.data.vertices];p=[tuple(f.vertices)for f in obj.data.polygons]
   angle=(side if 'Nose' in name else -side)*(1.5 if 'Leg' in name else 1.65)
   for step in range(201):
    q=Matrix.Rotation(angle*step/200.,3,'X')
    if solid.overlap(tree([hinge+q@(pt-hinge)for pt in v],p)):fail.append((index,part,name,step))
  # Check both main and both nose tyre paths as well as the doors.
  for nose in [False,True]:
   for side in [-1,1]:
    name='HornetWheel'+('Nose'if nose else'Main')+('L'if side<0 else'R')+'Mesh'
    _,wv,wp=data(name);original=Vector((61.7,side*2.12,-20.17))if nose else Vector((6.1,side*16.03,-20.3))
    for step in range(101):
     r=step/100.;centre=g['nose_axle'](side,r)if nose else g['axle'](side,r,False)
     angle=0 if nose else side*max(0,min((r-.45)/.55,1))*math.pi/2
     q=Matrix.Rotation(angle,3,'X')
     if solid.overlap(tree([centre+q@(pt-original)for pt in wv],wp)):fail.append((index,part,name,step))
if fail:raise RuntimeError('Store/gear interference: '+repr(fail[:30]))
print('PASS: both actual missiles and adapters clear every door at 201 angles and all four tyres at 101 retraction samples')
