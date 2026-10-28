"""Compose labeled native evidence without modifying the source captures."""
from pathlib import Path
import argparse,json
from PIL import Image,ImageDraw,ImageFont

def main():
    p=argparse.ArgumentParser();p.add_argument("folder",type=Path);p.add_argument("--output",type=Path);p.add_argument("--columns",type=int,default=3)
    args=p.parse_args();records=json.loads((args.folder/"manifest.json").read_text())["captures"]
    width,height,label=640,400,38;columns=args.columns;rows=(len(records)+columns-1)//columns
    sheet=Image.new("RGB",(columns*width,rows*(height+label)),"#101c27");draw=ImageDraw.Draw(sheet)
    font=ImageFont.truetype("/System/Library/Fonts/Menlo.ttc",15)
    for i,r in enumerate(records):
        x=i%columns*width;y=i//columns*(height+label)
        image=Image.open(args.folder/r["file"]).convert("RGB");image.thumbnail((width,height),Image.Resampling.LANCZOS)
        sheet.paste(image,(x+(width-image.width)//2,y+(height-image.height)//2))
        draw.text((x+10,y+height+9),r.get("label",r["mode"]),font=font,fill="#cce7eb")
    output=args.output or args.folder/"contact.jpg";sheet.save(output,quality=93)
    print(output.resolve())

if __name__=="__main__":main()
