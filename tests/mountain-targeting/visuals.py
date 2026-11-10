"""Native mountain enemy cues, tactical map and cannon director captures."""
import hashlib,json,shutil,sys,time
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tests/optimization3'))
from run import isolated,run
out=ROOT/'tests/mountain-targeting'/sys.argv[1];out.mkdir(parents=True,exist_ok=True)
binary=Path(sys.argv[2]).resolve()
(out/'executable.json').write_text(json.dumps({'path':str(binary),'sha256':hashlib.sha256(binary.read_bytes()).hexdigest()},indent=2))
with isolated():
 for label,mode,extra in [('merge','merge',{}),('tail','tail',{}),('map','map',{}),('gun','gun',{}),('gun-native-scale','gun',{'NORD_RENDER_SCALE':'1'}),('hornet-cockpit','gun-cockpit',{'NORD_AIRCRAFT':'fa18f'})]:
  if len(sys.argv)>3 and label not in sys.argv[3].split(','):continue
  env={'NORD_RENDER_SCALE':'0.5','NORD_ENVIRONMENT':'earth','NORD_TARGETING_CAPTURE':mode,'RUST_LOG':'warn'};env.update(extra)
  stamp=time.time_ns();run(binary,['--visual-check','terrain-mountain-combat'],out,label,env,timeout=180)
  source=ROOT/'tests/visual-terrain-mountain-combat.png'
  if source.stat().st_mtime_ns<stamp:raise RuntimeError('Stale screenshot')
  shutil.copy2(source,out/(label+'.png'))
