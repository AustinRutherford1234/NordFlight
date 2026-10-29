"""Read-only rail inspection in Blender; outputs a top view and deck-strip bounds.
Run: Blender --background --python scripts/nimitz/inspect_catapult.py
Coordinates are source Blender coordinates and the game's centred/scaled ship frame.
"""
import bpy
import math
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
out=ROOT/'tests/carrier-catapult';out.mkdir(parents=True,exist_ok=True)
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=str(ROOT/'assets/ships/nimitz/source.glb'))
s=333/303.842285
rows=[]
for o in bpy.data.objects:
    if o.type!='MESH':continue
    for poly in o.data.polygons:
        p=[o.matrix_world@o.data.vertices[i].co for i in poly.vertices]
        lo=[min(v[i] for v in p) for i in range(3)];hi=[max(v[i] for v in p) for i in range(3)]
        dx,dy,dz=[hi[i]-lo[i] for i in range(3)]
        if 20.8<lo[2]<22 and dz<.4 and ((dx<2 and dy>12)or(dy<2 and dx>12)or(dy>100 and dx<20)):
            rows.append({'object':o.name,'source_lo':lo,'source_hi':hi,
                         'ship_lo':[(222.12742-hi[0])*s,lo[2]*s,(51.34455+lo[1])*s],
                         'ship_hi':[(222.12742-lo[0])*s,hi[2]*s,(51.34455+hi[1])*s]})
(out/'deck-strips.json').write_text(json.dumps(rows,indent=2)+'\n')
bpy.ops.object.camera_add(location=(222.12742,-51.34455,420),rotation=(0,0,math.pi))
camera=bpy.context.object;camera.data.type='ORTHO';camera.data.ortho_scale=340
bpy.ops.object.light_add(type='SUN',rotation=(.15,.25,0));bpy.context.object.data.energy=3
scene=bpy.context.scene;scene.camera=camera;scene.render.engine='CYCLES';scene.cycles.samples=4;scene.cycles.use_denoising=True
scene.world=bpy.data.worlds.new('Inspection world');scene.world.color=(.3,.3,.3)
scene.render.resolution_x=900;scene.render.resolution_y=1800;scene.render.resolution_percentage=100
scene.render.filepath=str(out/'mesh-top.png');bpy.ops.render.render(write_still=True)
print('DECK_STRIPS',len(rows))
