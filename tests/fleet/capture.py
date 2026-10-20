"""Sequential native captures of the selected player aircraft; never mocked art."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time

ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tests/optimization3'))
from run import isolated,record_child

p=argparse.ArgumentParser()
p.add_argument('--aircraft',nargs='+',default=['su27','f35b','f22','fa18e'])
p.add_argument('--views',nargs='+',default=['exterior','cockpit'])
p.add_argument('--output',default='native-captures')
p.add_argument('--binary',default='target/debug/gripen-flight')
args=p.parse_args()
out=ROOT/'tests/fleet'/args.output
out.mkdir(parents=True,exist_ok=True)
binary=ROOT/args.binary
manifest=out/'manifest.json'
identity=hashlib.sha256(binary.read_bytes()).hexdigest()
record=json.loads(manifest.read_text()) if manifest.exists() else {'executable_sha256':identity,'captures':[],'source_hashes':{str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest() for p in (ROOT/'src').rglob('*.rs')}}
assert record['executable_sha256']==identity,'Use a fresh directory for a different executable'
with isolated():
 for aircraft in args.aircraft:
  for view in args.views:
   env=os.environ.copy()
   env.update(NORD_AIRCRAFT=aircraft,NORD_FLEET_CAPTURE=view,
              NORD_FLIGHT_AUDIO_SETTINGS=str(ROOT/'tests/optimization3/audio-settings.json'))
   with (out/f'{aircraft}-{view}.log').open('w') as log:
    started=time.time_ns()
    game=subprocess.Popen([str(binary),'--visual-check','full'],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT)
    record_child(game.pid)
    try:game.wait(timeout=100)
    finally:
     if game.poll() is None:game.terminate();game.wait(timeout=10)
     record_child(None)
    if game.returncode:raise SystemExit(f'{aircraft}/{view} failed ({game.returncode})')
   log_text=(out/f'{aircraft}-{view}.log').read_text()
   screenshot=ROOT/'tests/visual-full.png'
   if 'panicked at' in log_text or 'Encountered a panic' in log_text:
    raise SystemExit(f'{aircraft}/{view} renderer panicked; rejecting capture')
   if not screenshot.exists() or screenshot.stat().st_mtime_ns < started:
    raise SystemExit(f'{aircraft}/{view} produced no fresh screenshot')
   shutil.copy2(ROOT/'tests/visual-full.png',out/f'{aircraft}-{view}.png')
   record['captures'].append({'aircraft':aircraft,'view':view,'image':f'{aircraft}-{view}.png'})
   manifest.write_text(json.dumps(record,indent=2)+'\n')
   print(aircraft,view,flush=True)
