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
