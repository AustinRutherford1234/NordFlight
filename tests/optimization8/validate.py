"""Focused, sequential near-terrain acceptance on battery; no quality changes."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import sys

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'tests/optimization8'
sys.path.insert(0, str(ROOT / 'tests/optimization3'))
from run import isolated, run, command
from verify import physical

p = argparse.ArgumentParser()
p.add_argument('phase', choices=['dev-visual', 'visual', 'matrix', 'repeat', 'repeat-external', 'repeat-cockpit', 'repeat-live', 'shader', 'mechanism', 'stability', 'flight', 'windowed', 'windowed-cockpit', 'transitions'])
args = p.parse_args()
baseline = OUT / 'baseline/gripen-flight'
candidate = ROOT / ('target/debug/gripen-flight' if args.phase == 'dev-visual' else 'Nord Flight.app/Contents/MacOS/gripen-flight')
folder = OUT / args.phase
folder.mkdir(exist_ok=True)
(folder / 'identity.json').write_text(json.dumps({
    'baseline_sha256': hashlib.sha256(baseline.read_bytes()).hexdigest(),
    'candidate_sha256': hashlib.sha256(candidate.read_bytes()).hexdigest(),
    'power': command(['pmset', '-g', 'batt']),
    'thermal': command(['pmset', '-g', 'therm']),
}, indent=2))

def render(binary, tag, cockpit=False, extra=None, seconds='25'):
    env = {'NORD_BENCH_SCENE': 'terrain-forest', 'NORD_BENCH_LIVE': '0',
           'NORD_BENCH_SCRIPT': 'terrain-stream', 'NORD_TERRAIN_AGL': '50',
           'NORD_TERRAIN_SPEED': '100', 'NORD_BENCH_WARMUP': '10',
           'NORD_BENCH_SECONDS': seconds, 'NORD_BENCH_FPS_ONLY': '1',
           'NORD_BENCH_REPORT': str(folder / (tag + '.json'))}
    if not args.phase.startswith('windowed'): env['NORD_BENCH_FULLSCREEN'] = '1'
    if cockpit: env['NORD_TERRAIN_COCKPIT'] = '1'
    env.update(extra or {})
    if args.phase == 'mechanism': env.pop('NORD_BENCH_FPS_ONLY', None)
    (folder / (tag + '-host.json')).write_text(json.dumps({
        'power': command(['pmset', '-g', 'batt']),
        'thermal': command(['pmset', '-g', 'therm']),
        'processes': command(['ps', '-A', '-o', '%cpu=,comm=']),
    }, indent=2))
    run(binary, ['--benchmark-render', tag], folder, tag, env, timeout=240)
    data = json.loads((folder / (tag + '.json')).read_text())
    if env['NORD_BENCH_SCRIPT'] == 'terrain-stream' and args.phase != 'stability':
        assert all(e['forest'] for e in data['exercise_events'] if e['wall_s'] >= 10)

with isolated():
    if args.phase in ('dev-visual', 'visual'):
        for label, binary in [('baseline', baseline), ('candidate', candidate)]:
            for cockpit, view in [(False, 'external'), (True, 'cockpit')]:
                source = ROOT / 'tests/visual-terrain-forest.png'
                stamp = source.stat().st_mtime_ns if source.exists() else 0
                env = {'NORD_TERRAIN_AGL': '50', 'NORD_AIRCRAFT': 'f22'}
                if cockpit: env['NORD_TERRAIN_COCKPIT'] = '1'
                tag = label + '-' + view
                run(binary, ['--visual-check', 'terrain-forest'], folder, tag, env)
                assert source.stat().st_mtime_ns > stamp
                shutil.copy2(source, folder / (tag + '.png'))
    elif args.phase in ('matrix', 'repeat', 'repeat-external', 'repeat-cockpit', 'repeat-live', 'windowed', 'windowed-cockpit'):
        for cockpit, view in [(False, 'external'), (True, 'cockpit'), (True, 'live')]:
            if args.phase == 'windowed-cockpit' and view != 'cockpit': continue
            if args.phase.startswith('repeat-') and view != args.phase.removeprefix('repeat-'): continue
            versions = [('baseline', baseline), ('candidate', candidate)]
            if args.phase.startswith('repeat'): versions.reverse()
            for label, binary in versions:
                extra = {'NORD_BENCH_SCRIPT': '', 'NORD_BENCH_LIVE': '1', 'NORD_TERRAIN_AGL': '350'} if view == 'live' else {}
                render(binary, view + '-' + label, cockpit, extra)
    elif args.phase == 'shader':
        for cockpit, view in [(False, 'external'), (True, 'cockpit')]:
            for reference in (True, False):
                render(candidate, view + ('-reference' if reference else '-restricted'), cockpit,
                       {'NORD_OPAQUE_DISCARD_REFERENCE': '1'} if reference else {})
    elif args.phase == 'mechanism':
        # Counters are intentionally kept out of the accepted FPS measurements.
        for reference in (True, False):
            render(candidate, 'source-' + ('reference' if reference else 'gated'), True,
                   {'NORD_BENCH_FPS_ONLY': '0', **({'NORD_ENV_SOURCE_REFERENCE': '1'} if reference else {})}, '10')
    elif args.phase == 'stability':
        render(candidate, 'streaming-120s', True, seconds='120')
    elif args.phase == 'transitions':
        run(candidate, ['--benchmark-render', 'transition-captures'], folder, 'capture',
            {'NORD_TRANSITION_CAPTURE': str(folder / 'frames'), 'NORD_BENCH_SECONDS': '1800',
             'NORD_BENCH_SCENE': 'cockpit', 'NORD_BENCH_LIVE': '0'}, timeout=300)
    else:
        for label, binary in [('baseline', baseline), ('candidate', candidate)]:
            run(binary, ['--study-flight'], folder, label)
            shutil.copy2(ROOT / 'tests/aerodynamic-study.json', folder / (label + '.json'))
        a = physical('aerodynamic-study.json', json.loads((folder / 'baseline.json').read_text()))
        b = physical('aerodynamic-study.json', json.loads((folder / 'candidate.json').read_text()))
        (folder / 'equivalence.json').write_text(json.dumps({'equal_excluding_timings': a == b}))
        assert a == b
