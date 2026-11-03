"""Preserve and articulate the supplied SR-71; Blender background or UI. Game coordinates in metadata."""
from pathlib import Path
import bpy,bmesh,numpy as np,json,math,sys
from mathutils import Vector,Matrix
from mathutils.bvhtree import BVHTree
sys.path.insert(0,str(Path(__file__).parent))
from prepare import load_source
ROOT=Path(__file__).resolve().parents[2];OUT=ROOT/'assets/aircraft/sr71';REVIEW=ROOT/'tests/sr71-rig';REVIEW.mkdir(parents=True,exist_ok=True)
objects,scale,_,_=load_source()
def game(v):return Vector((v[0],v[2],-v[1]))
def blender(v):return Vector((v[0],-v[2],v[1]))
def stem(o):return o.name.split('-FACES')[0]
source={stem(o):o for o in objects}
# Duplicate named glazing surfaces share the same position in this source; retain one layer.
for o in list(objects):
 if o.name=='fuzel_I01-FACES.003':objects.remove(o);bpy.data.objects.remove(o,do_unlink=True)
metadata={'version':1,'pivots':[],'links':[],'source_triangles':31413,'notes':['Timing, hinges and actuator strokes are fitted visual estimates; source geometry retained.']}
source={}
for o in objects:source.setdefault(stem(o),o)
# Original skin BVH, before cutting wells: closed door vertices project to this exact surface.
verts=[];polys=[]
for o in objects:
 if stem(o) not in ['fuzel_I01','Line04','Line13','Object01','Object12']:continue
 offset=len(verts);verts.extend([v.co.copy()for v in o.data.vertices]);polys.extend([[offset+i for i in p.vertices]for p in o.data.polygons])
skin=BVHTree.FromPolygons(verts,polys)
def underside(x,z):
 hit,_,_,_=skin.ray_cast(blender((x,-3.,z)),Vector((0,0,1)),6.)
 return game(hit).y if hit is not None else (-.5 if abs(x)<.5 else -.15)
def panel_surface(x,z):
 # The source already has open wells: an upward ray INSIDE a well hits its roof, not its outer skin.
 # Interpolate the authored belly profiles immediately fore/aft of the opening instead.
 a,b=(-7.85,-4.10)if z<0 else(5.45,7.10)
 t=max(0,min(1,(z-a)/(b-a)))
 return underside(x,a)*(1-t)+underside(x,b)*t
def parent_keep(o,p):
 bpy.context.view_layer.update();mat=o.matrix_world.copy();o.parent=p;o.matrix_world=mat
pivots={}
def pivot(name,at,kind,axis=(0,0,1),angle=0.,index=0,parent=None):
 o=bpy.data.objects.new(name,None);bpy.context.scene.collection.objects.link(o);o.location=blender(at)
 bpy.context.view_layer.update()
 if parent:parent_keep(o,parent)
 pivots[name]=o
 metadata['pivots'].append(dict(name=name,kind=kind,index=index,at=list(at),axis=list(axis),angle=angle))
 return o
def attach(names,p):
 for name in names:
  for o in objects:
   if stem(o)==name:parent_keep(o,p)
# Real source triplex mains fold inboard; nose rotates forward without axial twist.
gear=[]
for idx,side in [(0,0),(1,-1),(2,1)]:
 at=(0,0.,-5.52)if idx==0 else(side*2.56,.21,6.27)
 axis=(1,0,0)if idx==0 else(0,0,1)
 angle=math.pi/2 if idx==0 else(-side*math.pi/2)
 name=['Sr71GearNose','Sr71GearLeft','Sr71GearRight'][idx];gear.append(pivot(name,at,'gear',axis,angle,idx))
# Keep the original gear/hub/fork/brace meshes, rather than drawing replacement assemblies.
mainL=['Box05','Cylinder13','Object09','Box07','Box06','Object08','Cylinder14']
mainR=['Box10','Cylinder21','Object11','Box09','Box08','Object10','Cylinder16']
nose=['Box02','Box04','Box03','Cylinder10','Cylinder09','Box11','Cylinder08','Cylinder07','Sphere01','Sphere02']
attach(mainL,gear[1]);attach(mainR,gear[2]);attach(nose,gear[0])
# Wheels spin about actual axles, with hubs carried by the same pivots.
for idx,pairs in [(1,[('Torus01','Cylinder02'),('Torus02','Cylinder03'),('Torus03','Cylinder04')]),(2,[('Torus08','Cylinder18'),('Torus06','Cylinder19'),('Torus07','Cylinder20')]),(0,[('Torus04','Cylinder05'),('Torus05','Cylinder06')])]:
 for j,(tyre,hub) in enumerate(pairs):
  o=source[tyre];coords=[game(v.co)for v in o.data.vertices];at=tuple((min(v[k]for v in coords)+max(v[k]for v in coords))*.5 for k in range(3))
  wp=pivot(f'Sr71Wheel{idx}_{j}',at,'wheel',(1,0,0),0.,idx,parent=gear[idx]);attach([tyre,hub],wp)
# Steering below the upper strut; wheel pivots reparent with preserved world pose.
steer=pivot('Sr71NoseSteering',(0,-1.05,-5.52),'steering',(0,1,0),0.,0,parent=gear[0])
for name in ['Sr71Wheel0_0','Sr71Wheel0_1']:parent_keep(pivots[name],steer)
attach(['Box04','Box03','Cylinder10','Cylinder07'],steer)
# Sliding lower gear carriers preserve the source forks, hubs and tyre detail.
for idx,names in [(1,['Box05','Box07','Box06','Object09']),(2,['Box10','Box09','Box08','Object11'])]:
 op=pivot('Sr71Oleo'+str(idx),metadata['pivots'][idx]['at'],'oleo',(0,1,0),0.,idx,parent=gear[idx])
 for wp in list(pivots.values()):
  if wp.name.startswith('Sr71Wheel'+str(idx)+'_'):parent_keep(wp,op)
 attach(names,op)
# Door geometry is the authored panel, refitted in its closed pose to the ORIGINAL skin.
# Insets, thickness and edge radii are derived from that panel, not generic flat boxes.
doors=[]
for name,key,side,x0,x1,z0,z1,open_angle in [
 ('NoseL','Object03',-1,-.465,-.006,-7.55,-5.40,-88),('NoseR','stvorknos',1,.006,.465,-7.55,-5.40,88),
 ('MainOuterL','Object06',-1,-2.94,-1.40,5.76,6.81,-92),('MainOuterR','Object07',1,1.40,2.94,5.76,6.81,92),
 ('MainInnerL','Object04',-1,-1.394,-.065,5.76,6.81,92),('MainInnerR','Object05',1,.065,1.394,5.76,6.81,-92)]:
 o=source[key]
 # Keep the authored exterior sheet; mapping its original thickness to width would fold the panel.
 bm=bmesh.new();bm.from_mesh(o.data);bm.normal_update()
 outer=[f for f in bm.faces if game(f.normal).x*side>0.2]
 if outer:
  bmesh.ops.delete(bm,geom=[f for f in bm.faces if f not in outer],context='FACES')
  bm.to_mesh(o.data)
 bm.free()
 g=np.array([tuple(game(v.co))for v in o.data.vertices]);
 # Source panel height becomes closed panel width; preserve its longitudinal curvature and UV layout.
 h0,h1=g[:,1].min(),g[:,1].max();zlo,zhi=g[:,2].min(),g[:,2].max()
 inner='Inner'in name;hingex=(x1 if side<0 else x0)if inner else(x0 if side<0 else x1)
 at=(hingex,panel_surface(hingex,(z0+z1)*.5)-.008,(z0+z1)*.5)
 dp=pivot('Sr71Door'+name,at,'door',(0,0,1),math.radians(open_angle),0 if 'Nose'in name else 1 if side<0 else 2)
 for v,row in zip(o.data.vertices,g):
  u=(row[1]-h0)/(h1-h0);x=(x0+(x1-x0)*u)if abs(hingex-x1)<.01 else(x1-(x1-x0)*u)
  z=z0+(row[2]-zlo)/(zhi-zlo)*(z1-z0);v.co=blender((x,panel_surface(x,z)+.038,z))
 parent_keep(o,dp);doors.append((name,o,dp))
# Aft nose door closes against the same belly surface and follows the leg sequence.
o=source['stvnos1'];coords=np.array([tuple(game(v.co))for v in o.data.vertices]);lo=coords.min(0);hi=coords.max(0)
at=(0,panel_surface(0,-4.36)-.008,-4.36);dp=pivot('Sr71DoorNoseAft',at,'door',(1,0,0),math.radians(-68),0)
for v,row in zip(o.data.vertices,coords):
 x=(row[0]-(lo[0]+hi[0])*.5)*(.92/(hi[0]-lo[0]));z=-5.394+(row[2]-lo[2])/(hi[2]-lo[2])*1.028
 v.co=blender((x,panel_surface(x,z)+.038,z))
parent_keep(o,dp);doors.append(('NoseAft',o,dp))
# Hollow door edges and recessed inner surfaces: solidify source, retaining outer paint/UVs.
interior=bpy.data.materials.new('SR71 recessed warm-grey door interior');interior.diffuse_color=(.19,.21,.18,1);interior.use_nodes=True
bsdf=interior.node_tree.nodes.get('Principled BSDF');bsdf.inputs['Base Color'].default_value=(.19,.21,.18,1);bsdf.inputs['Metallic'].default_value=.35;bsdf.inputs['Roughness'].default_value=.65
for name,o,p in doors:
 o.name='Sr71SkinDoor'+name;o.data.materials.append(interior)
 bm=bmesh.new();bm.from_mesh(o.data)
 for face in bm.faces:
  if face.normal.z>0:face.normal_flip()
 bm.to_mesh(o.data);bm.free()
 bpy.context.view_layer.objects.active=o;mod=o.modifiers.new('Folded metal edge','SOLIDIFY');mod.thickness=.018;mod.offset=-1;mod.material_offset=len(o.data.materials)-1;mod.material_offset_rim=len(o.data.materials)-1
 bpy.ops.object.modifier_apply(modifier=mod.name)
# Open true wells in the stationary lower skin, retaining its UVs, and extrude cut boundaries inward.
bays=[(-.469,.469,-7.56,-4.35,.42),(-2.95,-.055,5.75,6.82,.60),(.055,2.95,5.75,6.82,.60)]
for o in objects:
 if stem(o)not in ['fuzel_I01','Line04','Line13']:continue
 o.data.materials.append(interior);lining=len(o.data.materials)-1
 bm=bmesh.new();bm.from_mesh(o.data)
 for x0,x1,z0,z1,roof in bays:
  # Bisect only when the airframe's source bounds overlap the well.
  ps=[game(v.co)for v in bm.verts]
  if not ps or max(v.x for v in ps)<x0 or min(v.x for v in ps)>x1:continue
  for pt,normal in [((x0,0,0),(1,0,0)),((x1,0,0),(1,0,0)),((0,0,z0),(0,0,1)),((0,0,z1),(0,0,1)),((0,.12,0),(0,1,0))]:
   bmesh.ops.bisect_plane(bm,geom=list(bm.verts)+list(bm.edges)+list(bm.faces),dist=1e-5,plane_co=blender(pt),plane_no=blender(normal))
  for pt,normal in [((0,0,0),(1,0,0)),((-1.397,0,0),(1,0,0)),((1.397,0,0),(1,0,0)),((0,0,-5.397),(0,0,1))]:
   bmesh.ops.bisect_plane(bm,geom=list(bm.verts)+list(bm.edges)+list(bm.faces),dist=1e-5,plane_co=blender(pt),plane_no=blender(normal))
  bm.faces.index_update()
  gone=[f for f in bm.faces if x0+1e-5<game(f.calc_center_median()).x<x1-1e-5 and z0+1e-5<game(f.calc_center_median()).z<z1-1e-5 and game(f.calc_center_median()).y < panel_surface(game(f.calc_center_median()).x,game(f.calc_center_median()).z)+.015]
  def door_for(face):
   c=game(face.calc_center_median())
   if c.z<0:return 'NoseAft'if c.z> -5.397 else'NoseL'if c.x<0 else'NoseR'
   return 'Main'+('Inner'if abs(c.x)<1.397 else'Outer')+('L'if c.x<0 else'R')
  for label in set(door_for(face)for face in gone):
   faces=[f for f in gone if door_for(f)==label]
   if not faces:continue
   # Copy the bisected authored skin and its loop UVs, then retain this door footprint only.
   temp=bm.copy();temp.faces.ensure_lookup_table();bm.faces.ensure_lookup_table();keep={f.index for f in faces}
   bmesh.ops.delete(temp,geom=[f for f in temp.faces if f.index not in keep],context='FACES')
   for face in temp.faces:
    if face.normal.z>0:face.normal_flip()
   data=bpy.data.meshes.new('SR71 extracted painted door '+label);temp.to_mesh(data);temp.free()
   panel=bpy.data.objects.new('Sr71PaintDoor'+label+o.name,data);bpy.context.scene.collection.objects.link(panel)
   for mat in o.data.materials:data.materials.append(mat)
   parent_keep(panel,pivots['Sr71Door'+label])
   bpy.context.view_layer.objects.active=panel
   mod=panel.modifiers.new('Actual skin gauge','SOLIDIFY');mod.thickness=.012;mod.offset=-1;mod.material_offset=lining;mod.material_offset_rim=lining
   bpy.ops.object.modifier_apply(modifier=mod.name)

  bmesh.ops.delete(bm,geom=gone,context='FACES')
  edges=[e for e in bm.edges if e.is_boundary and all(x0-.002<=game(v.co).x<=x1+.002 and z0-.002<=game(v.co).z<=z1+.002 and game(v.co).y<.13 for v in e.verts)]
  if edges:
   result=bmesh.ops.extrude_edge_only(bm,edges=edges)
   for v in result['geom']:
    if isinstance(v,bmesh.types.BMVert):
     gp=game(v.co);hit,_,_,_=skin.ray_cast(blender((gp.x,4.,gp.z)),Vector((0,0,-1)),8.);v.co.z=min(roof,game(hit).y-.018)if hit is not None else roof
    elif isinstance(v,bmesh.types.BMFace):v.material_index=lining
  # Roof uses the cut edges' existing vertices; fills each source patch without a floating external box.
  top=[e for e in bm.edges if e.is_boundary and all(abs(v.co.z-roof)<1e-4 for v in e.verts)]
  if top:
   filled=bmesh.ops.holes_fill(bm,edges=top,sides=0)
   for face in filled['faces']:face.material_index=lining
 bm.to_mesh(o.data);bm.free();o.data.update()
# Separate the two supplied canopy shells, leaving the forward windscreen fixed.
canopies=[]
for suffix,z0,z1 in [('Front',-10.65,-9.55),('Rear',-9.55,-8.20)]:
 cp=pivot('Sr71Canopy'+suffix,(0,.43,z1),'canopy',(1,0,0),math.radians(62));canopies.append(cp)
for o in list(objects):
 if stem(o)not in ['kabina','fuzel_I01']:continue
 bm=bmesh.new();bm.from_mesh(o.data)
 for z in [-10.65,-9.55,-8.20]:bmesh.ops.bisect_plane(bm,geom=list(bm.verts)+list(bm.edges)+list(bm.faces),dist=1e-5,plane_co=blender((0,0,z)),plane_no=blender((0,0,1)))
 bmesh.ops.bisect_plane(bm,geom=list(bm.verts)+list(bm.edges)+list(bm.faces),dist=1e-5,plane_co=blender((0,.43,0)),plane_no=blender((0,1,0)))
 for suffix,z0,z1 in [('Front',-10.65,-9.55),('Rear',-9.55,-8.20)]:
  bm.faces.index_update();selected=[f for f in bm.faces if z0<game(f.calc_center_median()).z<z1 and game(f.calc_center_median()).y>.431 and abs(game(f.calc_center_median()).x)<.9]
  if not selected:continue
  keep={f.index for f in selected};tmp=bm.copy();tmp.faces.ensure_lookup_table();bmesh.ops.delete(tmp,geom=[f for f in tmp.faces if f.index not in keep],context='FACES')
  data=bpy.data.meshes.new('SR71 authored canopy '+suffix);tmp.to_mesh(data);tmp.free()
  child=bpy.data.objects.new('Sr71CanopySkin'+suffix+o.name,data);bpy.context.scene.collection.objects.link(child)
  for material in o.data.materials:data.materials.append(material)
  parent_keep(child,pivots['Sr71Canopy'+suffix]);bmesh.ops.delete(bm,geom=selected,context='FACES')
 bm.to_mesh(o.data);bm.free()
# Seven missing closed outer skins: bounded tessellated patches sampled from the authored belly profiles.
# The supplied asset has open wheel wells and no closed-pose skin across those openings.
# Everything else (legs, wheels, braces, doors' inner structure, moving surfaces) stays source-derived.
from mathutils.geometry import barycentric_transform
for label,x0,x1,z0,z1 in [('NoseL',-.469,0.,-7.56,-5.397),('NoseR',0.,.469,-7.56,-5.397),('NoseAft',-.469,.469,-5.397,-4.35),('MainInnerL',-1.397,-.055,5.75,6.82),('MainOuterL',-2.95,-1.397,5.75,6.82),('MainInnerR',.055,1.397,5.75,6.82),('MainOuterR',1.397,2.95,5.75,6.82)]:
 positions=[];uvs=[];faces=[];nx,nz=16,12
 ref=source['fuzel_I01'if'Outer'not in label else'Line04'if label.endswith('L')else'Line13']
 # Source texture coordinates sampled along nearby existing painted triangles.
 ref.data.calc_loop_triangles();tris=[t for t in ref.data.loop_triangles if abs(t.normal.z)>.25 and t.material_index==0];layer=ref.data.uv_layers.active
 centers=np.array([tuple(sum((ref.data.vertices[i].co for i in t.vertices),Vector())/3)for t in tris])
 for iz in range(nz+1):
  z=z0+(z1-z0)*iz/nz
  for ix in range(nx+1):
   x=x0+(x1-x0)*ix/nx;y=panel_surface(x,z)-.006;pt=blender((x,y,z));positions.append(tuple(pt))
   if layer and tris:
    target=blender((x,panel_surface(x,z),(-7.85 if z< -5.9 else -4.10)if z<0 else(5.45 if z<6.3 else 7.10)))
    tri=tris[int(np.sum((centers-np.array(target))**2,axis=1).argmin())]
    a,b,c=[ref.data.vertices[i].co for i in tri.vertices];ta,tb,tc=[Vector((*layer.data[i].uv,0))for i in tri.loops]
    mapped=barycentric_transform(pt,a,b,c,ta,tb,tc);uvs.append((mapped.x,mapped.y))
   else:uvs.append((ix/nx,iz/nz))
 for iz in range(nz):
  for ix in range(nx):
   a=iz*(nx+1)+ix;faces.extend([(a,a+1,a+nx+2),(a,a+nx+2,a+nx+1)])
 data=bpy.data.meshes.new('SR71 fitted closure '+label);data.from_pydata(positions,[],faces);data.materials.append(ref.data.materials[0]);data.materials.append(interior)
 layer=data.uv_layers.new(name='UVMap')
 for poly in data.polygons:
  for loop in poly.loop_indices:layer.data[loop].uv=uvs[data.loops[loop].vertex_index]
 obj=bpy.data.objects.new('Sr71Closure'+label,data);bpy.context.scene.collection.objects.link(obj);parent_keep(obj,pivots['Sr71Door'+label])
 bpy.context.view_layer.objects.active=obj;mod=obj.modifiers.new('Metal skin thickness','SOLIDIFY');mod.thickness=.012;mod.offset=-1;mod.material_offset=1;mod.material_offset_rim=1;bpy.ops.object.modifier_apply(modifier=mod.name)

# Existing all-moving fins and four separate elevons retain their authored panel borders.
for key,name,side,at,axis,kind in [
 ('Line05','FinL',-1,(-4.,1.28,10.4),(.22,1.,.05),'fin'),('Line14','FinR',1,(4.,1.28,10.4),(-.22,1.,.05),'fin'),
 ('Line03','ElevonInnerL',-1,(-1.9,.14,13.1),(1.,0.,0.),'elevon'),('Line16','ElevonInnerR',1,(1.9,.14,13.1),(1.,0.,0.),'elevon'),
 ('Line01','ElevonOuterL',-1,(-6.8,.16,12.2),(1.,0.,-.23),'elevon'),('Line15','ElevonOuterR',1,(6.8,.16,12.2),(1.,0.,.23),'elevon')]:
 p=pivot('Sr71'+name,at,kind,axis,0.,0 if side<0 else 1);attach([key],p)
for side,key in [(-1,'Cone01'),(1,'Cone02')]:
 p=pivot('Sr71Spike'+('L'if side<0 else'R'),(side*4.2,.18,1.4),'spike',(0,0,1),.66);attach([key],p)
for side,key in [(-1,'dvig_stv'),(1,'dvig_stv01')]:
 p=pivot('Sr71Nozzle'+('L'if side<0 else'R'),(side*4.35,.49,10.97),'nozzle',index=0 if side<0 else 1);attach([key],p)
# Reuse source cylinder geometry for telescoping rams, eye-pin ends and door hinge barrels.
# No new procedural primitive meshes: canonical pieces are rescaled source mesh derivatives.
template=source['Cylinder09'].data.copy();v=np.array([tuple(v.co)for v in template.vertices]);mins=v.min(0);maxs=v.max(0);mid=(mins+maxs)*.5
for vert,row in zip(template.vertices,v):vert.co=((row[0]-mid[0])/max((maxs[0]-mins[0])*.5,1e-5),(row[1]-mid[1])/max((maxs[1]-mins[1])*.5,1e-5),(row[2]-mins[2])/(maxs[2]-mins[2]))
metal=bpy.data.materials.new('SR71 hydraulic satin alloy');metal.diffuse_color=(.32,.35,.34,1);metal.use_nodes=True
n=metal.node_tree.nodes.get('Principled BSDF');n.inputs['Base Color'].default_value=(.32,.35,.34,1);n.inputs['Metallic'].default_value=.75;n.inputs['Roughness'].default_value=.28
chrome=metal.copy();chrome.name='SR71 polished piston';chrome.diffuse_color=(.55,.58,.61,1);chrome.node_tree.nodes.get('Principled BSDF').inputs['Roughness'].default_value=.17
for key in ['Cylinder15','Cylinder17','Cylinder11','Cylinder12']:
 o=source[key];objects.remove(o);bpy.data.objects.remove(o,do_unlink=True)
def tube_instance(name,mat):
 obj=bpy.data.objects.new(name,template.copy());bpy.context.scene.collection.objects.link(obj);obj.data.materials.clear();obj.data.materials.append(mat);return obj
def link(label,a,parent,local,barrel,radius):
 bn='Sr71Ram'+label+'Barrel';rn='Sr71Ram'+label+'Rod';tube_instance(bn,metal);tube_instance(rn,chrome)
 metadata['links'].append(dict(barrel=bn,rod=rn,base=list(a),parent=parent.name,local=list(local),length=barrel,radius=radius))
for idx,side in [(1,-1),(2,1)]:link('Gear'+str(idx),(side*.72,.19,6.34),gear[idx],(0,-.53,.05),1.10,.045)
link('Gear0',(0,.31,-4.5),gear[0],(0,-.72,.08),1.10,.032)
for name,o,p in doors:
 item=next(n for n in metadata['pivots']if n['name']==p.name);at=Vector(item['at']);inner='Inner'in name
 if 'Aft'in name:local=Vector((0,0,-.55));base=at+Vector((0,.25,-.35))
 else:
  direction=(-1 if at.x>0 else 1)*(-1 if inner else 1);local=Vector((direction*(.45 if 'Main'in name else .24),0,.2));base=at+Vector((direction*.16,.27,.2))
 link('Door'+name,base,p,local,.18 if'Main'in name else .12,.018)
for idx,cp in enumerate(canopies):
 at=Vector(next(p['at']for p in metadata['pivots']if p['name']==cp.name))
 for side in [-1,1]:link('Canopy'+str(idx)+str(side),at+Vector((side*.32,-.02,-.18)),cp,(side*.30,.05,-.46),.19,.014)
def placed(name,a,b,r,mat,parent=None):
 o=tube_instance(name,mat);delta=Vector(b)-Vector(a);o.location=blender(a);o.rotation_mode='QUATERNION';o.rotation_quaternion=Vector((0,0,1)).rotation_difference(blender(delta.normalized()));o.scale=(r,r,delta.length)
 bpy.context.view_layer.update()
 if parent:parent_keep(o,parent)
 return o
for name,o,p in doors:
 at=Vector(next(d['at']for d in metadata['pivots']if d['name']==p.name))
 if name=='NoseAft':
  for side in [-1,1]:placed('Sr71Hinge'+name+str(side),at+Vector((side*.32,0,0)),at+Vector((side*.40,0,0)),.022,metal,p)
 else:
  for dz in [-.32,.32]:placed('Sr71Hinge'+name+str(dz),at+Vector((0,0,dz-.055)),at+Vector((0,0,dz+.055)),.027,metal,p)
 # Three shallow stiffeners derive from the source cylinder mesh; they stay inside the painted skin.
 vs=[game(o.matrix_world@v.co)for v in o.data.vertices];xmin=min(v.x for v in vs);xmax=max(v.x for v in vs);zmin=min(v.z for v in vs);zmax=max(v.z for v in vs)
 for j in range(1,4):
  z=zmin+(zmax-zmin)*j/4.;a=(xmin+.025,panel_surface(xmin+.025,z)+.045,z);b=(xmax-.025,panel_surface(xmax-.025,z)+.045,z)
  placed('Sr71DoorStiffener'+name+str(j),a,b,.009,interior,p)
# Store the exact unposed local transforms. Runtime and Blender review share this contract.
bpy.context.view_layer.update()
for item in metadata['pivots']:
 p=pivots[item['name']];item['rest_translation']=list(game(p.location));item['rest_rotation']=[0,0,0,1]
(OUT/'rig.json').write_text(json.dumps(metadata,indent=2)+'\n')
# Canonical export: doors closed, gear down. Runtime immediately poses doors from current gear state.
bpy.ops.object.select_all(action='DESELECT')
for o in bpy.context.scene.objects:
 if o.type=='MESH'or o.name.startswith('Sr71'):o.select_set(True)
bpy.ops.export_scene.gltf(filepath=str(OUT/'rigged.glb'),export_format='GLB',use_selection=True,export_animations=False,export_cameras=False,export_lights=False)
bpy.context.preferences.filepaths.save_version=0
bpy.ops.wm.save_as_mainfile(filepath=str(OUT/'sr71-editable.blend'))
print('SR71 RIG',len(metadata['pivots']),'pivots',len(metadata['links']),'telescoping assemblies')
