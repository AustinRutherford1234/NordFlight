"""Approved paired 30-second render checks; restores existing game processes."""
import os,signal,subprocess,json
from pathlib import Path
root=Path(__file__).resolve().parents[2];out=root/'tests/escort-model-review';paused=[]
try:
 for pid in map(int,subprocess.run(['pgrep','-x','gripen-flight'],capture_output=True,text=True).stdout.split()):
  state=subprocess.run(['ps','-o','stat=','-p',str(pid)],capture_output=True,text=True).stdout.strip()
  if state and not state.startswith('T'):os.kill(pid,signal.SIGSTOP);paused.append(pid)
 for mode in ['reference','escorts']:
  env=dict(os.environ,NORD_CARRIER_CAPTURE='escorts-fleet',NORD_BENCH_SCENE='dry',NORD_BENCH_FULLSCREEN='1',NORD_PRESENT_MODE='vsync',NORD_BENCH_WARMUP='10',NORD_BENCH_SECONDS='30',NORD_BENCH_FPS_ONLY='1',NORD_BENCH_REPORT=str(out/(mode+'-performance.json')))
  for key in ['NORD_BENCH_SCRIPT','NORD_ESCORTS_REFERENCE','NORD_FLEET_CAPTURE','NORD_CREW_CAPTURE']:env.pop(key,None)
  if mode=='reference':env['NORD_ESCORTS_REFERENCE']='1'
  with (out/(mode+'-performance.log')).open('w') as log:
   subprocess.run([str(root/'Nord Flight.app/Contents/MacOS/gripen-flight'),'--benchmark-render','dry'],cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=140)
  print(mode,json.loads((out/(mode+'-performance.json')).read_text()),flush=True)
finally:
 for pid in paused:
  try:os.kill(pid,signal.SIGCONT)
  except ProcessLookupError:pass
 print('Original game sessions resumed.',flush=True)
