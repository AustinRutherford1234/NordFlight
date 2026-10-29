"""Check the F/A-18F derivative without rendering it.

Run: blender --background --python-exit-code 1 --python scripts/fa18f/validate_rig.py
"""
from pathlib import Path
import bpy
import json

root = Path(__file__).resolve().parents[2] / 'assets/aircraft/fa18f'
required = {'HornetCanopyPivot', 'HornetCanopyFrame', 'HornetHookPivot',
            'HornetWingTipPivotL', 'HornetWingTipPivotR', 'HornetGearNose',
            'HornetGearMainL', 'HornetGearMainR', 'HornetWheelNoseL', 'HornetWheelNoseR',
            'HornetWheelMainL', 'HornetWheelMainR', 'HornetMainDoorForeL', 'HornetMainDoorForeR', 'HornetMainLegDoorL', 'HornetMainLegDoorR', 'HornetNoseDoorL', 'HornetNoseDoorR'} | {'HornetStrut'+part+leg for part in ('Upper','Lower','Brace','ScissorA','ScissorB','Hose') for leg in ('Nose','MainL','MainR')} | {'HornetPanel%d' % n for n in (0,1,2,3,4,5,6,7,9,10,11,12)}

def inspect(path):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=str(path))
    bpy.context.view_layer.update()
    names = set(o.name for o in bpy.data.objects)
    total = 0.
    for o in bpy.data.objects:
        if o.type != 'MESH':
            continue
        ancestors = []
        parent = o
        while parent:
            ancestors.append(parent.name)
            parent = parent.parent
        if not any(('F18-airframe' in n or 'HornetPanel' in n or 'HornetWingTip' in n or 'HornetCanopyFrame' in n or 'HornetHookPivot' in n) for n in ancestors):
            continue
        mesh = o.data
        mesh.calc_loop_triangles()
        points = [o.matrix_world @ v.co for v in mesh.vertices]
        total += sum((points[t.vertices[1]]-points[t.vertices[0]]).cross(points[t.vertices[2]]-points[t.vertices[0]]).length*0.5 for t in mesh.loop_triangles)
    return names, total

_, source_area = inspect(root / 'source.glb')
names, rig_area = inspect(root / 'rigged.glb')
missing = required - names
if missing:
    raise RuntimeError('Missing rig nodes: ' + ', '.join(sorted(missing)))
relative = abs(rig_area-source_area)/source_area
expected_area=source_area-json.loads((root / "rig_metadata.json").read_text())["removed_bay_area"]
if abs(rig_area-expected_area)/source_area > 1e-4:
    raise RuntimeError(f'Unexpected airframe area after bay cutouts: {source_area} -> {rig_area}')
print(f'Rig nodes present; neutral area {source_area:.3f} -> {rig_area:.3f}, relative error {relative:.2g}')

# No residual source gear-state mesh may coexist with the articulated gear.
for obj in bpy.data.objects:
    if obj.type == 'MESH' and obj.parent and obj.parent.name.startswith('F18-landingOn_'):
        raise RuntimeError('Residual source gear fragment: '+obj.name)
# The generated hinge knuckles must actually adjoin their own panel.
from mathutils import Vector
for obj in bpy.data.objects:
    if obj.type!='MESH' or ' hinge' not in obj.name: continue
    root=obj.parent
    panel=next((c for c in root.children if c.name==root.name+'Mesh'),None)
    if panel is None: raise RuntimeError('Hinge without panel: '+obj.name)
    points=[panel.matrix_world@Vector(v) for v in panel.bound_box]
    center=obj.matrix_world.translation
    for axis in range(3):
        if not min(v[axis] for v in points)-.4 <= center[axis] <= max(v[axis] for v in points)+.4:
            raise RuntimeError('Floating hinge: '+obj.name)
print('No residual source gear meshes or detached door hinges')

# Blender authors Z, exported as glTF Y. Reject an authoring change
# that moves the link's length onto a different local axis.
import math
for obj in bpy.data.objects:
    if obj.type!='MESH':continue
    if any(not all(math.isfinite(v) for v in vertex.co) for vertex in obj.data.vertices):
        raise RuntimeError('Nonfinite geometry: '+obj.name)
    if obj.name.startswith('HornetStrut'):
        lo=[min(v.co[i] for v in obj.data.vertices) for i in range(3)]
        hi=[max(v.co[i] for v in obj.data.vertices) for i in range(3)]
        if abs(lo[2]+.5)>1e-5 or abs(hi[2]-.5)>1e-5:
            raise RuntimeError('Strut local axis no longer matches runtime: '+obj.name)
print('Finite vertices and unit-Z strut authoring verified')
import struct
raw=(Path(__file__).resolve().parents[2]/'assets/aircraft/fa18f/rigged.glb').read_bytes()
length=struct.unpack_from('<I',raw,12)[0]
gltf=json.loads(raw[20:20+length])
for node in gltf['nodes']:
    if not node.get('name','').startswith('HornetStrut') or 'mesh' not in node:continue
    for primitive in gltf['meshes'][node['mesh']]['primitives']:
        bounds=gltf['accessors'][primitive['attributes']['POSITION']]
        if abs(bounds['min'][1]+.5)>1e-5 or abs(bounds['max'][1]-.5)>1e-5:
            raise RuntimeError('Exported strut axis disagrees with runtime: '+node['name'])
print('Exported glTF unit-Y links match runtime transforms')
