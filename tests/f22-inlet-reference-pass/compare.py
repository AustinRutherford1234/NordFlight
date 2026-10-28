"""Compose supplied references and native captures; plot exported real duct stations."""
from pathlib import Path
import argparse,json
from PIL import Image,ImageDraw,ImageFont

ROOT=Path(__file__).resolve().parents[2]
OUT=Path(__file__).parent

def comparisons(folder):
    font=ImageFont.truetype('/System/Library/Fonts/Menlo.ttc',18)
    pairs=[(1,'detail-reference-1',(350,120,1090,740)),
           (3,'detail-reference-3',(220,200,1370,900)),
           (4,'detail-reference-4',(0,170,1100,900))]
    manifest=json.loads((folder/'manifest.json').read_text())
    files={r['mode'].split(':')[0]:folder/r['file'] for r in manifest['captures']}
    sheet=Image.new('RGB',(1320,1560),'#101c27');draw=ImageDraw.Draw(sheet)
    for row,(number,mode,crop) in enumerate(pairs):
        for column,path in enumerate([OUT/f'reference-{number}.png',files[mode]]):
            im=Image.open(path).convert('RGB')
            if column:im=im.crop(crop)
            im.thumbnail((640,470),Image.Resampling.LANCZOS)
            x=column*660;y=row*520
            sheet.paste(im,(x+(660-im.width)//2,y+40+(470-im.height)//2))
            draw.text((x+12,y+10),f'Reference {number}' if not column else 'Native | matched viewing direction',font=font,fill='#cce7eb')
    sheet.save(OUT/'photo-comparisons.jpg',quality=95)

def routing():
    import numpy as np
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    from matplotlib.patches import Rectangle
    samples=json.loads((OUT/'duct-geometry.json').read_text())
    inner=np.array([s['inner'] for s in samples]);outer=np.array([s['outer'] for s in samples]);center=inner.mean(axis=1)
    with plt.style.context('dark_background'):
        fig,axes=plt.subplots(2,1,figsize=(12,7.5),layout='constrained')
        fig.patch.set_facecolor('#101c27')
        for ax in axes:
            ax.set_facecolor('#101c27');ax.grid(alpha=.18);ax.set_xlim(-4.5,5.3);ax.set_xlabel('Aircraft longitudinal coordinate (m; aft →)')
        for k in range(4):
            axes[0].plot(inner[:,k,2],inner[:,k,1],color='#8bc7e6',lw=1.3,alpha=.9)
            axes[1].plot(inner[:,k,2],inner[:,k,0],color='#8bc7e6',lw=1.3,alpha=.9)
        axes[0].plot(center[:,2],center[:,1],color='#ffd077',lw=2,label='Actual duct station center')
        axes[1].plot(center[:,2],center[:,0],color='#ffd077',lw=2)
        axes[1].plot(outer[:,1,2],outer[:,1,0],color='#e7f0f5',ls='--',lw=1.8,label='Exterior shoulder toward wing root')
        axes[0].add_patch(Rectangle((-1.75,-1.1),3.5,.82,facecolor='#868e9d',alpha=.18))
        axes[0].text(-.1,-.9,'Main bay region',ha='center',color='#c5cbd5')
        axes[1].add_patch(Rectangle((-2.32,1.48),3.33,.7,facecolor='#c68ba5',alpha=.18))
        axes[1].text(-.65,2.12,'Repositioned side bay',ha='center',color='#efc4d8')
        for ax in axes:ax.axvspan(center[0,2],center[10,2],color='#ffd077',alpha=.07)
        axes[0].annotate('Aft run before vertical bend',xy=(center[9,2],center[9,1]),xytext=(-2.4,.78),arrowprops={'arrowstyle':'->','color':'#ffd077'},color='#ffd077')
        axes[0].set_ylabel('Height (m)');axes[1].set_ylabel('Right of centerline (m)')
        axes[0].set_ylim(-1.2,1.05);axes[1].set_ylim(.15,2.5)
        axes[0].legend(loc='upper right');axes[1].legend(loc='lower right')
        fig.suptitle('Actual model geometry: aft-running entrance, then inboard/upward S-bend\nPublic-reference visual approximation; plotted boundaries are exported cross-section stations',fontsize=13)
        fig.savefig(OUT/'duct-routing.png',dpi=170)
        plt.close(fig)

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--captures',type=Path);args=p.parse_args()
    routing()
    if args.captures:comparisons(args.captures)
