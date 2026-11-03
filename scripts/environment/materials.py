#!/usr/bin/env python3
"""Download CC0 material sources and build mipmapped array payloads for the game."""
import hashlib,json,urllib.request
from pathlib import Path
import numpy as np
from PIL import Image
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'assets/environment/materials';OUT.mkdir(parents=True,exist_ok=True)
ASSETS=['aerial_grass_rock','brown_mud_leaves_01','rock_boulder_cracked','gravel_floor_02','aerial_beach_01','snow_02']
manifest=[];albedo=bytearray();normal=bytearray()
for index,name in enumerate(ASSETS):
    maps={}
    for kind in ['diff','nor_gl','rough']:
        url=f'https://dl.polyhaven.org/file/ph-assets/Textures/jpg/1k/{name}/{name}_{kind}_1k.jpg'
        path=OUT/f'{name}_{kind}.jpg'
        try:
            if not path.exists():urllib.request.urlretrieve(url,path)
            maps[kind]=Image.open(path).convert('RGB').resize((512,512),Image.Resampling.LANCZOS)
            manifest.append(dict(asset=name,map=kind,source=url,license='CC0',sha256=hashlib.sha256(path.read_bytes()).hexdigest()))
        except Exception as error:
            print(name,kind,error,flush=True)
    if 'diff' not in maps:
        raise SystemExit(f'Missing required material {name}')
    a=np.asarray(maps['diff']).astype(float)/255.
    # Store local detail relative to its mean, so macro land-cover palette owns
    # distant color and sci-fi tints; preserve RGB relationships of real samples.
    mean=a.mean(axis=(0,1)).clip(.03,1.)
    a=np.clip(a/mean*.5,0,1)
    rgb=Image.fromarray(np.uint8(a*255),'RGB').convert('RGBA')
    n=maps.get('nor_gl',Image.new('RGB',(512,512),(128,128,255))).convert('RGBA')
    n.putalpha(maps.get('rough',Image.new('L',(512,512),230)).convert('L'))
    for image,payload in [(rgb,albedo),(n,normal)]:
        for level in range(10):payload.extend(image.resize((max(1,512>>level),)*2,Image.Resampling.BOX).tobytes())
    print(name,'prepared',flush=True)
(OUT/'albedo.bin').write_bytes(albedo);(OUT/'normal-roughness.bin').write_bytes(normal)
(OUT/'manifest.json').write_text(json.dumps(dict(version=1,size=512,layers=ASSETS,mips=10,sources=manifest),indent=2)+'\n')
# DDS codec supplies portable BC3 blocks; strip each single-level DDS header.
# Keep the RGBA payloads for adapters without BC texture support.
import io
for name,payload in [('albedo',albedo),('normal-roughness',normal)]:
    blocks=bytearray();offset=0
    for layer in range(6):
        for level in range(10):
            size=max(1,512>>level);length=size*size*4
            image=Image.frombytes('RGBA',(size,size),bytes(payload[offset:offset+length]));offset+=length
            buf=io.BytesIO();image.save(buf,format='DDS',pixel_format='DXT5');blocks.extend(buf.getvalue()[128:])
    (OUT/(name+'-bc3.bin')).write_bytes(blocks)

# The fixed-size menu planet does not need the flight globe's 8K allocation.
Image.open(ROOT/'assets/earth.jpg').convert('RGB').resize((2048,1024),Image.Resampling.LANCZOS).save(ROOT/'assets/environment/menu-earth.jpg',quality=95)
