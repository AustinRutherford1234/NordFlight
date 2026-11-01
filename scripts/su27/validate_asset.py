"""Headless GLB integrity and coordinate-envelope check; no third-party modules."""
import json,struct,math
from pathlib import Path
root=Path(__file__).resolve().parents[2]/'assets/aircraft/su27'
counts=[]
for i in range(3):
 b=(root/f'lod{i}.glb').read_bytes();magic,version,size=struct.unpack_from('<III',b)
 assert magic==0x46546c67 and version==2 and size==len(b)
 n=struct.unpack_from('<I',b,12)[0];j=json.loads(b[20:20+n]);assert not j.get('animations')
 positions=[j['accessors'][p['attributes']['POSITION']] for m in j['meshes'] for p in m['primitives']]
 lo=[min(a['min'][k] for a in positions)for k in range(3)];hi=[max(a['max'][k]for a in positions)for k in range(3)]
 assert all(math.isfinite(v)for v in lo+hi)
 assert 20<hi[2]-lo[2]<24,(lo,hi)
 assert 12<hi[0]-lo[0]<18,(lo,hi)
 counts.append(sum(j['accessors'][p['indices']]['count']//3 for m in j['meshes']for p in m['primitives']))
 print('LOD',i,'triangles',counts[-1],'bounds',lo,hi)
assert counts[0]>counts[1]>counts[2]
assert (root/'ATTRIBUTION.txt').is_file()
# Every distance mesh must retain an outward-facing radome, not just its bounds.
assert all(c<=budget for c,budget in zip(counts,[8000,3000,1000])),counts
for i in range(3):
 b=(root/f'lod{i}.glb').read_bytes();n=struct.unpack_from('<I',b,12)[0];j=json.loads(b[20:20+n]);binary=20+n+8
 def values(index):
  a=j['accessors'][index];v=j['bufferViews'][a['bufferView']];fmt={5123:'H',5125:'I',5126:'f'}[a['componentType']];width={'SCALAR':1,'VEC3':3}[a['type']];size=struct.calcsize(fmt)*width;stride=v.get('byteStride',size);start=binary+v.get('byteOffset',0)+a.get('byteOffset',0)
  return [struct.unpack_from('<'+fmt*width,b,start+k*stride)for k in range(a['count'])]
 outward=0;inward=0
 for m in j['meshes']:
  for p in m['primitives']:
   if not j['materials'][p['material']].get('name','').startswith('RadomeGray'):continue
   pos=values(p['attributes']['POSITION']);idx=[v[0]for v in values(p['indices'])]
   for k in range(0,len(idx),3):
    a,c,d=[pos[v]for v in idx[k:k+3]];u=[c[t]-a[t]for t in range(3)];v=[d[t]-a[t]for t in range(3)];normal=[u[1]*v[2]-u[2]*v[1],u[2]*v[0]-u[0]*v[2]];centre=[sum(q[t]for q in [a,c,d])/3 for t in range(2)];dot=normal[0]*(centre[0]-.015*21.9/22.967)+normal[1]*(centre[1]+.35*21.9/22.967)
    outward+=dot>1e-8;inward+=dot< -1e-8
 assert outward>5 and inward==0,(i,outward,inward)
 print('LOD',i,'outward radome faces',outward)
