"""Three separately approved 30-second live camera/instrument comparisons.

Runtime checks require the user's approval. All runs use the same executable,
normal fullscreen rendering, and a single neutral-input carrier-approach start.
"""
from pathlib import Path
import hashlib
import json
import os
import sys

root = Path(__file__).resolve().parents[2]
folder = Path(__file__).resolve().parent / 'matrix'
folder.mkdir(exist_ok=True)
sys.path.insert(0, str(root / 'tests/optimization3'))
from run import isolated, run, command

binary = root / 'target/debug/gripen-flight'
for key in list(os.environ):
    if key.startswith(('NORD_FLEET_CAPTURE', 'NORD_CARRIER_CAPTURE',
                       'NORD_CREW_CAPTURE', 'NORD_BENCH', 'NORD_SOAK',
                       'NORD_TERRAIN', 'NORD_PROFILE', 'NORD_CAPTURE_SEQUENCE')):
        os.environ.pop(key)
base = {
    'NORD_AIRCRAFT': 'fa18f', 'NORD_BENCH_SCENE': 'terrain-forest',
    'NORD_BENCH_FULLSCREEN': '1', 'NORD_PRESENT_MODE': 'vsync',
    'NORD_TERRAIN_COCKPIT': '1', 'NORD_BENCH_SCRIPT': 'carrier-live',
    'NORD_BENCH_WARMUP': '10', 'NORD_BENCH_SECONDS': '30',
    'NORD_BENCH_LIVE': '1',
}
(folder / 'identity.json').write_text(json.dumps({
    'sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
    'source_changes': command(['git', 'diff', '--stat']),
    'env': base,
}, indent=2))
with isolated():
    for name, extra in [
        ('cockpit', {}),
        ('external', {'NORD_BENCH_CAMERA': 'external'}),
        ('frozen-instruments', {'NORD_BENCH_FREEZE_INSTRUMENTS': '1'}),
    ]:
        env = dict(base, **extra,
                   NORD_BENCH_REPORT=str(folder / (name + '.json')))
        run(binary, ['--benchmark-render', name], folder, name, env, timeout=120)
        d = json.loads((folder / (name + '.json')).read_text())
        print(json.dumps({k: d[k] for k in [
            'mode', 'mean_fps', 'p95_ms', 'p99_ms', 'simulation_rate',
        ]}), flush=True)
