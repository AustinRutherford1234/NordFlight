#!/usr/bin/env python3
"""Sequential deterministic environment screenshots with process recovery."""
import argparse,os,subprocess,sys,shutil,json,hashlib
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tests/optimization3'));from run import isolated,record_child
p=argparse.ArgumentParser();p.add_argument('binary',type=Path);p.add_argument('output',type=Path);p.add_argument('--quick',action='store_true');p.add_argument('--case',action='append');a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True)
cases=[('forest','earth','-8250','-1100','160'),('lake','earth','10000','0','160'),('mountain','earth','292700','301150','500'),('scifi','scifi-earth','292700','301150','500'),('forest-dense','earth','22810.2937493','85674.9951183','100')]
if not a.quick:cases += [('ground','earth','0','-1500','5'),('high','earth','292700','301150','5000'),('cockpit','earth','-8250','-1100','160'),('orbit','earth','0','0','220000'),('orbit-scifi','scifi-earth','0','0','220000'),('scifi-high','scifi-earth','292700','301150','500')]
if a.case:
    missing=set(a.case)-{case[0] for case in cases}
    if missing:p.error(f'unknown capture cases: {sorted(missing)}')
    cases=[case for case in cases if case[0] in a.case]
with isolated():
 for name,world,x,z,agl in cases:
    env=os.environ.copy();env.update(NORD_ENVIRONMENT=world,NORD_RENDER_SCALE='0.5',NORD_BENCH_FULLSCREEN='1',NORD_TERRAIN_X=x,NORD_TERRAIN_Z=z,NORD_TERRAIN_AGL=agl)
    if name=='cockpit':env['NORD_TERRAIN_COCKPIT']='1'
    if name.endswith('-high'):env.update(NORD_GRAPHICS='high',NORD_RENDER_SCALE='1')
    mode='earth-flight-220' if name.startswith('orbit') else 'terrain-forest'
    with (a.output/(name+'.log')).open('w')as log:
        child=subprocess.Popen([str(a.binary.resolve()),'--visual-check',mode],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT);record_child(child.pid)
        try:
            code=child.wait(timeout=120)
            if code:raise SystemExit(f'{name}: exit {code}; see {log.name}')
            shutil.copy2(ROOT/f'tests/visual-{mode}.png',a.output/(name+'.png'))
        finally:
            if child.poll()is None:child.terminate();child.wait(timeout=10)
            record_child(None)
    print(name,flush=True)
(a.output/'executable.json').write_text(json.dumps(dict(path=str(a.binary.resolve()),sha256=hashlib.sha256(a.binary.read_bytes()).hexdigest()),indent=2)+'\n')
