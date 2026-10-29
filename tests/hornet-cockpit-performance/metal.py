"""One explicitly approved five-second live-flight Metal capture, not an FPS test."""
from pathlib import Path
import hashlib
import json
import os
import shutil
import subprocess
import sys
import time
import statistics
import xml.etree.ElementTree as ET

root = Path(__file__).resolve().parents[2]
folder = Path(__file__).resolve().parent / 'gpu'
folder.mkdir(exist_ok=True)
sys.path.insert(0, str(root / 'tests/optimization3'))
from run import isolated, record_child, command

trace = folder / 'profile.trace'
assert not trace.exists(), 'Use a fresh capture destination'
original = root / 'target/debug/gripen-flight'
binary = folder / 'gripen-flight-profile'
shutil.copy2(original, binary)
entitlements = folder / 'profiling.entitlements'
entitlements.write_text('<?xml version="1.0"?><!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd"><plist version="1.0"><dict><key>com.apple.security.get-task-allow</key><true/></dict></plist>')
subprocess.run(['codesign', '--force', '--sign', '-', '--entitlements', str(entitlements), str(binary)], check=True)
env = {k: v for k, v in os.environ.items() if not k.startswith((
    'NORD_BENCH', 'NORD_TERRAIN', 'NORD_SOAK', 'NORD_FLEET_CAPTURE',
    'NORD_CARRIER_CAPTURE', 'NORD_CREW_CAPTURE', 'NORD_PROFILE', 'NORD_CAPTURE_SEQUENCE'))}
env.update(NORD_AIRCRAFT='fa18f', NORD_BENCH_SCENE='terrain-forest',
           NORD_BENCH_SCRIPT='carrier-live', NORD_BENCH_LIVE='1',
           NORD_BENCH_FULLSCREEN='1', NORD_PRESENT_MODE='vsync',
           NORD_TERRAIN_COCKPIT='1', NORD_BENCH_SECONDS='120',
           NORD_BENCH_FPS_ONLY='1', NORD_BENCH_REPORT=str(folder / 'unused.json'),
           NORD_FLIGHT_AUDIO_SETTINGS=str(root / 'tests/optimization3/audio-settings.json'))
(folder / 'identity.json').write_text(json.dumps({
    'original_sha256': hashlib.sha256(original.read_bytes()).hexdigest(),
    'profile_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
    'warmup_seconds': 15, 'capture_seconds': 5,
    'diagnostic_entitlement_only': True,
    'env': {k: v for k, v in env.items() if k.startswith('NORD_')},
    'power': command(['pmset', '-g', 'batt']),
}, indent=2))
with isolated(), (folder / 'game.log').open('w') as log:
    game = subprocess.Popen([str(binary), '--benchmark-render', 'metal-live'],
                            cwd=root, env=env, stdout=log, stderr=subprocess.STDOUT)
    record_child(game.pid)
    try:
        time.sleep(15)
        with (folder / 'capture.log').open('w') as output:
            result = subprocess.run(['xcrun', 'xctrace', 'record', '--template',
                'Metal System Trace', '--time-limit', '5s', '--output', str(trace),
                '--attach', str(game.pid)], stdout=output, stderr=subprocess.STDOUT, timeout=90)
        (folder / 'result.json').write_text(json.dumps({'exit': result.returncode}))
    finally:
        game.terminate()
        try:
            game.wait(timeout=10)
        except subprocess.TimeoutExpired:
            game.kill(); game.wait()
        record_child(None)
        binary.unlink(missing_ok=True)
if result.returncode:
    raise SystemExit('Capture failed; inspect gpu/capture.log')
with (folder / 'toc.xml').open('w') as toc:
    subprocess.run(['xcrun', 'xctrace', 'export', '--input', str(trace), '--toc'],
                   stdout=toc, check=True, timeout=60)
for schema, name in [('metal-gpu-intervals', 'gpu.xml'), ('metal-resource-allocations', 'resources.xml'),
                     ('ca-client-buffer-wait-interval', 'drawable-waits.xml')]:
    with (folder / (name + '.log')).open('w') as log:
        subprocess.run(['xcrun', 'xctrace', 'export', '--input', str(trace), '--xpath',
            '/trace-toc/run[@number="1"]/data/table[@schema="' + schema + '"]',
            '--output', str(folder / name)], stdout=log, stderr=subprocess.STDOUT,
            check=True, timeout=90)
subprocess.run([sys.executable, str(root / 'tests/optimization3/trace_summary.py'), str(folder)], check=True)

root_xml = ET.parse(folder / 'drawable-waits.xml').getroot()
ids = {e.attrib['id']: e for e in root_xml.iter() if 'id' in e.attrib}
def resolve(e):
    while 'ref' in e.attrib:
        e = ids[e.attrib['ref']]
    return e
waits = []
for row in root_xml.iter('row'):
    if 'gripen-flight-profile' in resolve(row.find('thread')).attrib.get('fmt', ''):
        waits.append(int(resolve(row.find('duration')).text) / 1e6)
waits.sort()
(folder / 'drawable-summary.json').write_text(json.dumps({
    'count': len(waits),
    'median_ms': statistics.median(waits) if waits else None,
    'p95_ms': waits[int(.95 * (len(waits) - 1))] if waits else None,
    'max_ms': max(waits) if waits else None,
    'note': 'Drawable waits can include scheduling, VSync and backpressure; not a GPU pass duration.',
}, indent=2) + '\n')
