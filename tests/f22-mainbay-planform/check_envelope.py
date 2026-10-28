"""Inspect exported authored cross sections, without approximating screenshots."""
import json
from pathlib import Path
import numpy as np

data=json.loads((Path(__file__).parent/'duct-geometry.json').read_text())[::2]
inner=np.array([s['inner'] for s in data]);outer=np.array([s['outer'] for s in data])
planes=[]
tetrahedra=[[0,1,2,6],[0,2,3,6],[0,3,7,6],[0,7,4,6],[0,4,5,6],[0,5,1,6]]
walls=[[0,4,5,1],[1,5,6,2],[3,2,6,7],[0,3,7,4]]
for j in range(64):
    v=np.concatenate([inner[j],inner[j+1]])
    for ids in tetrahedra:
        p=v[ids];center=p.mean(axis=0);group=[]
        for face in [[0,1,2],[0,3,1],[0,2,3],[1,3,2]]:
            a,b,c=p[face];n=np.cross(b-a,c-a);n/=np.linalg.norm(n)
            if n@(center-a)<0:n=-n
            wall=any(all(ids[k] in w for k in face) for w in walls)
            group.append([*n,n@a+(0.0002 if wall else 0.)])
        planes.append(group)
planes=np.array(planes)
smooth=lambda t:(lambda t:t*t*(3-2*t))(min(1,max(0,t)))
for thickness in [.012,.018,.024]:
    perimeter=[]
    for j,s in enumerate(data):
        q=outer[j].copy();shell=.012*smooth(s['u']/.1)
        q[0,0]-=shell;q[2]+=[shell,-shell,0];q[3]+=[-shell,-shell,0]
        q[0,1]+=thickness*smooth(s['u']/.1)
        crest=inner[j,1].copy();crest[1]+=thickness
        crest=q[1]*(1-smooth(s['u']/.1))+crest*smooth(s['u']/.1)
        crest[1]=max(crest[1],inner[j,1,1]);perimeter.append([q[0],crest,q[1],q[2],q[3]])
    p=np.array(perimeter)
    hits=[]
    for j in range(64):
        for k in range(5):
            q=np.array([p[j,k],p[j+1,k],p[j+1,(k+1)%5],p[j,(k+1)%5]])
            for face in [[0,1,2],[0,2,3]]:
                c=q[face].mean(axis=0)
                inside=np.all(planes[:,:,:3]@c>planes[:,:,3]+1e-5,axis=1)
                if inside.any():hits.append({'slice':j,'panel':k,'center':c.tolist()})
    print(thickness,hits)
