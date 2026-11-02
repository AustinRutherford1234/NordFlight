"""Bake the CC BY Su-27P maintenance-pose source to closed, airborne LODs.

Keep cockpit glazing and the fitted original radome separate from hull decimation.
The source lid and radome are open around their own object origins, not a shared
invented hinge. All transforms below are baked before glTF export.
"""
from pathlib import Path
import bpy,bmesh,json,hashlib,math
from mathutils import Matrix
ROOT=Path(__file__).resolve().parents[2];OUT=ROOT/'assets/aircraft/su27';SOURCE=OUT/'source/original.glb'
bpy.ops.wm.read_factory_settings(use_empty=True);bpy.ops.import_scene.gltf(filepath=str(SOURCE));bpy.context.view_layer.update()
scale=21.9/22.967;center=(14.967-8.0)/2
conversion=Matrix(((-scale,0,0,0),(0,-scale,0,center*scale),(0,0,scale,1.0*scale),(0,0,0,1)))
parts=[];removed=[]
for o in list(bpy.data.objects):
 if o.type!='MESH':continue
 name=o.name.lower()
 if any(k in name for k in ['gear','damper','bomb','ecm','fijo','sop','aam_wings']):removed.append(o.name);continue
 matrix=o.matrix_world.copy();group='hull'
 if name.startswith(('canopy_','glass_','nose_material')):
  # Close each opened part around the actual authored pivot, in source world.
  hinge=matrix.translation.copy();angle=125 if name.startswith('nose_material') else 40
  matrix=Matrix.Translation(hinge)@Matrix.Rotation(math.radians(angle),4,'X')@Matrix.Translation(-hinge)@matrix
 if name.startswith('nose_material'):group='radome'
 elif name.startswith('canopy_'):group='canopy_frame'
 elif name.startswith(('glass_','windshield_')):group='glazing'
 elif name.startswith('seats_'):group='seat'
 mesh=o.data.copy();mesh.transform(conversion@matrix)
 if group=='seat':mesh.transform(Matrix.Translation((0.,0.,-.20)))
 # Preserve UVs and winding after baking; avoid relying on negative-scale nodes.
 mesh.update()
 obj=bpy.data.objects.new('Su27_'+group+'_'+o.name,mesh);bpy.context.collection.objects.link(obj);parts.append((obj,group))
 for mat in mesh.materials:
  if mat.use_nodes:
   bs=mat.node_tree.nodes.get('Principled BSDF')
   if bs:bs.inputs['Roughness'].default_value=.52
  if group=='glazing':
   # A visible blue-gray coated surface, not almost-invisible 0.157-alpha glass.
   mat.name='Su27Glazing';mat.diffuse_color=(.12,.24,.30,.64);mat.use_backface_culling=False;mat.surface_render_method='BLENDED'
   bs=mat.node_tree.nodes.get('Principled BSDF')
   for key in ['Base Color','Alpha','Roughness','Metallic','Transmission Weight']:
    for link in list(bs.inputs[key].links):mat.node_tree.links.remove(link)
   bs.inputs['Base Color'].default_value=(.12,.24,.30,1);bs.inputs['Alpha'].default_value=.64;bs.inputs['Roughness'].default_value=.19;bs.inputs['Metallic'].default_value=.22;bs.inputs['Transmission Weight'].default_value=0.
for o in list(bpy.data.objects):
 if o not in [p[0]for p in parts]:bpy.data.objects.remove(o,do_unlink=True)
# Batch the opaque hull independently; critical silhouettes have their own budget.
hulls=[o for o,g in parts if g=='hull'];bpy.ops.object.select_all(action='DESELECT')
for o in hulls:o.select_set(True)
bpy.context.view_layer.objects.active=hulls[0];bpy.ops.object.join();hull=bpy.context.object;hull.name='Su27AirborneExterior'
parts=[(hull,'hull')]+[(o,g)for o,g in parts if g!='hull'];bases=[(o,g,o.data.copy())for o,g in parts]
ratios={'hull':[.225,.075,.027],'radome':[1.,.5,.1],'canopy_frame':[1.,.35,.05],'glazing':[1.,.5,.1],'seat':[.16,.04,.003]}
meta={'author':'Rhine_Lab_Muelsyse','license':'CC BY 4.0','source':'https://sketchfab.com/3d-models/sukhoi-su-27p-flanker-b-4b73e72471b440be949b5cbc3ca36184','source_sha256':hashlib.sha256(SOURCE.read_bytes()).hexdigest(),'length_m':21.9,'nose_body':[0,0,-1],'removed':removed,'repair':'Original fitted radome closed 125 degrees; canopy closed 40 degrees about authored origins; separate glazing/frame LOD budgets.','lods':[]}
close_hull=None
def forward_skin(level):
 # Reuse the known-good close mesh around the cockpit and radome join.
 mesh=close_hull.copy();bm=bmesh.new();bm.from_mesh(mesh)
 bmesh.ops.bisect_plane(bm,geom=list(bm.verts)+list(bm.edges)+list(bm.faces),dist=.00001,plane_co=(0,3.4,0),plane_no=(0,1,0),clear_inner=True)
 bm.to_mesh(mesh);bm.free();mesh.update()
 o=bpy.data.objects.new('Su27DistanceForwardSkin',mesh);bpy.context.collection.objects.link(o)
 if level==2:
  bpy.context.view_layer.objects.active=o;mod=o.modifiers.new('Forward silhouette LOD','DECIMATE');mod.ratio=.45;bpy.ops.object.modifier_apply(modifier=mod.name)
 return o
for i in range(3):
 counts={}
 for o,g,base in bases:
  o.data=base.copy();bpy.context.view_layer.objects.active=o
  ratio=ratios[g][i]
  if ratio<1:
   mod=o.modifiers.new('Distance LOD','DECIMATE');mod.ratio=ratio;bpy.ops.object.modifier_apply(modifier=mod.name)
  if i and g=='hull':
   # Replace the aggressively simplified forward skin as a complete region,
   # rather than leaving stretched source triangles over the backing surface.
   bm=bmesh.new();bm.from_mesh(o.data)
   bmesh.ops.bisect_plane(bm,geom=list(bm.verts)+list(bm.edges)+list(bm.faces),dist=.00001,plane_co=(0,3.4,0),plane_no=(0,1,0),clear_outer=True)
   bm.to_mesh(o.data);bm.free();o.data.update()
  o.data.calc_loop_triangles();counts[g]=counts.get(g,0)+len(o.data.loop_triangles)
 if i==0:close_hull=hull.data.copy()
 skin=forward_skin(i)if i else None
 if skin:skin.data.calc_loop_triangles();counts['forward_skin']=len(skin.data.loop_triangles)
 bpy.ops.export_scene.gltf(filepath=str(OUT/f'lod{i}.glb'),export_format='GLB',export_animations=False,export_yup=True,export_apply=True)
 meta['lods'].append({'triangles':sum(counts.values()),'parts':counts,'bytes':(OUT/f'lod{i}.glb').stat().st_size})
 if skin:bpy.data.objects.remove(skin,do_unlink=True)
(OUT/'inspection.json').write_text(json.dumps(meta,indent=2)+'\n');print(json.dumps(meta))
