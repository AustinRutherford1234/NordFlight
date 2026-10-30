"""Recessed F414-inspired intake detail; executed by build_rig.py.

Duct routing and blade shapes are visual estimates, not engine CAD. Preserve
external intake lips and replace only the source's internal blanking sheets.
"""
bm=bmesh.new();bm.from_mesh(airframe.data)
remove=[];removed_intake_area=0.
for f in bm.faces:
    pts=[airframe.matrix_world@v.co for v in f.verts]
    c=sum(pts,Vector())/len(pts)
    n=(airframe.matrix_world.to_3x3().inverted().transposed()@f.normal).normalized()
    if -10.<c.x<16. and 1.<abs(c.y)<12. and -9.<c.z<6.:
        ramp=c.z<2. and abs(n.x)>.20 and abs(n.z)>.90
        cap=-9.<c.x< -7.5 and abs(n.x)>.98
        if ramp or cap:
            remove.append(f)
            removed_intake_area+=sum((pts[i]-pts[0]).cross(pts[i+1]-pts[0]).length*.5 for i in range(1,len(pts)-1))
if not remove:raise RuntimeError('Intake blanking sheets not found')
bmesh.ops.delete(bm,geom=remove,context='FACES');bm.to_mesh(airframe.data);bm.free();airframe.data.update()

def duct_section(t,side):
    smooth=t*t*(3-2*t);rise=min(t/.29,1.);rise=rise*rise*(3-2*rise)
    return (25.-37.6*t,side*(8.1-2.1*smooth),-3.2+4.5*rise,3.05-.05*smooth,3.9-.9*rise,2.-smooth)

# Trim the old inner liner against the new stream tube, rather than leaving
# triangular ends projecting into it. Subtract short convex frusta; the new
# smooth liner covers the cut boundary. The external caret lip stays intact.
bm=bmesh.new();bm.from_mesh(airframe.data)
for side in (-1,1):
 for section in range(16):
    rings=[]
    for t in (section/16,(section+1)/16):
        x,cy,cz,ry,rz,power=duct_section(t,side)
        ring=[]
        for j in range(16):
            a=math.tau*j/16;c=math.cos(a);v=math.sin(a)
            ring.append(Vector((x,cy+1.04*ry*math.copysign(abs(c)**(1/power),c),cz+1.04*rz*math.copysign(abs(v)**(1/power),v))))
        rings.append(ring)
    front,back=rings;center=sum(front+back,Vector())/32
    planes=[(front[0],Vector((1.,0.,0.))),(back[0],Vector((-1.,0.,0.)))]
    for j in range(16):
        a=front[j];b=front[(j+1)%16];c=back[j];normal=(b-a).cross(c-a).normalized()
        if normal.dot(center-a)>0:normal=-normal
        planes.append((a,normal))
    xmin=back[0].x-.01;xmax=front[0].x+.01
    for point,normal in planes:
        selected=[f for f in bm.faces if min((airframe.matrix_world@v.co).x for v in f.verts)<xmax and max((airframe.matrix_world@v.co).x for v in f.verts)>xmin and side*(airframe.matrix_world@f.calc_center_median()).y>1.]
        geom=set(selected)
        for f in selected:geom.update(f.edges);geom.update(f.verts)
        bmesh.ops.bisect_plane(bm,geom=list(geom),dist=1e-5,plane_co=airframe.matrix_world.inverted()@point,plane_no=(airframe.matrix_world.to_3x3().transposed()@normal).normalized())
    remove=[]
    for f in bm.faces:
        center_f=airframe.matrix_world@f.calc_center_median()
        if xmin<center_f.x<xmax and all(normal.dot(center_f-point)<1e-4 for point,normal in planes):remove.append(f)
    for f in remove:
        points=[airframe.matrix_world@v.co for v in f.verts]
        removed_intake_area+=sum((points[i]-points[0]).cross(points[i+1]-points[0]).length*.5 for i in range(1,len(points)-1))
    bmesh.ops.delete(bm,geom=remove,context='FACES')
bm.to_mesh(airframe.data);bm.free();airframe.data.update()

duct_paint=metal('Intake ceramic gray',(0.30,.32,.32,1.),.12)
fan_metal=metal('Fan brushed titanium',(.44,.46,.49,1.),.65)
fan_dark=metal('Fan recess shadow',(.018,.022,.025,1.),.20)
def intake_mesh(name,verts,faces,mat,parent=None):
    mesh=bpy.data.meshes.new(name);mesh.from_pydata(verts,[],faces);mesh.update()
    obj=bpy.data.objects.new(name,mesh);bpy.context.scene.collection.objects.link(obj);mesh.materials.append(mat)
    for face in mesh.polygons:face.use_smooth=True
    if parent:world_parent(obj,parent)
    return obj

def ring_mesh(name,profile,cy,cz,mat,parent=None):
    verts=[];faces=[];n=64
    for x,r in profile:
        for j in range(n):
            a=math.tau*j/n;verts.append((x,cy+r*math.cos(a),cz+r*math.sin(a)))
    for i in range(len(profile)-1):
        for j in range(n):
            a=i*n+j;b=i*n+(j+1)%n;faces.append((a,b,b+n,a+n))
    return intake_mesh(name,verts,faces,mat,parent)

for side in (-1,1):
    suffix='L' if side>0 else 'R'
    # A gently offset, recessed transition, retaining the original caret lip.
    verts=[];faces=[];n=64;rows=20
    for i in range(rows+1):
        t=i/rows
        x,cy,cz,ry,rz,exponent=duct_section(t,side)
        for j in range(n):
            a=math.tau*j/n;c=math.cos(a);v=math.sin(a)
            y=math.copysign(abs(c)**(1/exponent),c)*ry
            z=math.copysign(abs(v)**(1/exponent),v)*rz
            verts.append((x,cy+y,cz+z))
    for i in range(rows):
        for j in range(n):
            a=i*n+j;b=i*n+(j+1)%n;faces.append((a,b,b+n,a+n))
    intake_mesh('HornetIntakeDuct'+suffix,verts,faces,duct_paint)
    cy=side*6.;cz=1.3
    ring_mesh('HornetFanCase'+suffix,[(-11.8,2.98),(-12.,2.98),(-12.3,2.87),(-13.,2.87),(-13.2,2.98)],cy,cz,fan_metal)
    ring_mesh('HornetFanBack'+suffix,[(-13.4,0.),(-13.4,2.90)],cy,cz,fan_dark)
    rotor=pivot('HornetIntakeRotor'+suffix,(-12.4,cy,cz))
    # One indexed mesh contains every twisted blade: no per-blade entities.
    verts=[];faces=[]
    for blade in range(36):
        start=len(verts)
        for radial in range(9):
            t=radial/8;r=.66+2.15*t
            for chord in range(5):
                u=chord/4;angle=math.tau*blade/36+.12*t+.155*(u-.5)
                x=-12.35+.28*(u-.5)*(1-.55*t)+.065*math.sin(math.pi*u)
                verts.append((x,cy+r*math.cos(angle),cz+r*math.sin(angle)))
        for radial in range(8):
            for chord in range(4):
                k=start+radial*5+chord;faces.append((k,k+5,k+6,k+1))
    blades=intake_mesh('HornetFanBlades'+suffix,verts,faces,fan_metal,rotor)
    # Thin blade skins are visible from both sides in the rotating assembly.
    fan_metal.use_backface_culling=False
    ring_mesh('HornetFanSpinner'+suffix,[(-9.8,.015),(-10.05,.20),(-10.7,.45),(-11.7,.64),(-12.5,.68)],cy,cz,fan_metal,rotor)
    # Fixed stator supports aft of the rotor.
    v=[];f=[]
    for j in range(12):
        a=math.tau*j/12;k=len(v)
        for x,r,da in [(-12.9,.7,-.013),(-12.9,2.86,-.013),(-13.15,2.86,.013),(-13.15,.7,.013)]:
            v.append((x,cy+r*math.cos(a+da),cz+r*math.sin(a+da)))
        f.append((k,k+1,k+2,k+3))
    intake_mesh('HornetFanStator'+suffix,v,f,fan_metal)
