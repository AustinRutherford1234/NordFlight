"""Prepare an editable derivative of the CC BY-NC-SA F/A-18F source GLB.

Run: blender --background --python scripts/fa18f/build_rig.py
All source-space coordinates are in the original model's units: +X forward,
+Y port/starboard, +Z up.  The output is an approximation for game visuals.
"""
from pathlib import Path
import math
import json
import bpy
import bmesh
from mathutils import Vector

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "assets/aircraft/fa18f/source.glb"
OUTPUT = ROOT / "assets/aircraft/fa18f/rigged.glb"

bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=str(SOURCE))

def world_parent(child, parent):
    # Object operators defer matrix evaluation after rotation/scale edits.
    # Read the evaluated matrix before preserving it across reparenting.
    bpy.context.view_layer.update()
    world = child.matrix_world.copy()
    child.parent = parent
    child.matrix_world = world

def pivot(name, location):
    obj = bpy.data.objects.new(name, None)
    bpy.context.scene.collection.objects.link(obj)
    obj.location = location
    bpy.context.view_layer.update()
    return obj

def metal(name, rgba, metallic=0.7):
    m = bpy.data.materials.new(name)
    m.diffuse_color = rgba
    m.use_nodes = True
    p = m.node_tree.nodes.get("Principled BSDF")
    p.inputs["Base Color"].default_value = rgba
    p.inputs["Metallic"].default_value = metallic
    p.inputs["Roughness"].default_value = 0.4
    return m

def cylinder(name, a, b, radius, material, parent):
    a, b = Vector(a), Vector(b)
    delta = b - a
    bpy.ops.mesh.primitive_cylinder_add(vertices=20, radius=radius, depth=delta.length,
                                        location=(a+b)/2)
    obj = bpy.context.object
    obj.name = name
    obj.rotation_euler = delta.to_track_quat('Z', 'Y').to_euler()
    obj.data.materials.append(material)
    world_parent(obj, parent)
    return obj

# The source joins the fixed windscreen and opening glazing in one mesh.
# Split at their existing seam, retaining UVs and materials on both halves.
canopy = next(o for o in bpy.data.objects if o.name.startswith("F18-canopy"))
glazing = next(o for o in canopy.children if o.type == "MESH")
windshield = glazing.copy()
windshield.data = glazing.data.copy()
bpy.context.scene.collection.objects.link(windshield)
windshield.name = "HornetFixedWindscreen"
bpy.context.view_layer.update()
for obj, keep_windscreen in [(glazing, False), (windshield, True)]:
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    to_remove = []
    for face in bm.faces:
        center = obj.matrix_world @ face.calc_center_median()
        if (center.x > 72.57) != keep_windscreen:
            to_remove.append(face)
    bmesh.ops.delete(bm, geom=to_remove, context='FACES')
    bm.to_mesh(obj.data)
    bm.free()
    obj.data.update()
world_parent(windshield, canopy.parent)

# Opening glazing is hinged near the aft end of the tandem cockpit. The source
# frame in the cockpit mesh is not yet separated with the glazing.
canopy_pivot = pivot("HornetCanopyPivot", (40.39, 0., 6.05))
world_parent(canopy, canopy_pivot)

# The downloaded airframe has the wing tips joined to the skin. Bisect along
# a spanwise fold line to retain the original textures on both pieces. These
# approximate hinge positions need visual review against F/A-18F references.
airframe_parent = next(o for o in bpy.data.objects if o.name.startswith("F18-airframe"))
airframe = next(o for o in airframe_parent.children if o.type == "MESH")


bpy.context.view_layer.update()
def cut_world(obj, location, normal, remove_negative):
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    point = obj.matrix_world.inverted() @ Vector(location)
    n = (obj.matrix_world.to_3x3().transposed() @ Vector(normal)).normalized()
    bmesh.ops.bisect_plane(bm, geom=list(bm.verts) + list(bm.edges) + list(bm.faces),
                           dist=1e-5, plane_co=point, plane_no=n,
                           clear_inner=remove_negative, clear_outer=not remove_negative)
    bm.to_mesh(obj.data)
    bm.free()
    obj.data.update()

for side in (-1, 1):
    outer = airframe.copy()
    outer.data = airframe.data.copy()
    bpy.context.scene.collection.objects.link(outer)
    outer.name = "HornetWingTipMeshL" if side < 0 else "HornetWingTipMeshR"
    bpy.context.view_layer.update()
    # Preserve the outboard side of the plane for this wingtip.
    cut_world(outer, (0., side * 45., 0.), (0., side, 0.), True)
    wing_pivot = pivot("HornetWingTipPivotL" if side < 0 else "HornetWingTipPivotR", (0., side * 45., 0.))
    world_parent(outer, wing_pivot)
    # Remove the corresponding outboard section from the fixed airframe.
    cut_world(airframe, (0., side * 45., 0.), (0., side, 0.), False)

# Separate control-surface regions while retaining the UVs on the original
# skin. Each plane is (point, inward normal) in source world coordinates.
# Outlines are visual approximations and do not alter the aerodynamic model.
def partition_surface(obj, name, planes, hinge, parent=None, allow_empty=False):
    mesh = obj.data
    bm = bmesh.new()
    bm.from_mesh(mesh)
    inv = obj.matrix_world.inverted()
    normal_matrix = obj.matrix_world.to_3x3().transposed()
    local_planes = []
    for point, normal in planes:
        point = inv @ Vector(point)
        normal = (normal_matrix @ Vector(normal)).normalized()
        local_planes.append((point, normal))
        bmesh.ops.bisect_plane(bm, geom=list(bm.verts)+list(bm.edges)+list(bm.faces),
                               dist=1e-5, plane_co=point, plane_no=normal)
    chosen = [face for face in bm.faces if all((face.calc_center_median()-p).dot(n) >= -1e-5 for p,n in local_planes)]
    if not chosen:
        raise RuntimeError("No geometry selected for " + name)
    bm.faces.index_update()
    moved = bm.copy()
    # BMesh copy preserves face ordering. Resolve selection before deleting.
    moved.faces.ensure_lookup_table()
    bm.faces.ensure_lookup_table()
    indices = {f.index for f in chosen}
    bmesh.ops.delete(moved, geom=[f for f in moved.faces if f.index not in indices], context='FACES')
    bmesh.ops.delete(bm, geom=chosen, context='FACES')
    shape = bpy.data.meshes.new(name + 'Mesh')
    moved.to_mesh(shape)
    bm.to_mesh(mesh)
    moved.free()
    bm.free()
    shape.update()
    mesh.update()
    part = bpy.data.objects.new(name + 'Mesh', shape)
    bpy.context.scene.collection.objects.link(part)
    part.matrix_world = obj.matrix_world.copy()
    for material in mesh.materials:
        part.data.materials.append(material)
    hinge_obj = pivot(name, hinge)
    if parent is not None:
        world_parent(hinge_obj, parent)
    world_parent(part, hinge_obj)
    print(name, 'faces', len(shape.polygons), 'remaining', len(mesh.polygons))
    if len(shape.polygons) < 4 or (not allow_empty and len(mesh.polygons) < 100):
        raise RuntimeError('Unexpected surface partition for ' + name)
    return hinge_obj, part

# Extract the canopy bow from the source airframe instead of leaving it
# suspended over the seats when the glazing opens. The front windscreen bow
# remains fixed with the windscreen.
partition_surface(airframe, 'HornetCanopyFrame',
    [((40.4,0.,0.),(1.,0.,0.)),((72.57,0.,0.),(-1.,0.,0.)),
     ((0.,-5.5,0.),(0.,1.,0.)),((0.,5.5,0.),(0.,-1.,0.)),
     ((0.,0.,9.5),(0.,0.,1.))], (40.39,0.,6.05))

for side, base in ((-1, 0), (1, 4)):
    # Inboard trailing flaps and outboard ailerons, split at the visible seam.
    for slot, y0, y1 in ((base+1, 12., 30.), (base+2, 30., 45.)):
        planes=[((0.,side*y0,0.),(0.,side,0.)),((0.,side*y1,0.),(0.,-side,0.)),
                ((-26.,0.,0.),(1.,0.,0.)),((-6.,0.,0.),(-1.,0.,0.)),
                ((0.,0.,-2.),(0.,0.,1.)),((0.,0.,5.),(0.,0.,-1.))]
        partition_surface(airframe, 'HornetPanel%d' % slot, planes, (-6.,side*(y0+y1)*.5,0.))
    # Leading-edge flap region, bounded by a swept hinge line.
    planes=[((0.,side*25.,0.),(0.,side,0.)),((0.,side*45.,0.),(0.,-side,0.)),
            ((18.,0.,0.),(1.,0.40*side,0.)),((22.,0.,0.),(-1.,0.,0.)),
            ((0.,0.,-2.),(0.,0.,1.)),((0.,0.,5.),(0.,0.,-1.))]
    partition_surface(airframe, 'HornetPanel%d' % (9 if side<0 else 10), planes,
                      (6.,side*35.,0.))
    # Horizontal stabilators, isolated from the engine ducts and fuselage.
    planes=[((0.,side*12.,0.),(0.,side,0.)),((0.,side*38.,0.),(0.,-side,0.)),
            ((-39.,0.,0.),(-1.,0.,0.)),((0.,0.,-3.),(0.,0.,1.)),((0.,0.,6.),(0.,0.,-1.))]
    partition_surface(airframe, 'HornetPanel%d' % base, planes, (-44.,side*17.,0.))
    # Rudders on the aft portion of each canted vertical tail.
    planes=[((0.,side*7.,0.),(0.,side,0.)),((0.,side*26.,0.),(0.,-side,0.)),
            ((-42.,0.,0.),(-1.,0.,0.)),((0.,0.,7.),(0.,0.,1.))]
    partition_surface(airframe, 'HornetPanel%d' % (base+3), planes, (-42.,side*14.,15.))
    # Small leading-edge-root spoiler panel, approximated from the source skin.
    planes=[((22.,0.,0.),(1.,0.,0.)),((32.,0.,0.),(-1.,0.,0.)),
            ((0.,side*7.,0.),(0.,side,0.)),((0.,side*13.,0.),(0.,-side,0.)),
            ((0.,0.,1.),(0.,0.,1.)),((0.,0.,8.),(0.,0.,-1.))]
    partition_surface(airframe, 'HornetPanel%d' % (11 if side<0 else 12), planes, (22.,side*10.,2.))

# Partition the supplied detailed gear-down geometry into individual gear legs.
# Separate wheels, articulated links and independently sequenced doors replace
# the joined source leg. The simulator controls actual extension.
gear_parent = next(o for o in bpy.data.objects if o.name.startswith("F18-landingOn_") )
gear_mesh = next(o for o in gear_parent.children if o.type == "MESH")
# The closed-gear mesh contains distinct left and right main-bay skins.
# Extract these from the closed pose so the same geometry remains visible
# throughout the cycle instead of switching abruptly at full retraction.
closed_parent = next(o for o in bpy.data.objects if o.name.startswith("F18-landingOff_"))
closed_mesh = next(o for o in closed_parent.children if o.type == "MESH")
# Clean watertight bay panels replace the source's overlapping state meshes.
# Dimensions match the lower-skin openings; separate leg panels stay open
# around deployed struts while the large wheel-cover panels close.
door_paint=metal('Hornet bay exterior',(0.54,0.56,0.57,1.),0.25)
inner_paint=metal('Bay panel inner face',(0.64,0.66,0.64,1.),0.15)
fastener=metal('Door hinge hardware',(0.22,0.24,0.25,1.),0.8)
def door_panel(name,x0,x1,y0,y1,z,hinge):
    # A continuous cambered shell replaces detached rectangular patches.
    nx,ny=12,12
    def skin(u,v):
        inset=.48*abs(2*v-1)**8
        return (x0+inset+(x1-x0-2*inset)*u,y0+(y1-y0)*v,
                z+.36*math.sin(math.pi*v))
    vertices=[];faces=[]
    for layer in (0,1):
        for i in range(nx+1):
            for j in range(ny+1):
                x,y,h=skin(i/nx,j/ny);vertices.append((x,y,h+layer*.18))
    stride=ny+1;offset=(nx+1)*stride
    for i in range(nx):
        for j in range(ny):
            k=i*stride+j
            faces.append((k,k+stride,k+stride+1,k+1))
            faces.append((k+offset,k+1+offset,k+stride+1+offset,k+stride+offset))
    boundary=list(range(ny+1))+[i*stride+ny for i in range(1,nx+1)]+[nx*stride+j for j in range(ny-1,-1,-1)]+[i*stride for i in range(nx-1,0,-1)]
    for i,k in enumerate(boundary):
        n=boundary[(i+1)%len(boundary)];faces.append((k,n,n+offset,k+offset))
    faces=[tuple(reversed(face)) for face in faces]
    mesh=bpy.data.meshes.new(name+' shell');mesh.from_pydata(vertices,[],faces);mesh.update()
    obj=bpy.data.objects.new(name+'Mesh',mesh);bpy.context.scene.collection.objects.link(obj)
    mesh.materials.append(door_paint);mesh.materials.append(inner_paint)
    for i,face in enumerate(mesh.polygons):
        face.material_index=1 if i<nx*ny*2 and i%2 else 0
        face.use_smooth=i<nx*ny*2
    hinge_obj=pivot(name,hinge);world_parent(obj,hinge_obj)
    for fraction in (.28,.72):
        for j in range(10):
            a=Vector(skin(fraction,.04+.92*j/10));b=Vector(skin(fraction,.04+.92*(j+1)/10))
            a.z+=.22;b.z+=.22
            cylinder(name+' inner rib',a,b,.065,inner_paint,hinge_obj)
    for u in (.04,.96):
        for v in (.05,.95):
            x,y,h=skin(u,v)
            cylinder(name+' flush fastener',(x,y,h-.025),(x,y,h+.20),.055,fastener,hinge_obj)
    for fraction in (.22,.78):
        x=x0+(x1-x0)*fraction
        cylinder(name+' hinge',(x-.22,hinge[1],hinge[2]),(x+.22,hinge[1],hinge[2]),.12,fastener,hinge_obj)
for side in (-1,1):
    suffix='L' if side<0 else 'R'
    y0,y1=sorted((side*2.,side*11.))
    # One connected cover, hinged along the inboard edge. The former
    # inner/outer/leg patches rotated about distant axes and looked detached.
    door_panel('HornetMainDoorFore'+suffix,-9.6,14.,y0,y1,-6.0,(5.,side*2.,-6.0))
    lo,hi=sorted((0.,side*3.6))
    door_panel('HornetNoseDoor'+suffix,65.,81.,lo,hi,-4.4,(73.,side*3.6,-4.4))
    door_panel('HornetNoseLegDoor'+suffix,48.,65.,lo,hi,-4.4,(61.5,side*3.6,-4.4))
bpy.data.objects.remove(closed_mesh,do_unlink=True)
# Opaque bay wells prevent seeing through the aircraft between the doors.
well_paint=metal('Gear well interior',(0.27,0.29,0.29,1.),0.2)
for x0,x1,y0,y1,z0,z1 in [(48.,81.,-3.6,3.6,-4.4,4.),(-9.6,14.,-11.,-2.,-6.0,1.),(-9.6,14.,2.,11.,-6.0,1.)]:
    for loc,dim in [(((x0+x1)/2,(y0+y1)/2,z1),(x1-x0,y1-y0,.16)),((x0,(y0+y1)/2,(z0+z1)/2),(.16,y1-y0,z1-z0)),((x1,(y0+y1)/2,(z0+z1)/2),(.16,y1-y0,z1-z0)),(((x0+x1)/2,y0,(z0+z1)/2),(x1-x0,.16,z1-z0)),(((x0+x1)/2,y1,(z0+z1)/2),(x1-x0,.16,z1-z0))]:
        bpy.ops.mesh.primitive_cube_add(size=1.,location=loc)
        obj=bpy.context.object;obj.name='HornetBayWell';obj.dimensions=dim;obj.data.materials.append(well_paint)
# Roof-mounted stiffeners and service conduits stay above the swept wheel
# envelope. These are representative visual details, not a maintenance drawing.
for bay,x0,x1,y0,y1,roof in [('Nose',48.,81.,-3.6,3.6,4.),('MainL',-9.6,14.,-11.,-2.,1.),('MainR',-9.6,14.,2.,11.,1.)]:
    fixed=pivot('HornetWellDetail'+bay,(0.,0.,0.))
    for fraction in (.12,.36,.64,.88):
        x=x0+(x1-x0)*fraction
        cylinder('Well roof rib '+bay,(x,y0+.22,roof-.18),(x,y1-.22,roof-.18),.11,inner_paint,fixed)
    for offset in (.35,.65):
        y=y0+offset
        cylinder('Well hydraulic tube '+bay,(x0+.4,y,roof-.38),(x1-.4,y,roof-.38),.065,fastener,fixed)
        for fraction in (.18,.5,.82):
            x=x0+(x1-x0)*fraction
            cylinder('Well tube clamp '+bay,(x,y-.15,roof-.36),(x,y+.15,roof-.36),.10,inner_paint,fixed)
nose_pivot, nose_mesh = partition_surface(gear_mesh, 'HornetGearNose', [((40.,0.,0.),(1.,0.,0.))], (63.,0.,-6.))
# Forward door skins are now represented by the independent closed-pose
# doors. Keeping their gear-down copies on the leg swings them through the
# cockpit during retraction.
cut_world(nose_mesh, (70.,0.,0.), (1.,0.,0.), False)
partition_surface(nose_mesh, 'HornetNoseBrace',
    [((60.,0.,0.),(-1.,0.,0.)), ((0.,0.,-14.),(0.,0.,1.))], (49.,0.,-6.))
# Remove fixed bay-wall/door copies from the leg. Preserve the narrow strut
# components; only broad shallow disconnected sheets qualify.
bm=bmesh.new(); bm.from_mesh(nose_mesh.data)
unseen=set(bm.verts)
while unseen:
    seed=unseen.pop(); group=[seed]; pending=[seed]
    while pending:
        for edge in pending.pop().link_edges:
            for vertex in edge.verts:
                if vertex in unseen:
                    unseen.remove(vertex); group.append(vertex); pending.append(vertex)
    points=[nose_mesh.matrix_world @ v.co for v in group]
    lo=Vector([min(p[i] for p in points) for i in range(3)])
    hi=Vector([max(p[i] for p in points) for i in range(3)])
    if hi.x-lo.x>5. and lo.z>-10.:
        bmesh.ops.delete(bm,geom=group,context='VERTS')
bm.to_mesh(nose_mesh.data); bm.free(); nose_mesh.data.update()
main_gears = {}
for side in (-1, 1):
    main_gears[side] = partition_surface(gear_mesh, 'HornetGearMainL' if side < 0 else 'HornetGearMainR',
                      [((25.,0.,0.),(-1.,0.,0.)),((0.,0.,0.),(0.,side,0.))],
                      (7.,side*8.,-6.), allow_empty=side > 0)
    # Bay doors supplied in the down-state mesh are duplicate skins, not leg
    # structure. Independent doors above own the bay closure for every pose.
    leg_mesh=main_gears[side][1]
    cut_world(leg_mesh,(1.5,0.,0.),(1.,0.,0.),True)
    cut_world(leg_mesh,(0.,side*6.,0.),(0.,side,0.),True)

# Give the two nose wheels and the main wheels their own axles so they can
# spin independently while their parent gear retracts.
def wheel_planes(x0,x1,y0,y1,z0,z1):
    return [((x0,0.,0.),(1.,0.,0.)),((x1,0.,0.),(-1.,0.,0.)),
            ((0.,y0,0.),(0.,1.,0.)),((0.,y1,0.),(0.,-1.,0.)),
            ((0.,0.,z0),(0.,0.,1.)),((0.,0.,z1),(0.,0.,-1.))]
bpy.context.view_layer.update()
for side in (-1,1):
    y0,y1=sorted((side*1.12,side*3.13))
    partition_surface(nose_mesh, 'HornetWheelNoseL' if side<0 else 'HornetWheelNoseR',
        wheel_planes(58.85,64.55,y0,y1,-23.05,-17.25), (61.7,side*2.12,-20.17), parent=nose_pivot)
    parent, mesh = main_gears[side]
    y0,y1=sorted((side*14.5,side*17.55))
    partition_surface(mesh, 'HornetWheelMainL' if side<0 else 'HornetWheelMainR',
        wheel_planes(2.,10.2,y0,y1,-24.4,-16.2), (6.1,side*16.03,-20.3), parent=parent)

# Detailed independent struts, pistons, drag braces, torque links and hoses.
for obj in (nose_mesh, main_gears[-1][1], main_gears[1][1], gear_mesh):
    bpy.data.objects.remove(obj, do_unlink=True)
brace=bpy.data.objects.get('HornetNoseBrace')
if brace:
    for obj in list(brace.children): bpy.data.objects.remove(obj,do_unlink=True)
    bpy.data.objects.remove(brace,do_unlink=True)
paint=metal('Gear satin white',(0.72,0.74,0.73,1.),0.25)
chrome=metal('Gear polished piston',(0.65,0.68,0.7,1.),0.95)
dark=metal('Gear hydraulic hose',(0.025,0.028,0.03,1.),0.1)
for leg in ('Nose','MainL','MainR'):
    for segment,radius,mat in [('Upper',.7,paint),('Lower',.4,chrome),('Brace',.22,paint),('ScissorA',.16,paint),('ScissorB',.16,paint),('Hose',.09,dark)]:
        bpy.ops.mesh.primitive_cylinder_add(vertices=32,radius=radius,depth=1.)
        obj=bpy.context.object; obj.name='HornetStrut'+segment+leg
        obj.data.materials.append(mat)
        for face in obj.data.polygons: face.use_smooth = len(face.vertices)==4
        # Collars, gland nuts and retaining rings follow the link transform.
        if segment in ('Upper','Lower'):
            for position in (-.42,-.3,.3,.42):
                bpy.ops.mesh.primitive_torus_add(major_radius=radius*1.08,minor_radius=.008,major_segments=32,minor_segments=10,location=(0,0,position))
                ring=bpy.context.object;ring.name='Gear collar';ring.data.materials.append(paint)
                world_parent(ring,obj)
# Open the actual lower skin, rather than allowing gear to pass through it.
removed_bay_area=0.
for x0,x1,y0,y1 in [(48.,81.,-3.6,3.6),(-9.6,14.,-11.,-2.),(-9.6,14.,2.,11.)]:
    bm=bmesh.new();bm.from_mesh(airframe.data)
    inv=airframe.matrix_world.inverted()
    nm=airframe.matrix_world.to_3x3().transposed()
    for point,normal in [((x0,0,0),(1,0,0)),((x1,0,0),(1,0,0)),((0,y0,0),(0,1,0)),((0,y1,0),(0,1,0)),((0,0,-4),(0,0,1))]:
        bmesh.ops.bisect_plane(bm,geom=list(bm.verts)+list(bm.edges)+list(bm.faces),dist=1e-5,plane_co=inv@Vector(point),plane_no=(nm@Vector(normal)).normalized())
    faces=[f for f in bm.faces if x0-1e-5<(airframe.matrix_world@f.calc_center_median()).x<x1+1e-5 and y0-1e-5<(airframe.matrix_world@f.calc_center_median()).y<y1+1e-5 and (airframe.matrix_world@f.calc_center_median()).z < -4.]
    for face in faces:
        points=[airframe.matrix_world@v.co for v in face.verts]
        removed_bay_area+=sum((points[i]-points[0]).cross(points[i+1]-points[0]).length*.5 for i in range(1,len(points)-1))
    bmesh.ops.delete(bm,geom=faces,context='FACES');bm.to_mesh(airframe.data);bm.free()

# Articulate the original textured arresting-hook assembly. A second modeled
# hook would leave the original fixed one visible above the moving shank.
bm=bmesh.new();bm.from_mesh(airframe.data)
unseen=set(bm.verts); selected=set()
while unseen:
    seed=unseen.pop();group=[seed];queue=[seed]
    while queue:
        for edge in queue.pop().link_edges:
            for vertex in edge.verts:
                if vertex in unseen: unseen.remove(vertex);group.append(vertex);queue.append(vertex)
    points=[airframe.matrix_world@v.co for v in group]
    lo=Vector([min(p[i] for p in points) for i in range(3)])
    hi=Vector([max(p[i] for p in points) for i in range(3)])
    if lo.x < -35. and hi.x <= -26.+.01 and ((max(abs(lo.y),abs(hi.y))<.81 and hi.z<2.) or (hi.x < -50. and max(abs(lo.y),abs(hi.y))<1.5 and hi.z<2.)):
        selected.update(group)
bm.faces.index_update();indices={f.index for f in bm.faces if all(v in selected for v in f.verts)}
if not indices: raise RuntimeError('Source hook components not found')
moved=bm.copy();bmesh.ops.delete(moved,geom=[f for f in moved.faces if f.index not in indices],context='FACES')
bmesh.ops.delete(bm,geom=[f for f in bm.faces if f.index in indices],context='FACES')
shape=bpy.data.meshes.new('HornetHookMesh');moved.to_mesh(shape);bm.to_mesh(airframe.data)
bm.free();moved.free()
obj=bpy.data.objects.new('HornetHookMesh',shape);bpy.context.scene.collection.objects.link(obj)
obj.matrix_world=airframe.matrix_world.copy()
for mat in airframe.data.materials:obj.data.materials.append(mat)
world_parent(obj,pivot('HornetHookPivot',(-26.,0.,-2.)))

bpy.ops.export_scene.gltf(filepath=str(OUTPUT), export_format='GLB', export_animations=False,
                          export_yup=True, export_apply=False)
(OUTPUT.parent / "rig_metadata.json").write_text(json.dumps({"removed_bay_area":removed_bay_area},indent=2)+"\n")
print("Prepared", OUTPUT)
