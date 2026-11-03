"""Bounded SR-71 review; run only with live-app diagnostic approval."""
import os,subprocess,signal,time,shutil,json,sys
from pathlib import Path
root=Path(__file__).resolve().parents[2];out=root/'tests/sr71-visuals';out.mkdir(parents=True,exist_ok=True)
previous=json.loads((out/'review.json').read_text())if(out/'review.json').exists()else{}
used=previous.get('seconds',0.);start=time.monotonic();paused=[];results=[]
try:
 for pid in map(int,subprocess.run(['pgrep','-x','gripen-flight'],capture_output=True,text=True).stdout.split()):
  state=subprocess.run(['ps','-o','stat=','-p',str(pid)],capture_output=True,text=True).stdout.strip()
  if state and not state.startswith('T'):os.kill(pid,signal.SIGSTOP);paused.append(pid)
 for mode in (sys.argv[1:] or ['top','side','front']):
  remaining=115-used-(time.monotonic()-start)
  if remaining<15:break
  env={k:v for k,v in os.environ.items() if not k.startswith('NORD_')};env.update(NORD_AIRCRAFT='sr71',NORD_FLEET_CAPTURE=mode)
  with (out/('imported-'+mode+'.log')).open('w') as log:subprocess.run([str(root/'Nord Flight.app/Contents/MacOS/gripen-flight'),'--visual-check','dry'],cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT,timeout=min(40,remaining),check=True)
  shutil.copyfile(root/'tests/visual-dry.png',out/('imported-'+mode+'.png'));results.append('imported-'+mode)
finally:
 for pid in paused:
  try:os.kill(pid,signal.SIGCONT)
  except ProcessLookupError:pass
 data=dict(seconds=used+time.monotonic()-start,captures=previous.get('captures',[])+results,restoration_handler_completed=True)
 (out/'review.json').write_text(json.dumps(data,indent=2));print(data)
