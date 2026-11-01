"""Bake user-supplied CC BY Su-27P to metre-scale shared static exterior LODs."""
from pathlib import Path
import bpy,json,hashlib,math
from mathutils import Matrix
ROOT=Path(__file__).resolve().parents[2];OUT=ROOT/'assets/aircraft/su27';SOURCE=OUT/'source/original.glb'
bpy.ops.wm.read_factory_settings(use_empty=True);bpy.ops.import_scene.gltf(filepath=str(SOURCE));bpy.context.view_layer.update()
# Blender source nose is -Y, up +Z. glTF export maps Blender +Y to body -Z.
scale=21.9/(14.967+8.0);center=(14.967-8.0)/2
conversion=Matrix(((-scale,0,0,0),(0,-scale,0,center*scale),(0,0,scale,1.0*scale),(0,0,0,1)))
meshes=[];removed=[]
for o in list(bpy.data.objects):
 if o.type!='MESH':continue
 name=o.name.lower()
 if any(k in name for k in ['gear','damper','bomb','ecm','fijo','sop','aam_wings']):removed.append(o.name);continue
 if name.startswith('nose_'):
  removed.append(o.name);continue
 matrix=o.matrix_world
 if name.startswith(('canopy_','glass_')):
  hinge=__import__('mathutils').Vector((0.,-0.17,-0.5))
  matrix=Matrix.Translation(hinge)@Matrix.Rotation(math.radians(45),4,'X')@Matrix.Translation(-hinge)@matrix
 mesh=o.data.copy();mesh.transform(conversion@matrix)
 obj=bpy.data.objects.new('Su27_'+o.name,mesh);bpy.context.collection.objects.link(obj);meshes.append(obj)
# The supplied radome is modeled in its raised maintenance pose. Replace it
# with an original closed ogive fitted to the imported forward fuselage.
verts=[];faces=[]
for j in range(17):
 t=j/16;radius=0.62*math.sin(t*math.pi/2)
 for k in range(32):
  a=k*math.tau/32;verts.append(conversion@__import__('mathutils').Vector((radius*math.cos(a),-8.+2.9*t,-0.68+radius*0.86*math.sin(a))))
for j in range(16):
 for k in range(32):a=j*32+k;b=j*32+(k+1)%32;faces.append((a,b,b+32,a+32))
mesh=bpy.data.meshes.new('ClosedRadome');mesh.from_pydata(verts,[],faces);mesh.update()
obj=bpy.data.objects.new('Su27ClosedRadome',mesh);bpy.context.collection.objects.link(obj);meshes.append(obj)
mat=bpy.data.materials.new('RadomeGray');mat.diffuse_color=(0.20,0.27,0.29,1);mat.use_nodes=True
mat.node_tree.nodes.get('Principled BSDF').inputs['Base Color'].default_value=mat.diffuse_color;mesh.materials.append(mat)
for poly in mesh.polygons:poly.use_smooth=True
for o in list(bpy.data.objects):
 if o not in meshes:bpy.data.objects.remove(o,do_unlink=True)
for m in bpy.data.materials:
 if m.use_nodes:
  bs=m.node_tree.nodes.get('Principled BSDF')
  if bs:bs.inputs['Roughness'].default_value=.52
# Material-batch meshes while retaining UVs and canopy materials.
bpy.ops.object.select_all(action='SELECT');bpy.context.view_layer.objects.active=meshes[0];bpy.ops.object.join();hull=bpy.context.object;hull.name='Su27AirborneExterior';base=hull.data.copy()
meta={'author':'Rhine_Lab_Muelsyse','license':'CC BY 4.0','source':'https://sketchfab.com/3d-models/sukhoi-su-27p-flanker-b-4b73e72471b440be949b5cbc3ca36184','source_sha256':hashlib.sha256(SOURCE.read_bytes()).hexdigest(),'length_m':21.9,'nose_body':[0,0,-1],'removed':removed,'lods':[]}
for i,ratio in enumerate([1.,.35,.10]):
 hull.data=base.copy()
 if ratio<1:
  mod=hull.modifiers.new('Distance LOD','DECIMATE');mod.ratio=ratio;bpy.ops.object.modifier_apply(modifier=mod.name)
 hull.data.calc_loop_triangles()
 bpy.ops.export_scene.gltf(filepath=str(OUT/f'lod{i}.glb'),export_format='GLB',export_animations=False,export_yup=True,export_apply=True)
 meta['lods'].append({'triangles':len(hull.data.loop_triangles),'bytes':(OUT/f'lod{i}.glb').stat().st_size})
(OUT/'inspection.json').write_text(json.dumps(meta,indent=2)+'\n');print(json.dumps(meta))
