"""Approved <=3-minute shoulder-mount review; always restore the user's running flight."""
from pathlib import Path
import os,signal,subprocess,shutil,argparse,time,json
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'tests/shoulder-mount-review';OUT.mkdir(parents=True,exist_ok=True)
p=argparse.ArgumentParser();p.add_argument('--modes',nargs='+',default=['gear-cycle-0-extend-stores','gear-cycle-150-extend-stores','gear-cycle-60-extend-stores','gear-cycle-420-extend-stores','gear-cycle-150-retract-stores','gear-cycle-150-extend-stores-right']);a=p.parse_args()
budget=OUT/'review-budget.json';used=json.loads(budget.read_text()).get('seconds',0.)if budget.exists()else 0.
start=time.monotonic();paused=[];captured=[]
try:
 for pid in map(int,subprocess.run(['pgrep','-x','gripen-flight'],capture_output=True,text=True).stdout.split()):
  state=subprocess.run(['ps','-o','stat=','-p',str(pid)],capture_output=True,text=True).stdout.strip()
  if state and not state.startswith('T'):os.kill(pid,signal.SIGSTOP);paused.append(pid)
 for mode in a.modes:
  remaining=175.-used-(time.monotonic()-start)
  if remaining<15:raise TimeoutError('Approved native review budget exhausted')
  env=dict(os.environ,NORD_AIRCRAFT='fa18f',NORD_FLEET_CAPTURE=mode)
  for key in ['NORD_NAVAL_CAPTURE','NORD_TAXI_CAPTURE','NORD_CARRIER_CAPTURE','NORD_CREW_CAPTURE','NORD_NAVAL_LIVE','NORD_RADAR_CAPTURE']:env.pop(key,None)
  shot=ROOT/'tests/visual-dry.png';shot.unlink(missing_ok=True)
  with (OUT/(mode+'.log')).open('w')as log:
   subprocess.run([str(ROOT/'Nord Flight.app/Contents/MacOS/gripen-flight'),'--visual-check','dry'],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=min(65.,remaining))
  shutil.copyfile(shot,OUT/(mode+'.png'));captured.append(mode);print(mode,flush=True)
finally:
 for pid in paused:
  try:os.kill(pid,signal.SIGCONT)
  except ProcessLookupError:pass
 seconds=used+time.monotonic()-start
 budget.write_text(json.dumps({'seconds':seconds,'latest_captures':captured},indent=2)+'\n')
 print('Sessions restored; total diagnostic seconds',round(seconds,1),flush=True)
