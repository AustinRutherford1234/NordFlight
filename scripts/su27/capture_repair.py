"""Approved <=2-minute Su-27 close-up review; always resume the user's flight."""
from pathlib import Path
import os,signal,subprocess,shutil,time,json
root=Path(__file__).resolve().parents[2];out=root/'tests/su27-repair/native';out.mkdir(parents=True,exist_ok=True)
start=time.monotonic();paused=[];captures=[]
try:
 for pid in map(int,subprocess.run(['pgrep','-x','gripen-flight'],capture_output=True,text=True).stdout.split()):
  state=subprocess.run(['ps','-o','stat=','-p',str(pid)],capture_output=True,text=True).stdout.strip()
  if state and not state.startswith('T'):os.kill(pid,signal.SIGSTOP);paused.append(pid)
 for tag in ['right','left','nose','canopy']:
  remaining=115-(time.monotonic()-start)
  if remaining<15:raise TimeoutError('Native review budget exhausted')
  env=dict(os.environ,NORD_AIRCRAFT='fa18f',NORD_NAVAL_CAPTURE='fighter-inspect-'+tag)
  for k in ['NORD_BENCH_SCRIPT','NORD_BENCH_SCENE','NORD_BENCH_SOAK','NORD_BENCH_REPORT','NORD_NAVAL_LIVE','NORD_TAXI_CAPTURE','NORD_FLEET_CAPTURE','NORD_CARRIER_CAPTURE','NORD_CREW_CAPTURE','NORD_RADAR_CAPTURE']:env.pop(k,None)
  shot=root/'tests/visual-dry.png';shot.unlink(missing_ok=True)
  with (out/(tag+'.log')).open('w') as log:
   subprocess.run([str(root/'Nord Flight.app/Contents/MacOS/gripen-flight'),'--visual-check','dry'],cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT,timeout=min(50,remaining),check=True)
  shutil.copyfile(shot,out/(tag+'.png'));captures.append(tag);print('Captured',tag,flush=True)
finally:
 for pid in paused:
  try:os.kill(pid,signal.SIGCONT)
  except ProcessLookupError:pass
 elapsed=time.monotonic()-start;(out/'review.json').write_text(json.dumps({'seconds':elapsed,'captures':captures,'restoration_handler_completed':True},indent=2)+'\n')
 print('Existing flights restored; seconds',round(elapsed,2),flush=True)
