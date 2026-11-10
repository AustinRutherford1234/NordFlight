"""Sequential loading A/B and unchanged flight-streaming checks on one release."""
import hashlib,json,shutil,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tests/optimization3'))
from run import isolated,run
out=ROOT/'tests/loading-speed'/sys.argv[1];out.mkdir(parents=True,exist_ok=True)
binary=Path(sys.argv[2]).resolve()
(out/'executable.json').write_text(json.dumps({'path':str(binary),'sha256':hashlib.sha256(binary.read_bytes()).hexdigest()},indent=2))
with isolated():
 for label,extra in [('serial-1',{'NORD_LOADING_SERIAL':'1'}),('optimized-1',{}),('optimized-2',{}),('serial-2',{'NORD_LOADING_SERIAL':'1'})]:
  env={'NORD_ENVIRONMENT':'earth','NORD_LOADING_TRANSITION':'mountains','NORD_LOADING_REPORT':str(out/(label+'.json')),'RUST_LOG':'warn,gripen_flight::loading=info'};env.update(extra)
  run(binary,['--visual-check','terrain-forest'],out,label,env,timeout=180)
  shutil.copy2(ROOT/'tests/visual-terrain-forest.png',out/(label+'.png'))
 for label,extra in [('stream-reference',{'NORD_LOADING_SERIAL':'1'}),('stream-optimized',{})]:
  env={'NORD_ENVIRONMENT':'earth','NORD_BENCH_SCENE':'terrain-forest','NORD_BENCH_SCRIPT':'terrain-stream','NORD_TERRAIN_AGL':'260','NORD_TERRAIN_SPEED':'230','NORD_BENCH_WARMUP':'20','NORD_BENCH_SECONDS':'30','NORD_BENCH_REPORT':str(out/(label+'.json')),'RUST_LOG':'warn'};env.update(extra)
  run(binary,['--benchmark-render','live'],out,label,env,timeout=180)
