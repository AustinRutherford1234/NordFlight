"""Standalone inspection only; never changes Nord Flight assets or game code."""
import bpy,json,hashlib
from pathlib import Path
from mathutils import Vector
root=Path(__file__).resolve().parents[2];out=root/'tests/escort-model-review'
source=root/'assets/ships/burke/source.glb'
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=str(source));bpy.context.view_layer.update()
points=[o.matrix_world@v.co for o in bpy.data.objects if o.type=='MESH' for v in o.data.vertices]
lo=Vector([min(v[i] for v in points) for i in range(3)]);hi=Vector([max(v[i] for v in points) for i in range(3)])
rows=[]
for o in bpy.data.objects:
 if o.type=='MESH':
  o.data.calc_loop_triangles();rows.append({'name':o.name,'triangles':len(o.data.loop_triangles),'vertices':len(o.data.vertices),'materials':[m.name for m in o.data.materials]})
report={'source':str(source),'sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'min':list(lo),'max':list(hi),'dimensions':list(hi-lo),'triangles':sum(r['triangles'] for r in rows),'meshes':rows,'textures':[{'name':i.name,'size':list(i.size)} for i in bpy.data.images],'emissive_materials':[]}
for m in bpy.data.materials:
 if m.use_nodes:
  for node in m.node_tree.nodes:
   if node.type=='BSDF_PRINCIPLED':report['emissive_materials'].append({'material':m.name,'strength':node.inputs['Emission Strength'].default_value})
(out/'inspection.json').write_text(json.dumps(report,indent=2)+'\n');print('REVIEW',json.dumps(report))
bpy.ops.wm.save_as_mainfile(filepath=str(out/'inspection.blend'))
