"""One explicitly approved five-second Metal capture of the restored ocean."""
from pathlib import Path
import os,sys,shutil,subprocess,time,json,hashlib
root=Path(__file__).resolve().parents[2];out=root/'tests/ocean-optimization/gpu';out.mkdir(exist_ok=True)
sys.path.insert(0,str(root/'tests/optimization3'))
from run import isolated,record_child,command
trace=out/'profile.trace';assert not trace.exists(),'Use a fresh capture destination'
contents=out/'Nord Flight.app/Contents';(contents/'MacOS').mkdir(parents=True,exist_ok=True);(contents/'Resources').mkdir(exist_ok=True)
binary=contents/'MacOS/gripen-flight';source=root/'Nord Flight.app/Contents/MacOS/gripen-flight';shutil.copy2(source,binary);shutil.copy2(root/'packaging/Info.plist',contents/'Info.plist')
(contents/'Resources/assets').symlink_to(root/'assets',target_is_directory=True)
entitlements=out/'profiling.entitlements';entitlements.write_text('<?xml version="1.0"?><plist version="1.0"><dict><key>com.apple.security.get-task-allow</key><true/></dict></plist>')
subprocess.run(['codesign','--force','--sign','-','--entitlements',str(entitlements),str(contents.parent)],check=True)
env={k:v for k,v in os.environ.items()if not k.startswith(('NORD_BENCH','NORD_TERRAIN','NORD_SOAK','NORD_FLEET_CAPTURE','NORD_CARRIER_CAPTURE','NORD_CREW_CAPTURE','NORD_PROFILE','NORD_OCEAN_REFERENCE'))}
env.update(NORD_CARRIER_CAPTURE='ocean-approach',NORD_BENCH_SCRIPT='ocean',NORD_BENCH_SCENE='dry',NORD_BENCH_FULLSCREEN='1',NORD_PRESENT_MODE='vsync',NORD_BENCH_SECONDS='120',NORD_BENCH_WARMUP='20',NORD_BENCH_FPS_ONLY='1',NORD_BENCH_REPORT=str(out/'unused.json'))
(out/'identity.json').write_text(json.dumps({'original_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'profile_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'warmup_seconds':20,'capture_seconds':5,'power':command(['pmset','-g','batt']),'thermal':command(['pmset','-g','therm'])},indent=2))
with isolated(),(out/'game.log').open('w')as log:
 game=subprocess.Popen([str(binary),'--benchmark-render','dry'],cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT);record_child(game.pid)
 try:
  time.sleep(20)
  with(out/'capture.log').open('w')as capture:
   result=subprocess.run(['xcrun','xctrace','record','--template','Metal System Trace','--time-limit','5s','--output',str(trace),'--attach',str(game.pid)],stdout=capture,stderr=subprocess.STDOUT,timeout=90)
  (out/'result.json').write_text(json.dumps({'exit':result.returncode}))
 finally:
  game.terminate()
  try:game.wait(timeout=10)
  except subprocess.TimeoutExpired:game.kill();game.wait()
  record_child(None)
if result.returncode:raise SystemExit('Capture failed: inspect gpu/capture.log')
with(out/'toc.xml').open('w')as toc:subprocess.run(['xcrun','xctrace','export','--input',str(trace),'--toc'],stdout=toc,check=True,timeout=60)
for schema,name in [('metal-gpu-intervals','gpu.xml'),('metal-resource-allocations','resources.xml'),('ca-client-buffer-wait-interval','drawable-waits.xml')]:
 with(out/(name+'.log')).open('w')as log:subprocess.run(['xcrun','xctrace','export','--input',str(trace),'--xpath','/trace-toc/run[@number="1"]/data/table[@schema="'+schema+'"]','--output',str(out/name)],stdout=log,stderr=subprocess.STDOUT,check=True,timeout=90)
subprocess.run([sys.executable,str(root/'tests/optimization3/trace_summary.py'),str(out)],check=True)
print('Original game sessions restored.',flush=True)
