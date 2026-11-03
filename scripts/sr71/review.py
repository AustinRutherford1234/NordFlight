"""Standalone Blender review; pose contract mirrors the lightweight Bevy rig driver."""
import bpy,math,json,sys,argparse
from pathlib import Path
from mathutils import Vector,Quaternion
ROOT=Path(__file__).resolve().parents[2];OUT=ROOT/'tests/sr71-rig';OUT.mkdir(parents=True,exist_ok=True)
def B(v):return Vector((v[0],-v[2],v[1]))
def G(v):return Vector((v[0],v[2],-v[1]))
def smooth(v):v=max(0,min(1,v));return v*v*(3-2*v)
def load():
 bpy.ops.wm.open_mainfile(filepath=str(ROOT/'assets/aircraft/sr71/sr71-editable.blend'))
 return json.loads((ROOT/'assets/aircraft/sr71/rig.json').read_text())
def pose(rig,e,d,pitch=0,roll=0,yaw=0,mach=0,spool=.3):
 for p in rig['pivots']:
  o=bpy.data.objects[p['name']];kind=p['kind'];angle=0.
  if kind=='gear':angle=p['angle']*smooth(1-e)
  if kind=='door':angle=p['angle']*max(d,e)
  if kind=='elevon':angle=(-pitch+(1 if p['index']==0 else -1)*roll)*.35
  if kind=='fin':angle=-yaw*.3
  if kind=='canopy':angle=p['angle']*(1 if '--canopy' in sys.argv else 0)
  o.rotation_mode='QUATERNION';o.rotation_quaternion=Quaternion(B(p['axis']).normalized(),angle)
  if kind=='spike':o.location=B(p['rest_translation'])+B((0,0,1))*.66*smooth((mach-1.6)/1.6)
  if kind=='nozzle':o.scale=(1+spool*.05,1,1+spool*.05)
 bpy.context.view_layer.update()
 for link in rig['links']:
  a=Vector(link['base']);parent=bpy.data.objects[link['parent']];b=G(parent.matrix_world@B(link['local']));v=b-a;length=v.length;direction=v.normalized();barrel=min(link['length'],length*.85)
  for name,start,extent,r in [(link['barrel'],a,barrel,link['radius']),(link['rod'],a+direction*(barrel-.045),max(.001,length-barrel+.045),link['radius']*.52)]:
   o=bpy.data.objects[name];o.location=B(start);o.rotation_mode='QUATERNION';o.rotation_quaternion=Vector((0,0,1)).rotation_difference(B(direction));o.scale=(r,r,extent)
 bpy.context.view_layer.update()
def studio():
 scene=bpy.context.scene;scene.render.engine='CYCLES';scene.cycles.samples=16;scene.cycles.use_denoising=True
 scene.render.resolution_x=1200;scene.render.resolution_y=900;scene.render.resolution_percentage=100
 scene.world=bpy.data.worlds.new('SR71 Review Studio');scene.world.use_nodes=True;scene.world.node_tree.nodes['Background'].inputs[0].default_value=(.35,.39,.45,1);scene.world.node_tree.nodes['Background'].inputs[1].default_value=.45
 for pos,power,size in [((6,0,10),3500,10),((-7,-3,8),4500,10),((0,-3,-7),2500,6)]:
  bpy.ops.object.light_add(type='AREA',location=pos);o=bpy.context.object;o.data.energy=power;o.data.shape='DISK';o.data.size=size;o.rotation_euler=(Vector((0,0,0))-o.location).to_track_quat('-Z','Y').to_euler()
 bpy.ops.object.camera_add();cam=bpy.context.object;scene.camera=cam;cam.data.type='ORTHO'
 return scene,cam
def camera(cam,mode):
 positions={'main':((0,-6.27,-.3),(6,-9,-4),7.5),'main-close':((2.0,-6.3,-.25),(5.5,-7.4,-2),4.0),'nose':((0,5.9,-.5),(3,8,-3),4.3),'belly':((0,0,0),(0,0,-28),35),'top':((0,0,0),(10,-18,24),38),'rear':((0,-10,1),(10,-25,7),24)}
 at,eye,scale=positions[mode];cam.location=eye;cam.rotation_euler=(Vector(at)-cam.location).to_track_quat('-Z','Y').to_euler();cam.data.ortho_scale=scale
if __name__=='__main__':
 args=sys.argv[sys.argv.index('--')+1:]if'--'in sys.argv else[]
 parser=argparse.ArgumentParser();parser.add_argument('--states',nargs='+',default=['down','half','up']);parser.add_argument('--views',nargs='+',default=['main','nose']);parser.add_argument('--save',action='store_true');parser.add_argument('--canopy',action='store_true');parser.add_argument('--pitch',type=float,default=0.);parser.add_argument('--roll',type=float,default=0.);parser.add_argument('--yaw',type=float,default=0.);p=parser.parse_args(args)
 rig=load();scene,cam=studio()
 for state in p.states:
  e,d={'down':(1,0),'half':(.5,1),'up':(0,0),'doors':(0,1),'quarter':(.25,1),'threequarter':(.75,1)}[state];pose(rig,e,d,pitch=p.pitch,roll=p.roll,yaw=p.yaw)
  for view in p.views:
   camera(cam,view);scene.render.filepath=str(OUT/f'rig-{"canopy-"if p.canopy else"controls-"if p.pitch or p.roll or p.yaw else""}{state}-{view}.png');bpy.ops.render.render(write_still=True)
 if p.save:
  # Timeline: closed/up -> doors open -> legs down -> down -> legs up -> doors closed.
  for frame in range(1,177):
   if frame<=16:e,d=0.,(frame-1)/15.
   elif frame<=76:e,d=(frame-16)/60.,1.
   elif frame<=100:e,d=1.,1.
   elif frame<=160:e,d=1.-(frame-100)/60.,1.
   else:e,d=0.,1.-(frame-160)/16.
   pose(rig,e,d)
   for n in rig['pivots']:
    o=bpy.data.objects[n['name']];o.keyframe_insert('rotation_quaternion',frame=frame);o.keyframe_insert('location',frame=frame);o.keyframe_insert('scale',frame=frame)
   for n in rig['links']:
    for name in [n['barrel'],n['rod']]:
     o=bpy.data.objects[name];o.keyframe_insert('rotation_quaternion',frame=frame);o.keyframe_insert('location',frame=frame);o.keyframe_insert('scale',frame=frame)
  scene.frame_start=1;scene.frame_end=176;scene.render.fps=24;scene.frame_set(100);camera(cam,'main')
  bpy.ops.wm.save_as_mainfile(filepath=str(OUT/'sr71-animation-review.blend'))
