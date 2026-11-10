"""Cockpit target diamond with general enemy icons off and HUD caged."""
import hashlib,json,shutil,sys,time
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];sys.path.insert(0,str(ROOT/'tests/optimization3'))
from run import isolated,run
out=ROOT/'tests/cockpit-diamond'/sys.argv[1];out.mkdir(parents=True,exist_ok=True)
binary=ROOT/'Nord Flight.app/Contents/MacOS/gripen-flight'
(out/'executable.json').write_text(json.dumps({'sha256':hashlib.sha256(binary.read_bytes()).hexdigest()},indent=2))
with isolated():
 for label,aircraft,mode in [('external','gripen','gun'),('gripen','gripen','gun-cockpit'),('hornet-caged-gun','fa18f','gun-caged'),('hornet-caged-air','fa18f','gun-caged-air'),('hornet-caged-far','fa18f','gun-caged-far')]:
  env={'NORD_ENVIRONMENT':'earth','NORD_RENDER_SCALE':'0.5','NORD_TARGETING_CAPTURE':mode,'NORD_AIRCRAFT':aircraft,'RUST_LOG':'warn'}
  stamp=time.time_ns();run(binary,['--visual-check','terrain-mountain-combat'],out,label,env,timeout=180)
  source=ROOT/'tests/visual-terrain-mountain-combat.png'
  if source.stat().st_mtime_ns<stamp:raise RuntimeError('Stale capture')
  shutil.copy2(source,out/(label+'.png'))
