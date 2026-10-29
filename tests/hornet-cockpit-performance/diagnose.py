"""Approved sequential cockpit comparison and five-minute streaming soak."""
from pathlib import Path
import sys,os,json,hashlib,subprocess
root=Path(__file__).resolve().parents[2];out=Path(__file__).resolve().parent
sys.path.insert(0,str(root/'tests/optimization3'))
from run import isolated,run,command
binary=root/'Nord Flight.app/Contents/MacOS/gripen-flight'
(out/'identity.json').write_text(json.dumps({'sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'head':command(['git','rev-parse','HEAD']),'hardware':command(['sysctl','-n','hw.model','hw.memsize','machdep.cpu.brand_string']),'power':command(['pmset','-g','batt']),'thermal':command(['pmset','-g','therm'])},indent=2))
base={'NORD_AIRCRAFT':'fa18f','NORD_BENCH_SCENE':'terrain-forest','NORD_TERRAIN_AGL':'50','NORD_TERRAIN_SPEED':'100','NORD_BENCH_SCRIPT':'terrain-stream','NORD_BENCH_WARMUP':'8','NORD_BENCH_SECONDS':'30','NORD_BENCH_FPS_ONLY':'1','NORD_BENCH_LIVE':'0'}
# Remove capture/diagnostic flags inherited from unrelated work.
for key in list(os.environ):
 if key.startswith(('NORD_FLEET_CAPTURE','NORD_CARRIER_CAPTURE','NORD_CREW_CAPTURE','NORD_BENCH','NORD_SOAK','NORD_TERRAIN','NORD_PROFILE','NORD_CAPTURE_SEQUENCE')):os.environ.pop(key)
with isolated():
 for name,extra in [('external',{}),('cockpit',{'NORD_TERRAIN_COCKPIT':'1'}),('cockpit-live',{'NORD_TERRAIN_COCKPIT':'1','NORD_BENCH_LIVE':'1','NORD_BENCH_SCRIPT':''}),('cockpit-noglass',{'NORD_TERRAIN_COCKPIT':'1'}),('cockpit-profile',{'NORD_TERRAIN_COCKPIT':'1','NORD_BENCH_FPS_ONLY':'0','NORD_BENCH_SECONDS':'20'}),('cockpit-soak',{'NORD_TERRAIN_COCKPIT':'1','NORD_BENCH_SECONDS':'300'})]:
  env=dict(base,**extra);env['NORD_BENCH_REPORT']=str(out/(name+'.json'))
  run(binary,['--benchmark-render',name],out,name,env,timeout=400 if name.endswith('soak')else 120)
  d=json.loads((out/(name+'.json')).read_text());print(json.dumps({'case':name,'fps':d['mean_fps'],'p95':d['p95_ms'],'p99':d['p99_ms']}),flush=True)
