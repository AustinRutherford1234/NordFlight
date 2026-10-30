"""Check generated door/tyre intersections over the runtime gear stroke.
Run: blender --background --python-exit-code 1 --python scripts/fa18f/validate_gear_sweep.py
Coordinates here are the authoring space; transforms mirror hornet_gear.rs.
"""
import bpy,math
from pathlib import Path
from mathutils import Vector,Matrix
from mathutils.bvhtree import BVHTree
root=Path(__file__).resolve().parents[2]
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=str(root/'assets/aircraft/fa18f/rigged.glb'))
bpy.context.view_layer.update()
def data(prefix):
    obj=next(o for o in bpy.data.objects if o.name==prefix and o.type=='MESH')
    return obj,[obj.matrix_world@v.co for v in obj.data.vertices],[tuple(p.vertices) for p in obj.data.polygons]
def tree(v,p):return BVHTree.FromPolygons(v,p)
def axle(side,r,nose):
    if nose:
        return Vector((63,0,-3.5))+Matrix.Rotation(1.68*r,3,'Y')@(Vector((61.7,side*2.12,-20.17))-Vector((63,0,-3.5)))
    tuck=min(r/.45,1);lift=max(0,min((r-.45)/.55,1))
    return Vector((6.1,side*(16.03-9.53*tuck),-20.3+4.3*tuck+12*lift))
# Blender Y rotation has opposite sense to the exported Z nose hinge.
def nose_axle(side,r):
    mount=Vector((63,0,-3.5));return mount+Matrix.Rotation(-1.68*r,3,'Y')@(Vector((61.7,side*2.12,-20.17))-mount)
failures=[]
for nose in (True,False):
 for side in (-1,1):
    suffix='L' if side<0 else 'R';kind='Nose' if nose else 'Main'
    _,wv,wp=data('HornetWheel'+kind+suffix+'Mesh')
    original=Vector((61.7,side*2.12,-20.17)) if nose else Vector((6.1,side*16.03,-20.3))
    doors=[]
    for name in (['HornetNoseDoor'+suffix,'HornetNoseLegDoor'+suffix] if nose else ['HornetMainDoorFore'+suffix]):
        obj,v,p=data(name+'Mesh');doors.append((name,v,p,obj.parent.matrix_world.translation.copy()))
    for step in range(101):
        r=step/100;center=nose_axle(side,r) if nose else axle(side,r,False)
        angle=0 if nose else side*max(0,min((r-.45)/.55,1))*math.pi/2
        rot=Matrix.Rotation(angle,3,'X');w=tree([center+rot@(v-original) for v in wv],wp)
        for name,v,p,hinge in doors:
            q=Matrix.Rotation((side if nose else -side)*(1.5 if 'Leg' in name else 1.65),3,'X')
            d=tree([hinge+q@(pt-hinge) for pt in v],p)
            if w.overlap(d):failures.append((name,step))
    # Closed shells must not intersect their stowed tyre.
    center=nose_axle(side,1) if nose else axle(side,1,False)
    q=Matrix.Identity(3) if nose else Matrix.Rotation(side*math.pi/2,3,'X')
    w=tree([center+q@(v-original) for v in wv],wp)
    for name,v,p,hinge in doors:
        if w.overlap(tree(v,p)):failures.append((name,'closed'))
    for obj in bpy.data.objects:
        if obj.type=='MESH' and obj.name.startswith('HornetBayWell'):
            well=tree([obj.matrix_world@v.co for v in obj.data.vertices],[tuple(p.vertices) for p in obj.data.polygons])
            if w.overlap(well):failures.append((kind+suffix,'well'))
if failures:raise RuntimeError('Door/tyre intersections: '+repr(failures))
print('Both nose tyres and both main tyres clear their door shells at 101 stroke samples and at closed stowage')

# Use the actual station coordinates from the shared gameplay/mass/release
# definition. The AIM-120 mesh has a verified 3.66 m overall length.
import re
source=(root/'src/aircraft.rs').read_text().split('pub fn station(')[1].split('if a == Aircraft::F22')[0]
span,height,aft=map(float,re.search(r'_ => \(([-\d.]+), ([-\d.]+), ([-\d.]+)\)',source).groups())
missile_rear=(2.56-aft-1.83)*10
main_door_front=max(max(v.x for v in data('HornetMainDoorFore'+suffix+'Mesh')[1]) for suffix in ('L','R'))
assert missile_rear-main_door_front>1.5,(missile_rear,main_door_front)
print(f'Fuselage AIM-120 rear tips remain {(missile_rear-main_door_front)/10:.3f} m ahead of the main-door sweep')
