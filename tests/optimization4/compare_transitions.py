"""Consecutive-frame coverage, difference metrics and inspectable contact sheets.

Large differences are review flags, not an automatic claim of corruption: camera
cuts, wall-clock blinking lamps and live presentation may legitimately differ.
"""
import argparse
import json
from pathlib import Path
import numpy as np
from PIL import Image,ImageDraw

p=argparse.ArgumentParser(); p.add_argument('reference',type=Path);p.add_argument('candidate',type=Path)
p.add_argument('output',type=Path);args=p.parse_args();args.output.mkdir(parents=True,exist_ok=True)
rows=[]; missing=[]
for event in range(100):
    tiles=[]
    for offset in range(12):
        stem=f'transition-{event:03}-{offset:02}'
        rp=args.reference/(stem+'.png'); cp=args.candidate/(stem+'.png')
        if not rp.exists() or not cp.exists(): missing.append(stem);continue
        with Image.open(rp) as im: a=np.asarray(im.convert('RGB'),dtype=np.int16)
        with Image.open(cp) as im: b=np.asarray(im.convert('RGB'),dtype=np.int16)
        assert a.shape==b.shape==(900,1440,3)
        delta=np.abs(a-b);delta[:75,1200:]=0
        row={'transition':event,'offset':offset,'mean_abs_rgb':float(delta.mean()),
            'pixels_over_25_fraction':float(((delta[:,:,0]>25)|(delta[:,:,1]>25)|(delta[:,:,2]>25)).mean()),
            'clipped_white_fraction':float(((b[:,:,0]>=250)&(b[:,:,1]>=250)&(b[:,:,2]>=250)).mean()),
            'exact_black_fraction':float(((b[:,:,0]==0)&(b[:,:,1]==0)&(b[:,:,2]==0)).mean())}
        sa=json.loads(rp.with_suffix('.json').read_text());sb=json.loads(cp.with_suffix('.json').read_text())
        row['scene_state_equal']=all(sa[k]==sb[k] for k in ['frame','camera','map','quality','scenario','position'])
        rows.append(row)
        if offset in [0,1,2,3,4,6,8,11]:
            tile=Image.fromarray(b.astype(np.uint8)).resize((180,112))
            tiles.append((offset,tile))
    if event%10==0: sheet=Image.new('RGB',(1440,10*134),(12,18,23));draw=ImageDraw.Draw(sheet)
    for col,(offset,tile) in enumerate(tiles):
        y=(event%10)*134;sheet.paste(tile,(col*180,y+20));draw.text((col*180+4,y+3),f'cut {event:03} offset {offset}',fill='white')
    if event%10==9: sheet.save(args.output/f'contact-{event//10:02}.jpg',quality=90)
result={'compared_frames':len(rows),'missing':missing,'covered_transitions':len({r['transition'] for r in rows}),
        'unequal_scene_states':sum(not r['scene_state_equal'] for r in rows),
        'review_flags':[r for r in rows if r['pixels_over_25_fraction']>.05 or r['clipped_white_fraction']>.8],
        'mask':'Only top-right telemetry, x>=1200 y<75','frames':rows}
(args.output/'comparison.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({k:v for k,v in result.items() if k!='frames'},indent=2))
