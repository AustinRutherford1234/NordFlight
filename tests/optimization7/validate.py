"""Bounded native acceptance for near-terrain streaming, preserving user flight."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import sys

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'tests/optimization7'
sys.path.insert(0, str(ROOT / 'tests/optimization3'))
from run import isolated, run, command
from verify import physical

p = argparse.ArgumentParser()
p.add_argument('phase', choices=['matrix', 'repeat', 'ac', 'prepass', 'visual', 'coast', 'stability', 'flight', 'final', 'accepted'])
args = p.parse_args()
baseline = OUT / 'baseline/gripen-flight'
candidate = ROOT / 'Nord Flight.app/Contents/MacOS/gripen-flight'
folder = OUT / args.phase
folder.mkdir(exist_ok=True)
(folder / 'identity.json').write_text(json.dumps({
    'baseline_sha256': hashlib.sha256(baseline.read_bytes()).hexdigest(),
    'candidate_sha256': hashlib.sha256(candidate.read_bytes()).hexdigest(),
    'power': command(['pmset', '-g', 'batt']),
    'thermal': command(['pmset', '-g', 'therm']),
}, indent=2))

def render(binary, tag, cockpit=False, seconds='25', extra=None):
    env = {'NORD_BENCH_SCENE': 'terrain-forest', 'NORD_BENCH_LIVE': '0',
           'NORD_BENCH_SCRIPT': 'terrain-stream', 'NORD_TERRAIN_AGL': '50',
           'NORD_TERRAIN_SPEED': '220', 'NORD_BENCH_FULLSCREEN': '1',
           'NORD_BENCH_WARMUP': '10', 'NORD_BENCH_SECONDS': seconds,
           'NORD_BENCH_REPORT': str(folder / (tag + '.json'))}
    if cockpit:
        env['NORD_TERRAIN_COCKPIT'] = '1'
    env.update(extra or {})
    run(binary, ['--benchmark-render', tag], folder, tag, env, timeout=240)

with isolated():
    if args.phase=='prepass':
        for cockpit,view in [(False,'external'),(True,'cockpit')]:
            choices=[False,True] if not cockpit else [True,False]
            for active in choices:
                render(candidate,view+('-on' if active else '-off'),cockpit,
                       extra={'NORD_TERRAIN_DEPTH_PREPASS':'1'} if active else {})
    elif args.phase in ('matrix','repeat','ac'):
        views=[(False,'external'),(True,'cockpit')] if args.phase=='matrix' else [(True,'cockpit')]
        for cockpit, view in views:
            for pair in range(2 if args.phase=='matrix' else 1):
                versions = [('baseline', baseline), ('candidate', candidate)]
                if pair:
                    versions.reverse()
                for label, binary in versions:
                    render(binary, f'{view}-{pair+1}-{label}', cockpit)
    elif args.phase in ('visual','coast'):
        for label, binary in [('baseline', baseline), ('candidate', candidate)]:
            views=[(False,'external'),(True,'cockpit')] if args.phase=='visual' else [(True,'cockpit')]
            for cockpit, view in views:
                source = ROOT / 'tests/visual-terrain-forest.png'
                stamp = source.stat().st_mtime_ns if source.exists() else 0
                env = {'NORD_TERRAIN_AGL': '50', 'NORD_AIRCRAFT': 'f22'}
                if cockpit:
                    env['NORD_TERRAIN_COCKPIT'] = '1'
                if args.phase=='coast':
                    env['NORD_TERRAIN_Z']='20000'
                tag = label + '-' + view
                run(binary, ['--visual-check', 'terrain-forest'], folder, tag, env)
                assert source.stat().st_mtime_ns > stamp
                shutil.copy2(source, folder / (tag + '.png'))
    elif args.phase == 'stability':
        render(candidate, 'streaming-120s', True, '120')
    elif args.phase in ('final','accepted'):
        run(candidate,['--study-flight'],folder,'flight')
        shutil.copy2(ROOT/'tests/aerodynamic-study.json',folder/'flight.json')
        a=physical('aerodynamic-study.json',json.loads((OUT/'flight/baseline.json').read_text()))
        b=physical('aerodynamic-study.json',json.loads((folder/'flight.json').read_text()))
        (folder/'equivalence.json').write_text(json.dumps({'equal_excluding_timings':a==b}))
        assert a==b
        source=ROOT/'tests/visual-terrain-forest.png'
        stamp=source.stat().st_mtime_ns
        run(candidate,['--visual-check','terrain-forest'],folder,'cockpit',
            {'NORD_TERRAIN_AGL':'50','NORD_TERRAIN_COCKPIT':'1','NORD_AIRCRAFT':'f22'})
        assert source.stat().st_mtime_ns>stamp
        shutil.copy2(source,folder/'cockpit.png')
    else:
        for label, binary in [('baseline', baseline), ('candidate', candidate)]:
            run(binary, ['--study-flight'], folder, label)
            shutil.copy2(ROOT / 'tests/aerodynamic-study.json', folder / (label + '.json'))
        a = physical('aerodynamic-study.json', json.loads((folder / 'baseline.json').read_text()))
        b = physical('aerodynamic-study.json', json.loads((folder / 'candidate.json').read_text()))
        (folder / 'equivalence.json').write_text(json.dumps({'equal_excluding_timings': a == b}))
        assert a == b
