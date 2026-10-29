"""Fullscreen follow-up including the reported carrier approach and approved 5m soak."""
from pathlib import Path
import sys,os,json,hashlib
root=Path(__file__).resolve().parents[2];out=Path(__file__).resolve().parent/'fullscreen';out.mkdir(exist_ok=True)
sys.path.insert(0,str(root/'tests/optimization3'))
from run import isolated,run,command
binary=root/'target/debug/gripen-flight'
for key in list(os.environ):
 if key.startswith(('NORD_FLEET_CAPTURE','NORD_CARRIER_CAPTURE','NORD_CREW_CAPTURE','NORD_BENCH','NORD_SOAK','NORD_TERRAIN','NORD_PROFILE','NORD_CAPTURE_SEQUENCE')):os.environ.pop(key)
base={'NORD_AIRCRAFT':'fa18f','NORD_BENCH_SCENE':'terrain-forest','NORD_BENCH_FULLSCREEN':'1','NORD_PRESENT_MODE':'vsync','NORD_TERRAIN_AGL':'50','NORD_TERRAIN_SPEED':'100','NORD_BENCH_SCRIPT':'terrain-stream','NORD_BENCH_WARMUP':'8','NORD_BENCH_SECONDS':'20','NORD_BENCH_FPS_ONLY':'1','NORD_BENCH_LIVE':'0'}
(out/'identity.json').write_text(json.dumps({'sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'source_changes':command(['git','diff','--stat']),'hardware':command(['sysctl','-n','hw.model','hw.memsize','machdep.cpu.brand_string']),'power':command(['pmset','-g','batt'])},indent=2))
with isolated():
 for name,extra in [('terrain-external',{}),('terrain-cockpit',{'NORD_TERRAIN_COCKPIT':'1'}),('carrier-external',{'NORD_BENCH_SCRIPT':'carrier-approach'}),('carrier-cockpit',{'NORD_BENCH_SCRIPT':'carrier-approach','NORD_TERRAIN_COCKPIT':'1'}),('carrier-cockpit-noglass',{'NORD_BENCH_SCRIPT':'carrier-approach','NORD_TERRAIN_COCKPIT':'1'}),('carrier-cockpit-profile',{'NORD_BENCH_SCRIPT':'carrier-approach','NORD_TERRAIN_COCKPIT':'1','NORD_BENCH_SECONDS':'20'}),('carrier-cockpit-soak',{'NORD_BENCH_SCRIPT':'carrier-approach','NORD_TERRAIN_COCKPIT':'1','NORD_BENCH_SECONDS':'300'})]:
  env=dict(base,**extra);env['NORD_BENCH_REPORT']=str(out/(name+'.json'))
  if name.endswith('profile'):env.pop('NORD_BENCH_FPS_ONLY',None)
  run(binary,['--benchmark-render',name],out,name,env,timeout=400 if name.endswith('soak')else 120)
  d=json.loads((out/(name+'.json')).read_text());print(json.dumps({'case':name,'resolution':d['resolution'],'fps':d['mean_fps'],'p95':d['p95_ms'],'p99':d['p99_ms']}),flush=True)
