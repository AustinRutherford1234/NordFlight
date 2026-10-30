"""Prepare the licensed SB-129 Burke for Nord Flight. Blender 5.x, source untouched.
Exports shared-texture glTF LODs, per-hull markings, lighting metadata and collision skin.
"""
import bpy,bmesh,math,json,struct,hashlib,random
from pathlib import Path
from mathutils import Vector,Matrix
from mathutils.bvhtree import BVHTree
ROOT=Path(__file__).resolve().parents[2];OUT=ROOT/'assets/ships/burke';OUT.mkdir(parents=True,exist_ok=True)
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=str(OUT/'source.glb'));bpy.context.view_layer.update()
SCALE=155.3/10368.1484375;CY=2.2824;WATERLINE=9.1
removed=[]
for o in list(bpy.data.objects):
 if o.type!='MESH':continue
 if o.data.materials[0].name in ['Mat_6','Mat_7','Mat_8','Mat_9','Mat_10']:
  removed.append(o.name);bpy.data.objects.remove(o,do_unlink=True);continue
 matrix=o.matrix_world.copy();o.parent=None
 for v in o.data.vertices:
  p=matrix@v.co*SCALE;v.co=(p.y-CY,-p.x,p.z-WATERLINE)
 o.matrix_world=Matrix.Identity(4);o.data.update()
 bm=bmesh.new();bm.from_mesh(o.data);bmesh.ops.recalc_face_normals(bm,faces=list(bm.faces));bm.to_mesh(o.data);bm.free()
 for p in o.data.polygons:p.use_smooth=True
 o.data.set_sharp_from_angle(angle=math.radians(38))
 o.name='BurkeSource_'+o.name
base=[o for o in bpy.data.objects if o.type=='MESH']
# Geometry-backed deck queries exclude tiny rail/post tops and vertical walls.
verts=[];faces=[];all_verts=[];all_faces=[];face_meta=[]
for o in base:
 offset=len(all_verts);all_verts.extend(v.co.copy()for v in o.data.vertices)
 o.data.calc_loop_triangles()
 for t in o.data.loop_triangles:
  all_faces.append(tuple(offset+i for i in t.vertices))
  face_meta.append((o,tuple(t.loops)))
  a,b,c=[o.data.vertices[i].co for i in t.vertices];cross=(b-a).cross(c-a)
  if cross.length>.15 and cross.normalized().z>.7:
   j=len(verts);verts.extend([a.copy(),b.copy(),c.copy()]);faces.append((j,j+1,j+2))
decks=BVHTree.FromPolygons(verts,faces,all_triangles=True);skin=BVHTree.FromPolygons(all_verts,all_faces,all_triangles=True)
def deck(long,side):
 hit=decks.ray_cast(Vector((side,-long,60)),Vector((0,0,-1)))
 return hit[0].z if hit[0]is not None else None
def material(name,color,metal=.05,rough=.72,emission=None):
 m=bpy.data.materials.new(name);m.diffuse_color=(*color,1);m.use_nodes=True
 p=m.node_tree.nodes.get('Principled BSDF');p.inputs['Base Color'].default_value=(*color,1);p.inputs['Metallic'].default_value=metal;p.inputs['Roughness'].default_value=rough
 if emission:p.inputs['Emission Color'].default_value=(*emission,1);p.inputs['Emission Strength'].default_value=.1
 return m
paint=material('BurkePaintedSteel',(.19,.21,.23),.12)
steel=material('BurkeExposedSteel',(.15,.17,.19),.65,.4)
black=material('BurkeRubber',(.012,.017,.022),0,.86)
navy=material('BurkeCrewNavy',(.012,.028,.065),0,.9)
vest=material('BurkeCrewVest',(.58,.22,.018),0,.8)
skin_mat=material('BurkeCrewFace',(.3,.17,.1),0,.75)
window=material('BurkeWindows',(.012,.035,.045),.12,.2,(1.,.56,.23))
port=material('BurkePortLens',(.8,.012,.008),.1,.25,(1.,.01,.002))
starboard=material('BurkeStarboardLens',(.005,.5,.015),.1,.25,(.005,1.,.02))
white=material('BurkeWhiteLens',(.8,.8,.72),.1,.25,(1.,.9,.7))
letter=material('BurkeHullLetter',(.8,.82,.8),0,.72)
blank=material('BurkeFreshNumberPaint',(.11,.12,.13),.04,.8)
# Small code-authored paint normal field, shared by every source material.
normal=bpy.data.images.new('BurkePaintNormal',width=128,height=128,alpha=False);normal.colorspace_settings.name='Non-Color'
pixels=[]
for y in range(128):
 for x in range(128):
  dx=.018*math.sin(x*.83+y*.31)+.009*math.sin(x*1.71-y*.53);dy=.018*math.cos(y*.77-x*.27)
  pixels.extend([.5+dx,.5+dy,1.,1.])
normal.pixels=pixels
for m in {m for o in base for m in o.data.materials}:
 p=m.node_tree.nodes.get('Principled BSDF');p.inputs['Metallic'].default_value=.04;p.inputs['Roughness'].default_value=.76
 tex=m.node_tree.nodes.new('ShaderNodeTexImage');tex.image=normal
 n=m.node_tree.nodes.new('ShaderNodeNormalMap');n.inputs['Strength'].default_value=.2
 m.node_tree.links.new(tex.outputs['Color'],n.inputs['Color']);m.node_tree.links.new(n.outputs['Normal'],p.inputs['Normal'])
 m.name='Burke_'+m.name
created=[];lights=[];windows=0
def finish(o,name,mat):
 o.name=name;o['collision']=not name.startswith('Crew_');o.data.materials.clear();o.data.materials.append(mat);created.append(o);return o
def box(name,p,size,mat):
 bpy.ops.mesh.primitive_cube_add(size=1,location=p);o=bpy.context.object;o.scale=size;bpy.ops.object.transform_apply(location=False,rotation=False,scale=True);return finish(o,name,mat)
def tube(name,a,b,r,mat,segments=10):
 a,b=Vector(a),Vector(b);d=b-a
 bpy.ops.mesh.primitive_cylinder_add(vertices=segments,radius=r,depth=d.length,location=(a+b)/2)
 o=bpy.context.object;o.rotation_euler=d.to_track_quat('Z','Y').to_euler()
 for f in o.data.polygons:f.use_smooth=True
 return finish(o,name,mat)
def sphere(name,p,size,mat):
 bpy.ops.mesh.primitive_uv_sphere_add(segments=12,ring_count=6,radius=1,location=p);o=bpy.context.object;o.scale=size
 for f in o.data.polygons:f.use_smooth=True
 return finish(o,name,mat)
def pos(long,side,z):return Vector((side,-long,z))
# Additional attached perimeter handrails: deck queries prevent floating posts.
for side in [-7.05,7.05]:
 previous=None
 for k in range(13):
  long=54.+k*1.65;h=deck(long,side)
  if h is None:previous=None;continue
  p=pos(long,side,h+.015)
  box('Detail_RailFoot',p+Vector((0,0,.025)),(.13,.13,.05),paint)
  tube('Detail_RailPost',p,p+Vector((0,0,.95)),.022,paint)
  if previous:
   for height in [.45,.95]:tube('Detail_RailWire',previous+Vector((0,0,height)),p+Vector((0,0,height)),.009,steel,8)
  previous=p
# Flush helicopter-deck tie-down rings, not a parked helicopter.
for long in [57,61,65,69,73]:
 for side in [-5,-2.5,0,2.5,5]:
  h=deck(long,side)
  if h is None:continue
  bpy.ops.mesh.primitive_torus_add(major_radius=.075,minor_radius=.009,major_segments=12,minor_segments=6,location=pos(long,side,h+.013))
  finish(bpy.context.object,'Detail_TieDown',steel)
# Life-raft/boat-area fittings and bolted deck lockers on existing walkways.
for long,side in [(-43,-6.1),(-43,6.1),(6,-7),(6,7),(42,-6.4),(42,6.4)]:
 h=deck(long,side)
 if h is None:continue
 box('Detail_Locker',pos(long,side,h+.32),(.48,.72,.64),paint)
 box('Detail_LockerDoor',pos(long,side-.248,h+.32),(.018,.62,.54),paint)
 tube('Detail_LockerHandle',pos(long-.1,side-.265,h+.3),pos(long+.1,side-.265,h+.3),.014,steel,8)
 for dx in [-.18,.18]:
  for dy in [-.27,.27]:sphere('Detail_Fastener',pos(long+dy,side+dx,h+.645),(.018,.018,.012),steel)
# Static deck sailors, kept off the aviation landing area.
for index,(long,side) in enumerate([(-42,-5.1),(-41,-5.1),(-20,6.3),(-18,6.3),(8,-6.5),(10,-6.5),(45,5.8),(46.5,5.8)]):
 h=deck(long,side)
 if h is None:continue
 p=pos(long,side,h+.02)
 for dx in [-.105,.105]:
  box('Crew_Boot',p+Vector((dx,-.04,.065)),(.15,.27,.13),black)
  tube('Crew_Leg',p+Vector((dx,0,.15)),p+Vector((dx,0,.83)),.075,navy,10)
 sphere('Crew_Torso',p+Vector((0,0,1.14)),(.23,.15,.35),navy)
 box('Crew_LifeVest',p+Vector((0,-.035,1.2)),(.4,.27,.42),vest)
 for dx in [-.26,.26]:tube('Crew_Arm',p+Vector((dx,0,1.34)),p+Vector((dx*1.2,-.04,.91)),.061,navy,10)
 sphere('Crew_Head',p+Vector((0,0,1.65)),(.115,.105,.15),skin_mat)
 sphere('Crew_Helmet',p+Vector((0,0,1.77)),(.14,.125,.075),paint)
# Copy small cyan/blue bridge panes as actual shallow emissive window inserts.
for o in base:
 uv=o.data.uv_layers.active
 if not uv:continue
 m=o.data.materials[0];image=next((n.image for n in m.node_tree.nodes if n.type=='TEX_IMAGE' and n.image!=normal),None)
 if not image:continue
 for face in o.data.polygons:
  c=face.center
  if not (10<c.y<45 and 13<c.z<22 and .02<face.area<5):continue
  coords=[uv.data[k].uv for k in face.loop_indices];u=sum(v.x for v in coords)/len(coords);v=sum(v.y for v in coords)/len(coords)
  j=(int(v%1*image.size[1])*image.size[0]+int(u%1*image.size[0]))*image.channels
  r,g,b=[image.pixels[j+k]for k in range(3)]
  if not (g>r*1.13 and b>r*1.13 and r<.55):continue
  points=[c+(o.data.vertices[i].co-c)*.97+face.normal*.006 for i in face.vertices]
  mesh=bpy.data.meshes.new('WindowPane');mesh.from_pydata(points,[],[list(range(len(points)))]);mesh.update()
  pane=bpy.data.objects.new('Detail_BridgeWindow',mesh);bpy.context.collection.objects.link(pane);finish(pane,'Detail_BridgeWindow',window);windows+=1
# Navigation fixtures and hangar-exterior/deck luminaires; fixture positions are
# model-relative visual estimates and remain separate from combat systems.
for name,long,side,h,mat,color in [('Port',-26,-6.0,18.0,port,[1.,.01,.002]),('Starboard',-26,6.0,18.0,starboard,[.005,1.,.02]),('Masthead',-10,0,36.,white,[1.,.9,.7]),('Stern',76,0,4.,white,[1.,.9,.7])]:
 p=pos(long,side,h);sphere('Light_'+name,p,(.10,.10,.13),mat)
for long,side in [(53,-4.1),(53,4.1),(-21,-6.6),(-21,6.6)]:
 h=deck(52.,side) if long==53 else deck(long,side)
 if h is None:continue
 p=pos(long,side,h-.10 if long==53 else h+.12)
 box('Detail_DeckLamp',p,(.28,.18,.12),paint);sphere('Light_DeckLens',p-Vector((0,0,.04)),(.10,.06,.025),white)
 lights.append({'position':[p.x,p.z,-p.y],'color':[1.,.76,.48],'intensity':650.,'range':15.,'direction':[0.,-1.,.35]})
# Keep semantically named mount markers for later work, without weapon logic.
for name,long,side,z in [('GunFore',-53,0,8),('VlsFore',-43,0,7),('VlsAft',43,0,8),('CiwsAft',38,0,12)]:
 marker=bpy.data.objects.new('Mount_'+name,None);bpy.context.collection.objects.link(marker);marker.location=pos(long,side,z)
# Conforming paint patches and hull numbers. Each vertex is projected onto the
# actual hull skin; no floating rectangular signs and no second visible 125.
markings={};marking_objects=[]
def surface(long,side,z,offset=.008):
 origin=pos(long,side*30,z);direction=Vector((-side,0,0));hit=skin.ray_cast(origin,direction,60)
 return None if hit[0]is None else hit[0]+Vector((side*offset,0,0))
def paint_color(long,side,z):
 hit=skin.ray_cast(pos(long+15,side*30,z),Vector((-side,0,0)),60)
 if hit[0]is None:return (.12,.13,.14,1.)
 o,loops=face_meta[hit[2]];uv=o.data.uv_layers.active
 image=next((n.image for n in o.data.materials[0].node_tree.nodes if n.type=='TEX_IMAGE' and n.image!=normal),None)
 if not uv or not image:return (.12,.13,.14,1.)
 from mathutils.geometry import barycentric_transform
 triangle=all_faces[hit[2]]
 coords=[Vector((uv.data[k].uv.x,uv.data[k].uv.y,0))for k in loops]
 p=barycentric_transform(hit[0],*[all_verts[k]for k in triangle],*coords)
 i=(int(p.y%1*image.size[1])*image.size[0]+int(p.x%1*image.size[0]))*image.channels
 rgb=[image.pixels[i+k]for k in range(3)]
 if max(rgb)-min(rgb)>.12 or max(rgb)>.65:rgb=[.38,.39,.40]
 return tuple(c/12.92 if c<=.04045 else ((c+.055)/1.055)**2.4 for c in rgb)+(1.,)
blank.use_nodes=True
vertex=blank.node_tree.nodes.new('ShaderNodeVertexColor');vertex.layer_name='PaintColor'
blank.node_tree.links.new(vertex.outputs['Color'],blank.node_tree.nodes['Principled BSDF'].inputs['Base Color'])
for number in [125,128]:
 pieces=[]
 for side in [-1,1]:
  # A narrow fresh paint field masks the source number on the existing hull.
  points=[];faces=[];colors=[];valid=[];columns=40;rows=20
  for iy in range(rows+1):
   for ix in range(columns+1):
    long=-71.+ix*.45;z=3.+iy*.38;p=surface(long,side,z)
    valid.append(p is not None);points.append(p if p is not None else pos(long,side*5,z));colors.append(paint_color(long,side,z))
  for iy in range(rows):
   for ix in range(columns):
    a=iy*(columns+1)+ix;face=(a,a+1,a+columns+2,a+columns+1)
    if all(valid[k]for k in face):faces.append(face if side<0 else tuple(reversed(face)))
  mesh=bpy.data.meshes.new('HullPaint');mesh.from_pydata(points,[],faces);mesh.update();attribute=mesh.color_attributes.new(name='PaintColor',type='FLOAT_COLOR',domain='POINT')
  for i,color in enumerate(colors):attribute.data[i].color=color
  for p in mesh.polygons:p.use_smooth=True
  o=bpy.data.objects.new('HullNumberPaint',mesh);bpy.context.collection.objects.link(o);o.data.materials.append(blank);pieces.append(o)
  bpy.ops.object.text_add();text=bpy.context.object;text.data.body=str(number);text.data.align_x='CENTER';text.data.size=3.0;text.data.extrude=0.;text.data.resolution_u=4
  bpy.ops.object.convert(target='MESH');o=bpy.context.object
  bm=bmesh.new();bm.from_mesh(o.data);bmesh.ops.triangulate(bm,faces=list(bm.faces));bmesh.ops.subdivide_edges(bm,edges=list(bm.edges),cuts=3,use_grid_fill=True);bm.to_mesh(o.data);bm.free()
  for v in o.data.vertices:
   u,vv=v.co.x,v.co.y;p=surface(-61.8-side*u,side,5.3+vv,.025)
   v.co=p if p is not None else pos(-61.8-side*u,side*5,5.3+vv)
  o.name='HullNumber'+str(number);o.data.materials.append(letter);pieces.append(o)
 # Stern identity plate on the model transom, with a shallow text line.
 bpy.ops.object.text_add();o=bpy.context.object;o.data.body='JACK H. LUCAS' if number==125 else 'TED STEVENS';o.data.align_x='CENTER';o.data.size=.36;o.data.extrude=0.;bpy.ops.object.convert(target='MESH')
 for v in o.data.vertices:
  hit=skin.ray_cast(Vector((v.co.x,-100.,2.4+v.co.y)),Vector((0,1,0)),60)
  v.co=(hit[0]-Vector((0,.015,0))) if hit[0]is not None else Vector((v.co.x,-77.,2.4+v.co.y))
 o.data.materials.append(letter);o.name='SternName'+str(number);pieces.append(o)
 bpy.ops.object.text_add();o=bpy.context.object;o.data.body=str(number);o.data.align_x='CENTER';o.data.size=1.1;o.data.extrude=0.;bpy.ops.object.convert(target='MESH')
 h=deck(74.,0.)
 for v in o.data.vertices:v.co=Vector((v.co.x,-74.+v.co.y,(h or 3.)+.015))
 o.data.materials.append(letter);o.name='FlightDeckIdentity'+str(number);pieces.append(o)
 # Small, harmless staging differences between the two hulls: mooring coils.
 for k in range(2):
  side=-6. if number==125 else 6.;long=-40.+k*.65;h=deck(long,side)
  if h is None:continue
  bpy.ops.mesh.primitive_torus_add(major_radius=.18,minor_radius=.025,major_segments=16,minor_segments=8,location=pos(long,side,h+.035))
  o=bpy.context.object;o.name='IdentityMooringCoil'+str(number);o.data.materials.append(steel);pieces.append(o)
 markings[number]=pieces;marking_objects.extend(pieces)
# Join compatible static detail leaves, keeping the asset inexpensive to submit.
def merge(objects):
 groups={}
 for o in objects:
  if o.type=='MESH':groups.setdefault(tuple(m.name for m in o.data.materials)+(str(o.get('collision',True)),),[]).append(o)
 result=[]
 for group in groups.values():
  bpy.ops.object.select_all(action='DESELECT')
  for o in group:o.select_set(True)
  bpy.context.view_layer.objects.active=group[0]
  if len(group)>1:bpy.ops.object.join()
  result.append(group[0])
 return result
static=merge(created)
base=[o for o in base if o.name in bpy.data.objects]+static
# Collision geometry is exported before LOD simplification. Crew and painted
# number/window overlays are intentionally excluded from the solid ship skin.
triangles=[]
for o in base:
 if not o.get('collision',True)or o.data.materials[0]==window:continue
 o.data.calc_loop_triangles()
 for t in o.data.loop_triangles:
  p=[o.matrix_world@o.data.vertices[i].co for i in t.vertices]
  game=[(q.x,q.z,-q.y)for q in p]
  if (p[1]-p[0]).cross(p[2]-p[0]).length_squared<1e-12:continue
  triangles.extend(c for q in game for c in q)
data=struct.pack('<'+'f'*len(triangles),*triangles);(OUT/'collision.bin').write_bytes(data)
def export(path,objects):
 bpy.ops.object.select_all(action='DESELECT')
 for o in objects:o.select_set(True)
 bpy.context.view_layer.objects.active=objects[0]
 bpy.ops.export_scene.gltf(filepath=str(OUT/path),export_format='GLTF_SEPARATE',use_selection=True,export_texture_dir='textures',export_image_format='AUTO',export_yup=True,export_normals=True,export_extras=True)
 g=json.loads((OUT/path).read_text());g['asset']['copyright']='Base model SB-129 (hrd4588), CC BY 4.0; modified for Nord Flight. See README.md for source and changes.'
 (OUT/path).write_text(json.dumps(g,separators=(',',':'))+'\n')
 return g
for number,objects in markings.items():export('markings'+str(number)+'.gltf',objects)
report={'source_sha256':hashlib.sha256((OUT/'source.glb').read_bytes()).hexdigest(),'license':'CC-BY-4.0','author':'SB-129','length_m':155.3,'source_waterline_m':WATERLINE,'source_center_y_m':CY,'removed_helicopter_objects':removed,'bridge_window_panes':windows,'lights':lights,'collision_triangles':len(triangles)//9,'collision_sha256':hashlib.sha256(data).hexdigest(),'lods':[]}
for level,ratio in enumerate([1.,.45,.12,.025]):
 copies=[]
 for o in base:
  # Static sailors are useful only in the highest-detail view.
  if level>0 and not o.get('collision',True):continue
  copy=o.copy();copy.data=o.data.copy();bpy.context.collection.objects.link(copy);copies.append(copy)
  if level>0 and len(copy.data.polygons)>16:
   mod=copy.modifiers.new('DistanceSimplification','DECIMATE');mod.ratio=ratio;mod.use_collapse_triangulate=True
   bpy.context.view_layer.objects.active=copy;bpy.ops.object.modifier_apply(modifier=mod.name)
 g=export('lod'+str(level)+'.gltf',copies+[o for o in bpy.data.objects if o.type=='EMPTY' and o.name.startswith('Mount_')])
 count=sum(g['accessors'][p['indices']]['count']//3 for m in g.get('meshes',[])for p in m['primitives'])
 report['lods'].append({'file':'lod'+str(level)+'.gltf','triangles':count,'materials':[m.get('name','')for m in g.get('materials',[])],'texture_uris':[i.get('uri')for i in g.get('images',[])]})
 for o in copies:bpy.data.objects.remove(o,do_unlink=True)
(OUT/'inspection.json').write_text(json.dumps(report,indent=2)+'\n')
# Editable source remains local; exported asset and scripts are the distributable inputs.
bpy.ops.wm.save_as_mainfile(filepath=str(ROOT/'tests/escort-model-review/prepared.blend'))
print('BURKE_PREPARED',json.dumps(report))
