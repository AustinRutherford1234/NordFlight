"""Approved 30-second paired fullscreen carrier checks, with identical counters."""
from pathlib import Path
import os,sys,json,hashlib
root=Path(__file__).resolve().parents[2];folder=Path(__file__).resolve().parent/'paired'
sys.path.insert(0,str(root/'tests/optimization3'))
from run import isolated,run
label=sys.argv[1];binary=folder/'before' if label=='before'else root/'target/debug/gripen-flight'
for key in list(os.environ):
 if key.startswith(('NORD_FLEET_CAPTURE','NORD_CARRIER_CAPTURE','NORD_CREW_CAPTURE','NORD_BENCH','NORD_SOAK','NORD_TERRAIN','NORD_PROFILE','NORD_CAPTURE_SEQUENCE')):os.environ.pop(key)
env={'NORD_AIRCRAFT':'fa18f','NORD_BENCH_SCENE':'terrain-forest','NORD_BENCH_FULLSCREEN':'1','NORD_PRESENT_MODE':'vsync','NORD_TERRAIN_AGL':'50','NORD_TERRAIN_COCKPIT':'1','NORD_BENCH_SCRIPT':'carrier-approach','NORD_BENCH_WARMUP':'10','NORD_BENCH_SECONDS':'30','NORD_BENCH_LIVE':'0','NORD_BENCH_REPORT':str(folder/(label+'.json'))}
(folder/(label+'-identity.json')).write_text(json.dumps({'sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'env':env},indent=2))
with isolated():run(binary,['--benchmark-render',label],folder,label,env,timeout=120)
