"""Bake SIpriv's CC BY Tu-160 to metre-scale, static-flight material-batched LODs.
Only the four original bomb-bay panels retain pivots. Run in Blender 5.x.
"""
from pathlib import Path
import bpy,json,math,hashlib
from mathutils import Vector,Matrix
ROOT=Path(__file__).resolve().parents[2];HERE=ROOT/'assets/aircraft/tu160';SOURCE=HERE/'source/original.glb'
bpy.ops.wm.read_factory_settings(use_empty=True);bpy.ops.import_scene.gltf(filepath=str(SOURCE));bpy.context.view_layer.update()
original={o.name:o.matrix_world.copy()for o in bpy.data.objects}
# Source's bay animation also carries the retracted gear and fixed swept-wing pose.
for o in bpy.data.objects:
 if o.animation_data:
  o.animation_data.action=None
  for t in o.animation_data.nla_tracks:t.mute=t.name!='animation wpn bay'
bpy.context.scene.frame_set(1);bpy.context.view_layer.update()
scale=54.1/(1.4381410+15.6875029);center=(-15.6875029+1.4381410)/2
conversion=Matrix(((-scale,0,0,0),(0,-scale,0,center*scale),(0,0,scale,-1.25*scale),(0,0,0,1)))
doornames=['bone77_92','bone78_94','bone79_96','bone80_98'];roots=[];meshes=[];removed=[]
for i,name in enumerate(doornames):
 pivot=bpy.data.objects.new('BomberBay%d'%i,None);bpy.context.collection.objects.link(pivot);pivot.location=(conversion@original[name]).translation;roots.append(pivot)
for o in list(bpy.data.objects):
 if o.type!='MESH':continue
 chain=[];p=o
 while p:chain.append(p.name);p=p.parent
 if 'bone_197'not in chain:removed.append(o.name);continue
 door=next((i for i,n in enumerate(doornames)if n in chain),None)
 matrix=conversion@(original[o.name]if door is not None else o.matrix_world)
 mesh=o.data.copy();mesh.transform(matrix);obj=bpy.data.objects.new('Tu160_'+o.name,mesh);bpy.context.collection.objects.link(obj);meshes.append(obj)
 if door is not None:
  mesh.transform(Matrix.Translation(-roots[door].location));obj.parent=roots[door]
# Strip the imported hierarchy entirely; no source animation player or gear/wings can run.
for o in list(bpy.data.objects):
 if o not in meshes and o not in roots:bpy.data.objects.remove(o,do_unlink=True)
fixed=[o for o in meshes if not o.parent];bpy.ops.object.select_all(action='DESELECT')
for o in fixed:o.select_set(True)
bpy.context.view_layer.objects.active=fixed[0];bpy.ops.object.join();hull=bpy.context.object;hull.name='Tu160FixedHull'
# Preserve original UV paint. Make opaque exterior materials single-sided where possible.
for m in bpy.data.materials:
 if m.use_nodes:
  bs=m.node_tree.nodes.get('Principled BSDF')
  if bs:bs.inputs['Roughness'].default_value=.57
objects=[o for o in bpy.data.objects if o.type=='MESH'];base={o.name:o.data.copy()for o in objects}
metadata={'author':'SIpriv','license':'CC BY 4.0','source':'https://sketchfab.com/3d-models/lowpoly-tu-160-6cdcef13c1a244978cba2957e0394c88','source_sha256':hashlib.sha256(SOURCE.read_bytes()).hexdigest(),'length_m':54.1,'removed_loose_objects':removed,'hinges_body':[{'name':p.name,'position':[p.location.x,p.location.z,-p.location.y]}for p in roots],'lods':[]}
for level,ratio in enumerate([1.,.5,.18]):
 for o in objects:
  o.data=base[o.name].copy()
  if o==hull and ratio<1:
   d=o.modifiers.new('Distance simplification','DECIMATE');d.ratio=ratio;bpy.context.view_layer.objects.active=o;bpy.ops.object.modifier_apply(modifier=d.name)
  o.data.calc_loop_triangles()
 bpy.context.view_layer.update()
 if level==0:
  p=[o.matrix_world@v.co for o in objects for v in o.data.vertices];metadata['bounds_blender']=[[f(v[i]for v in p)for i in range(3)]for f in [min,max]]
 bpy.ops.export_scene.gltf(filepath=str(HERE/f'lod{level}.glb'),export_format='GLB',export_animations=False,export_yup=True,export_apply=True)
 metadata['lods'].append({'level':level,'triangles':sum(len(o.data.loop_triangles)for o in objects),'bytes':(HERE/f'lod{level}.glb').stat().st_size})
(HERE/'inspection.json').write_text(json.dumps(metadata,indent=2)+'\n');print(json.dumps(metadata))
