#!/usr/bin/env python3
"""Approved, cumulative <=180-second SR-71 exhaust visual review.
Pauses/restores existing flights and records actual diagnostic wall time.
"""
import hashlib,json,os,pathlib,signal,subprocess,sys,time,shutil
root=pathlib.Path(__file__).resolve().parents[2]
out=root/'tests/sr71-exhaust';out.mkdir(parents=True,exist_ok=True)
record=out/'review.json'
previous=json.loads(record.read_text()) if record.exists() else {'seconds':0.,'captures':[]}
used=previous['seconds'];start=time.monotonic();paused=[];captures=[]
try:
 for pid in map(int,subprocess.run(['pgrep','-x','gripen-flight'],capture_output=True,text=True).stdout.split()):
  state=subprocess.run(['ps','-o','stat=','-p',str(pid)],capture_output=True,text=True).stdout.strip()
  if state and not state.startswith('T'):
   os.kill(pid,signal.SIGSTOP);paused.append(pid)
 for suffix in sys.argv[1:]:
  remaining=178.-used-(time.monotonic()-start)
  if remaining<12.:break
  mode='sr71-exhaust-'+suffix
  env={k:v for k,v in os.environ.items() if not k.startswith('NORD_')}
  env.update(NORD_AIRCRAFT='sr71',NORD_FLEET_CAPTURE=mode)
  binary=root/'target/debug/gripen-flight'
  digest=hashlib.sha256(binary.read_bytes()).hexdigest()
  label=suffix+'-'+digest[:8]
  with (out/(label+'.log')).open('w') as log:
   subprocess.run([str(binary),'--visual-check','dry'],cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT,timeout=min(35.,remaining),check=True)
  shutil.copyfile(root/'tests/visual-dry.png',out/(label+'.png'))
  captures.append({'mode':mode,'image':label+'.png','binary_sha256':digest})
finally:
 for pid in paused:
  try:os.kill(pid,signal.SIGCONT)
  except ProcessLookupError:pass
 previous['seconds']=used+time.monotonic()-start
 previous['captures']+=captures
 previous['restoration_handler_completed']=True
 record.write_text(json.dumps(previous,indent=2)+'\n')
 print(json.dumps(previous,indent=2))
