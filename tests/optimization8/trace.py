"""One bounded Metal capture. Never run concurrently with FPS measurements."""
import json
import os
from pathlib import Path
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'tests/optimization8/metal'
sys.path.insert(0, str(ROOT / 'tests/optimization3'))
from run import isolated, record_child, command

OUT.mkdir(exist_ok=True)
trace = OUT / 'profile.trace'
assert not trace.exists(), 'Use a fresh capture destination'
binary = ROOT / 'Nord Flight.app/Contents/MacOS/gripen-flight'
env = dict(os.environ, NORD_BENCH_SCENE='terrain-forest', NORD_BENCH_LIVE='0',
           NORD_BENCH_SCRIPT='terrain-stream', NORD_TERRAIN_AGL='50',
           NORD_TERRAIN_COCKPIT='1', NORD_BENCH_FULLSCREEN='1', NORD_TERRAIN_SPEED='100',
           NORD_BENCH_SECONDS='120', NORD_BENCH_FPS_ONLY='1',
           NORD_FLIGHT_AUDIO_SETTINGS=str(ROOT / 'tests/optimization3/audio-settings.json'))
(OUT / 'host.json').write_text(json.dumps({'power':command(['pmset','-g','batt']),
    'thermal':command(['pmset','-g','therm'])}, indent=2))
with isolated(), (OUT / 'game.log').open('w') as log:
    game = subprocess.Popen([str(binary), '--benchmark-render', 'metal-terrain'], cwd=ROOT,
                            env=env, stdout=log, stderr=subprocess.STDOUT)
    record_child(game.pid)
    try:
        time.sleep(15)
        with (OUT / 'capture.log').open('w') as output:
            result = subprocess.run(['xcrun','xctrace','record','--template','Metal System Trace',
                '--time-limit','5s','--output',str(trace),'--attach',str(game.pid)],
                stdout=output, stderr=subprocess.STDOUT, timeout=90)
        (OUT / 'result.json').write_text(json.dumps({'exit':result.returncode}))
    finally:
        game.terminate()
        try: game.wait(timeout=10)
        except subprocess.TimeoutExpired: game.kill(); game.wait()
        record_child(None)
if result.returncode: sys.exit(result.returncode)
with (OUT / 'toc.xml').open('w') as toc:
    subprocess.run(['xcrun','xctrace','export','--input',str(trace),'--toc'],stdout=toc,check=True,timeout=60)
for schema, name in [('metal-gpu-intervals','gpu.xml'),('metal-resource-allocations','resources.xml')]:
    with (OUT / (name + '.log')).open('w') as log:
        subprocess.run(['xcrun','xctrace','export','--input',str(trace),'--xpath',
            '/trace-toc/run[@number="1"]/data/table[@schema="'+schema+'"]',
            '--output',str(OUT / name)],stdout=log,stderr=subprocess.STDOUT,check=True,timeout=90)
subprocess.run([sys.executable,str(ROOT / 'tests/optimization3/trace_summary.py'),str(OUT)],check=True)
