"""Bounded native visual review. Pauses and always restores existing flights."""
from pathlib import Path
import argparse,os,signal,subprocess,shutil,time
ROOT=Path(__file__).resolve().parents[2];OUT=ROOT/'tests/bomber-rudder-review';OUT.mkdir(parents=True,exist_ok=True)
p=argparse.ArgumentParser();p.add_argument('--modes',nargs='+',default=['rudder-neutral','rudder-left','rudder-right','bomber-front-closed','bomber-rear-closed','bomber-belly-closed','bomber-belly-half','bomber-belly-open','bomber-top-closed']);a=p.parse_args();paused=[];start=time.monotonic()
try:
 for pid in map(int,subprocess.run(['pgrep','-x','gripen-flight'],capture_output=True,text=True).stdout.split()):
  state=subprocess.run(['ps','-o','stat=','-p',str(pid)],capture_output=True,text=True).stdout.strip()
  if state and not state.startswith('T'):os.kill(pid,signal.SIGSTOP);paused.append(pid)
 for mode in a.modes:
  env=dict(os.environ,NORD_AIRCRAFT='fa18f');
  for key in ['NORD_FLEET_CAPTURE','NORD_NAVAL_CAPTURE','NORD_CARRIER_CAPTURE']:env.pop(key,None)
  env['NORD_FLEET_CAPTURE' if mode.startswith('rudder-')else'NORD_NAVAL_CAPTURE']=mode
  shot=ROOT/'tests/visual-dry.png';shot.unlink(missing_ok=True)
  with (OUT/(mode+'.log')).open('w')as log:subprocess.run([str(ROOT/'Nord Flight.app/Contents/MacOS/gripen-flight'),'--visual-check','dry'],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT,timeout=75,check=True)
  shutil.copyfile(shot,OUT/(mode+'.png'));print(mode,flush=True)
finally:
 for pid in paused:
  try:os.kill(pid,signal.SIGCONT)
  except ProcessLookupError:pass
 print('Sessions restored; seconds',round(time.monotonic()-start,1),flush=True)
