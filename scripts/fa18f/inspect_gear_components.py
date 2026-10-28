"""Report disconnected source gear pieces and their world-space bounds.

Run with Blender: blender --background --python scripts/fa18f/inspect_gear_components.py
"""
from pathlib import Path
import bpy
bpy.ops.wm.read_factory_settings(use_empty=True)
source = Path(__file__).resolve().parents[2] / 'assets/aircraft/fa18f/source.glb'
bpy.ops.import_scene.gltf(filepath=str(source))
import bmesh
for root in ['F18-landingOn_5','F18-landingOff_4']:
 obj=next(o for o in bpy.data.objects if o.type=='MESH' and o.parent and o.parent.name==root)
 bm=bmesh.new();bm.from_mesh(obj.data); unseen=set(bm.verts); parts=[]
 while unseen:
  v=unseen.pop(); stack=[v]; group=[v]
  while stack:
   for e in stack.pop().link_edges:
    for w in e.verts:
     if w in unseen: unseen.remove(w); stack.append(w);group.append(w)
  p=[obj.matrix_world@v.co for v in group];lo=[min(v[i] for v in p) for i in range(3)];hi=[max(v[i] for v in p) for i in range(3)]
  if len(group)>15:parts.append((len(group),tuple(round(x,1) for x in lo),tuple(round(x,1) for x in hi)))
 print(root, sorted(parts,reverse=True)[:35])
