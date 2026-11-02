"""GLB bounds, critical-part preservation, glazing and budget regressions."""
import json,struct,math
from pathlib import Path
root=Path(__file__).resolve().parents[2]/'assets/aircraft/su27';counts=[]
for i in range(3):
 b=(root/f'lod{i}.glb').read_bytes();magic,version,size=struct.unpack_from('<III',b)
 assert magic==0x46546c67 and version==2 and size==len(b)
 n=struct.unpack_from('<I',b,12)[0];j=json.loads(b[20:20+n]);binary=28+n
 assert not j.get('animations')
 def values(index):
  a=j['accessors'][index];v=j['bufferViews'][a['bufferView']];fmt={5121:'B',5123:'H',5125:'I',5126:'f'}[a['componentType']];width={'SCALAR':1,'VEC3':3}[a['type']];size=struct.calcsize(fmt)*width;stride=v.get('byteStride',size);start=binary+v.get('byteOffset',0)+a.get('byteOffset',0)
  return [struct.unpack_from('<'+fmt*width,b,start+k*stride)for k in range(a['count'])]
 parts={}
 for node in j['nodes']:
  if 'mesh'not in node:continue
  assert not any(k in node for k in ['matrix','rotation','scale','translation'])
  ps=[p for p in j['meshes'][node['mesh']]['primitives']];pts=[q for p in ps for q in values(p['attributes']['POSITION'])]
  assert all(math.isfinite(x)for p in pts for x in p)
  parts[node['name']]=(pts,sum(j['accessors'][p['indices']]['count']//3 for p in ps))
  for p in ps:
   ids=values(p['indices']);assert max(k[0]for k in ids)<j['accessors'][p['attributes']['POSITION']]['count']
 def points(prefix):return [p for name,(pts,_)in parts.items()if name.startswith(prefix)for p in pts]
 def bounds(ps):return ([min(p[k]for p in ps)for k in range(3)],[max(p[k]for p in ps)for k in range(3)])
 allpts=[p for pts,_ in parts.values()for p in pts];lo,hi=bounds(allpts)
 assert 21.5<hi[2]-lo[2]<22.5 and 13.5<hi[0]-lo[0]<15.5,(i,lo,hi)
 nose=points('Su27_radome');frame=points('Su27_canopy_frame');glass=points('Su27_glazing_glass');wind=points('Su27_glazing_windshield');seat=points('Su27_seat')
 assert all([nose,frame,glass,wind,seat])
 nl,nh=bounds(nose);gl,gh=bounds(glass);wl,wh=bounds(wind);sl,sh=bounds(seat)
 assert nl[2]<-10.8 and -7.7<nh[2]<-7.3 and nh[0]-nl[0]>1.1,(i,nl,nh)
 assert gh[1]>.95 and gl[2]<-5.7 and gh[2]>-4.0,(i,gl,gh)
 assert sh[1]<gh[1]-.05,(i,sh,gh)
 # The restored canopy and windshield must overlap longitudinally, with
 # forward/rear glazing roof points matching within a small seam envelope.
 forward=[p for p in glass if p[2]<-5.4]
 rear=[p for p in wind if p[2]>-6.2]
 assert forward and rear
 distance=min(math.dist(a,c)for a in forward for c in rear)
 assert distance<.16,(i,'canopy/windshield gap',distance)
 mats=[m for m in j['materials']if m['name'].startswith('Su27Glazing')];assert len(mats)==1
 m=mats[0];assert m['alphaMode']=='BLEND' and m['doubleSided']
 assert .55<m['pbrMetallicRoughness']['baseColorFactor'][3]<.75
 count=sum(c for _,c in parts.values());counts.append(count)
 assert count<=[8000,4500,1500][i],(i,count)
 print('LOD',i,'triangles',count,'parts',len(parts),'canopy/windshield nearest m',round(distance,4),'bounds',lo,hi)
assert counts[0]>counts[1]>counts[2]
assert (root/'ATTRIBUTION.txt').is_file()
print('All three LODs retain separate fitted radome, canopy frame, main glazing and windshield.')
