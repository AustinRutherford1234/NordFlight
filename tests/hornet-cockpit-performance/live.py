"""One separately approved 30-second fullscreen live carrier-approach profile."""
from pathlib import Path
import os
import sys
import json
import hashlib

root = Path(__file__).resolve().parents[2]
folder = Path(__file__).resolve().parent / (sys.argv[1] if len(sys.argv) > 1 else 'live')
folder.mkdir(exist_ok=True)
sys.path.insert(0, str(root / 'tests/optimization3'))
from run import isolated, run, command

binary = root / 'target/debug/gripen-flight'
for key in list(os.environ):
    if key.startswith(('NORD_FLEET_CAPTURE', 'NORD_CARRIER_CAPTURE',
                       'NORD_CREW_CAPTURE', 'NORD_BENCH', 'NORD_SOAK',
                       'NORD_TERRAIN', 'NORD_PROFILE', 'NORD_CAPTURE_SEQUENCE')):
        os.environ.pop(key)
env = {
    'NORD_AIRCRAFT': 'fa18f', 'NORD_BENCH_SCENE': 'terrain-forest',
    'NORD_BENCH_FULLSCREEN': '1', 'NORD_PRESENT_MODE': 'vsync',
    'NORD_TERRAIN_COCKPIT': '1', 'NORD_BENCH_SCRIPT': 'carrier-live',
    'NORD_BENCH_WARMUP': '10', 'NORD_BENCH_SECONDS': '30',
    'NORD_BENCH_LIVE': '1',
    'NORD_BENCH_REPORT': str(folder / 'carrier-live.json'),
}
(folder / 'identity.json').write_text(json.dumps({
    'sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
    'source_changes': command(['git', 'diff', '--stat']),
    'env': env,
}, indent=2))
with isolated():
    run(binary, ['--benchmark-render', 'carrier-live'], folder,
        'carrier-live', env, timeout=120)
