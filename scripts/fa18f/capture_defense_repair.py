"""Approved <=3-minute fleet defense visual inspection."""
from pathlib import Path
import os,signal,subprocess,shutil,time,json,argparse
root=Path(__file__).resolve().parents[2];out=root/'tests/naval-defense-repair';out.mkdir(parents=True,exist_ok=True)
parser=argparse.ArgumentParser();parser.add_argument('--modes',nargs='+',default=['defense-burke-port','defense-burke-starboard','defense-skim','defense-ciws']);args=parser.parse_args()
previous=json.loads((out/'review.json').read_text())if(out/'review.json').exists()else{}
used=previous.get('seconds',0.)
start=time.monotonic();paused=[];captures=[]
try:
 for pid in map(int,subprocess.run(['pgrep','-x','gripen-flight'],capture_output=True,text=True).stdout.split()):
  state=subprocess.run(['ps','-o','stat=','-p',str(pid)],capture_output=True,text=True).stdout.strip()
  if state and not state.startswith('T'):os.kill(pid,signal.SIGSTOP);paused.append(pid)
 for tag in args.modes:
  remaining=175-used-(time.monotonic()-start)
  if remaining<12:raise TimeoutError('Approved review budget exhausted')
  env=dict(os.environ,NORD_AIRCRAFT='fa18f',NORD_NAVAL_CAPTURE=tag)
  for k in ['NORD_BENCH_SCRIPT','NORD_BENCH_SCENE','NORD_BENCH_SOAK','NORD_CARRIER_CAPTURE','NORD_NAVAL_LIVE','NORD_TAXI_CAPTURE','NORD_FLEET_CAPTURE','NORD_CREW_CAPTURE','NORD_RADAR_CAPTURE']:env.pop(k,None)
  shot=root/'tests/visual-dry.png';shot.unlink(missing_ok=True)
  with (out/(tag+'.log')).open('w')as log:
   subprocess.run([str(root/'Nord Flight.app/Contents/MacOS/gripen-flight'),'--visual-check','dry'],cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT,timeout=min(55,remaining),check=True)
  shutil.copyfile(shot,out/(tag+'.png'));captures.append(tag);print(tag,flush=True)
finally:
 for pid in paused:
  try:os.kill(pid,signal.SIGCONT)
  except ProcessLookupError:pass
 elapsed=used+time.monotonic()-start;(out/'review.json').write_text(json.dumps({'seconds':elapsed,'captures':previous.get('captures',[])+captures,'restoration_handler_completed':True},indent=2)+'\n');print('Flights restored; seconds',round(elapsed,2),flush=True)
