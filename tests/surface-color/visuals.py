"""Matched native surface-color captures; uses the existing isolation watchdog."""
import hashlib
import json
import shutil
import sys
import time
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tests/optimization3'))
from run import isolated,run
out=ROOT/'tests/surface-color'/(sys.argv[1] if len(sys.argv)>1 else 'optimized')
out.mkdir(exist_ok=True)
binary=ROOT/'Nord Flight.app/Contents/MacOS/gripen-flight'
with isolated():
    (out/'after-executable.json').write_text(json.dumps({'binary':str(binary),'sha256':hashlib.sha256(binary.read_bytes()).hexdigest()},indent=2))
    for label,extra in [('after',{}),('earth-low',{'NORD_TERRAIN_AGL':'50'}),('scifi',{'NORD_ENVIRONMENT':'scifi-earth'}),('mountains',{'NORD_LOADING_TRANSITION':'mountains'})]:
        env={'NORD_ENVIRONMENT':'earth','RUST_LOG':'warn,gripen_flight::terrain::cover=info'}
        env.update(extra)
        started=time.time_ns()
        run(binary,['--visual-check','terrain-forest'],out,label,env,timeout=180)
        source=ROOT/'tests/visual-terrain-forest.png'
        if source.stat().st_mtime_ns<started:raise RuntimeError('Stale screenshot')
        shutil.copy2(source,out/(label+'.png'))
