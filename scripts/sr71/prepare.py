"""Shared source normalization; run directly to rebuild the historical static asset."""
import bpy, numpy as np, json,sys
from pathlib import Path
root=Path(__file__).resolve().parents[2]
def load_source():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=str(root/'assets/aircraft/sr71/source.glb'))
    objects=[o for o in bpy.context.scene.objects if o.type=='MESH']
    points=np.concatenate([np.array([tuple(o.matrix_world@v.co) for v in o.data.vertices])for o in objects])
    xy=points[:,:2];_,axes=np.linalg.eigh(np.cov(xy.T));f=axes[:,-1]
    cab=np.concatenate([np.array([tuple(o.matrix_world@v.co) for v in o.data.vertices])for o in objects if 'kabina' in o.name.lower()]).mean(0)
    if np.dot(cab[:2]-xy.mean(0),f)<0:f=-f
    forward=np.array([*f,0]);up=np.array([0,0,1]);right=np.cross(up,forward);rot=np.array([-right,forward,up]);p=points@rot.T
    scale=32.73/(p[:,1].max()-p[:,1].min());center=(p.min(0)+p.max(0))/2
    # Align tyre bottom with the existing generic contact envelope (2.0 m below origin).
    center[2]=p[:,2].min()+2.0/scale
    for o in objects:
     world=o.matrix_world.copy()
     for v in o.data.vertices:v.co=tuple(((rot@np.array(world@v.co)-center)*scale))
     o.parent=None;o.matrix_world.identity()
    # Collapse identical material instances; no geometry decimation or texture resizing.
    seen={}
    for o in objects:
     for slot in o.material_slots:
      mat=slot.material
      if mat:
       textures=tuple(sorted(n.image.name for n in mat.node_tree.nodes if n.type=='TEX_IMAGE' and n.image)) if mat.use_nodes else ()
       key=(textures,tuple(mat.diffuse_color),mat.blend_method if hasattr(mat,'blend_method')else str(mat.surface_render_method))
       if key in seen:slot.material=seen[key]
       else:seen[key]=mat
    return objects,scale,p,center

if __name__=="__main__":
    objects,scale,p,center=load_source()
    bpy.ops.object.select_all(action='DESELECT')
    for o in objects:o.select_set(True)
    bpy.context.view_layer.objects.active=objects[0];bpy.ops.object.join()
    bpy.ops.export_scene.gltf(filepath=str(root/'assets/aircraft/sr71/static.glb'),export_format='GLB',use_selection=True,export_animations=False,export_cameras=False,export_lights=False)
    q=(p-center)*scale
    report=dict(source_bytes=(root/'assets/aircraft/sr71/source.glb').stat().st_size,objects=len(objects),scale=float(scale),bounds_blender=[q.min(0).tolist(),q.max(0).tolist()],triangles=sum(len(poly.vertices)-2 for poly in bpy.context.object.data.polygons),materials=len(bpy.context.object.data.materials),animations=0)
    (root/'assets/aircraft/sr71/preparation.json').write_text(json.dumps(report,indent=2)+'\n');print(report)
