"""User-approved, aggregate <=3-minute native review; always restores flights.
Re-runs share a budget file so repair checks cannot silently extend the window.
"""
from pathlib import Path
import os,signal,subprocess,shutil,argparse,time,json
ROOT=Path(__file__).resolve().parents[2];OUT=ROOT/'tests/raid-flight-review';OUT.mkdir(parents=True,exist_ok=True)
p=argparse.ArgumentParser();p.add_argument('--modes',nargs='+',default=['flight-model-bomber','flight-model-fighter','fighter-live','bomber-live']);a=p.parse_args()
budget=OUT/'review-budget.json';used=json.loads(budget.read_text()).get('seconds',0.)if budget.exists()else 0.;start=time.monotonic();paused=[];captured=[]
try:
 for pid in map(int,subprocess.run(['pgrep','-x','gripen-flight'],capture_output=True,text=True).stdout.split()):
  state=subprocess.run(['ps','-o','stat=','-p',str(pid)],capture_output=True,text=True).stdout.strip()
  if state and not state.startswith('T'):os.kill(pid,signal.SIGSTOP);paused.append(pid)
 for mode in a.modes:
  remaining=175.-used-(time.monotonic()-start)
  if remaining<15:raise TimeoutError('Approved native review budget exhausted')
  env=dict(os.environ,NORD_AIRCRAFT='fa18f')
  for key in ['NORD_NAVAL_CAPTURE','NORD_FLEET_CAPTURE','NORD_CARRIER_CAPTURE','NORD_CREW_CAPTURE','NORD_NAVAL_LIVE']:env.pop(key,None)
  env['NORD_FLEET_CAPTURE'if mode.startswith('cockpit-')else'NORD_NAVAL_CAPTURE']=mode
  shot=ROOT/'tests/visual-dry.png';shot.unlink(missing_ok=True)
  with (OUT/(mode+'.log')).open('w')as log:subprocess.run([str(ROOT/'Nord Flight.app/Contents/MacOS/gripen-flight'),'--visual-check','dry'],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=min(75.,remaining))
  shutil.copyfile(shot,OUT/(mode+'.png'));captured.append(mode);print(mode,flush=True)
finally:
 for pid in paused:
  try:os.kill(pid,signal.SIGCONT)
  except ProcessLookupError:pass
 seconds=used+time.monotonic()-start;budget.write_text(json.dumps({'seconds':seconds,'latest_captures':captured},indent=2)+'\n');print('Sessions restored; total diagnostic seconds',round(seconds,1),flush=True)
