"""Labeled comparisons of native captures; originals are never modified."""
from pathlib import Path
from PIL import Image,ImageDraw,ImageFont
import argparse

root=Path(__file__).resolve().parents[2]
p=argparse.ArgumentParser();p.add_argument('accepted');args=p.parse_args()
before=root/'tests/f22-bay-inlet-rebuild/mainbay-planform-close-before'
after=root/'tests/f22-bay-inlet-rebuild'/args.accepted
output=Path(__file__).parent
font=ImageFont.truetype('/System/Library/Fonts/Menlo.ttc',21)

def panel(paths,boxes,labels,size,destination):
    width,height=size
    image=Image.new('RGB',(width*len(paths),height+52),'#101d29')
    draw=ImageDraw.Draw(image)
    for i,(path,box,label) in enumerate(zip(paths,boxes,labels)):
        source=Image.open(path).convert('RGB').crop(box)
        source.thumbnail((width,height),Image.Resampling.LANCZOS)
        image.paste(source,(i*width+(width-source.width)//2,52+(height-source.height)//2))
        draw.text((i*width+18,15),label,font=font,fill='#d9edf2')
    image.save(output/destination,quality=94)

panel([before/'01-detail-plan-top.png',after/'01-detail-plan-top.png'],
      [(175,55,1275,840)]*2,['BEFORE / native top view','AFTER / native top view'],
      (800,570),'top-before-after.jpg')
panel([before/'02-detail-mainbay-under.png',after/'05-detail-mainbay-under.png'],
      [(455,115,1010,895)]*2,['BEFORE / main bay open','AFTER / main bay open'],
      (600,843),'bay-before-after.jpg')
print(output/'top-before-after.jpg');print(output/'bay-before-after.jpg')
