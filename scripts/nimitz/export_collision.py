"""Export a static collision skin in the exact centred/scaled game ship frame.
Run with Blender --background --python-exit-code 1 --python this_file.
Planar simplification preserves openings and the silhouette; the source is untouched.
"""
import bpy,bmesh,math,struct,json,hashlib
from pathlib import Path
root=Path(__file__).resolve().parents[2];asset=root/'assets/ships/nimitz'
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=str(asset/'source.glb'))
bpy.context.view_layer.update()
scale=333/303.842285
triangles=[];report=[]
for obj in bpy.data.objects:
    if obj.type!='MESH':continue
    material=obj.data.materials[0].name if obj.data.materials else ''
    if 'light' in material or material in ('decal_texture','paint_bump'):continue
    bm=bmesh.new();bm.from_mesh(obj.data)
    bmesh.ops.remove_doubles(bm,verts=list(bm.verts),dist=0.00001)
    bmesh.ops.dissolve_limit(bm,angle_limit=math.radians(0.5),verts=list(bm.verts),edges=list(bm.edges),use_dissolve_boundaries=False)
    bmesh.ops.triangulate(bm,faces=list(bm.faces))
    count=0
    for face in bm.faces:
        points=[]
        for v in face.verts:
            p=obj.matrix_world@v.co
            points.append(((222.12742-p.x)*scale,p.z*scale,(51.34455+p.y)*scale))
        from mathutils import Vector
        if (Vector(points[1])-Vector(points[0])).cross(Vector(points[2])-Vector(points[0])).length_squared<1e-12:continue
        triangles.extend(c for p in points for c in p);count+=1
    report.append({'mesh':obj.name,'material':material,'triangles':count});bm.free()
data=struct.pack('<'+'f'*len(triangles),*triangles)
(asset/'collision.bin').write_bytes(data)
(asset/'collision.json').write_text(json.dumps({'source_sha256':hashlib.sha256((asset/'source.glb').read_bytes()).hexdigest(),'collision_sha256':hashlib.sha256(data).hexdigest(),'format':'little endian f32, three xyz vertices per triangle; metres in carrier local frame','planar_dissolve_degrees':0.5,'triangles':len(triangles)//9,'meshes':report},indent=2)+'\n')
print('COLLISION_EXPORT',len(triangles)//9,'triangles',len(data),'bytes')
