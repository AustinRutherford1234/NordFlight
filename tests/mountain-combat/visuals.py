"""Isolated packaged-app captures for the Mountain Flying tab and combat scene."""
import hashlib,json,shutil,sys,time
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tests/optimization3'))
from run import isolated,run
out=ROOT/'tests/mountain-combat'/('native' if len(sys.argv)<2 else sys.argv[1]);out.mkdir(parents=True,exist_ok=True)
binary=ROOT/'Nord Flight.app/Contents/MacOS/gripen-flight'
with isolated():
    (out/'executable.json').write_text(json.dumps({'binary':str(binary),'sha256':hashlib.sha256(binary.read_bytes()).hexdigest()},indent=2))
    for label,mode,extra in [('menu-f22','terrain-menu-mountain-f22',{'NORD_MENU_CAPTURE':'mountain'}),('menu-hornet','terrain-menu-mountain-hornet',{'NORD_MENU_CAPTURE':'mountain-hornet'}),('combat','terrain-mountain-combat',{})]:
        env={'NORD_ENVIRONMENT':'earth','RUST_LOG':'warn'};env.update(extra)
        start=time.time_ns();run(binary,['--visual-check',mode],out,label,env,timeout=180)
        source=ROOT/'tests'/('visual-'+mode+'.png')
        if source.stat().st_mtime_ns<start:raise RuntimeError('Stale screenshot')
        shutil.copy2(source,out/(label+'.png'))
