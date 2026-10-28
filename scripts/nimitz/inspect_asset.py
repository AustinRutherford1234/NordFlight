"""Inspect Grant's downloaded carrier without altering the source asset."""
import bpy,json
from pathlib import Path
from mathutils import Vector
root=Path(__file__).resolve().parents[2]
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=str(root/'assets/ships/nimitz/source.glb'))
bpy.context.view_layer.update()
points=[o.matrix_world@v.co for o in bpy.data.objects if o.type=='MESH' for v in o.data.vertices]
lo=Vector([min(v[i] for v in points) for i in range(3)])
hi=Vector([max(v[i] for v in points) for i in range(3)])
report={'blender_bounds_min':list(lo),'blender_bounds_max':list(hi),'dimensions':list(hi-lo),'meshes':[],'images':[]}
for o in bpy.data.objects:
 if o.type=='MESH':
  o.data.calc_loop_triangles()
  report['meshes'].append({'name':o.name,'triangles':len(o.data.loop_triangles),'materials':[m.name for m in o.data.materials]})
for img in bpy.data.images:
 report['images'].append({'name':img.name,'size':list(img.size)})
(root/'assets/ships/nimitz/inspection.json').write_text(json.dumps(report,indent=2)+'\n')
print('CARRIER_INSPECTION',json.dumps(report))
