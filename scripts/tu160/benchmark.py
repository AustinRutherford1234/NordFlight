"""Same executable, frozen same-scene fullscreen bomber A/B. No concurrent builds.
Alternating samples reduce ordering bias; always restores previously running flights.
"""
from pathlib import Path
import os,signal,subprocess,json,time,argparse
p=argparse.ArgumentParser();p.add_argument('--pair',action='store_true');a=p.parse_args()
ROOT=Path(__file__).resolve().parents[2];OUT=ROOT/('tests/bomber-performance/confirmation'if a.pair else'tests/bomber-performance');OUT.mkdir(parents=True,exist_ok=True);paused=[];results=[]
try:
 for pid in map(int,subprocess.run(['pgrep','-x','gripen-flight'],capture_output=True,text=True).stdout.split()):
  state=subprocess.run(['ps','-o','stat=','-p',str(pid)],capture_output=True,text=True).stdout.strip()
  if state and not state.startswith('T'):os.kill(pid,signal.SIGSTOP);paused.append(pid)
 for i,reference in enumerate([True,False]if a.pair else[True,False,False,True]):
  tag=f'{i}-'+('procedural'if reference else'tu160');env=dict(os.environ,NORD_NAVAL_CAPTURE='bomber-front-closed',NORD_BENCH_FULLSCREEN='1',NORD_BENCH_FPS_ONLY='1',NORD_BENCH_WARMUP='10',NORD_BENCH_SECONDS='30',NORD_BENCH_REPORT=str(OUT/(tag+'.json')))
  for key in ['NORD_NAVAL_LIVE','NORD_BOMBER_REFERENCE','NORD_FLEET_CAPTURE','NORD_CARRIER_CAPTURE']:env.pop(key,None)
  if reference:env['NORD_BOMBER_REFERENCE']='1'
  with (OUT/(tag+'.log')).open('w')as log:subprocess.run([str(ROOT/'Nord Flight.app/Contents/MacOS/gripen-flight'),'--benchmark-render','naval-live'],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT,timeout=100,check=True)
  report=json.loads((OUT/(tag+'.json')).read_text());results.append({'tag':tag,**{k:report.get(k)for k in ['resolution','paused','frames','mean_ms','p50_ms','p95_ms','p99_ms','mean_fps']}});print(json.dumps(results[-1]),flush=True)
 (OUT/'summary.json').write_text(json.dumps(results,indent=2)+'\n')
finally:
 for pid in paused:
  try:os.kill(pid,signal.SIGCONT)
  except ProcessLookupError:pass
 print('Existing sessions restored',flush=True)
