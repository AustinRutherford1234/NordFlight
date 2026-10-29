#!/usr/bin/env python3
"""Capture Hornet mounted stores, ballistic tank separation and missile releases."""
import os
import argparse
from pathlib import Path
import shutil
import subprocess
ROOT = Path(__file__).resolve().parents[2]
out = ROOT / 'tests/hornet-stores'
out.mkdir(parents=True, exist_ok=True)
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--modes',nargs='+',default=['stores-loaded','stores-folded','stores-drop-0','stores-drop-0.25','stores-drop-0.75','stores-launch-9x','stores-launch-120'])
args=parser.parse_args()
for mode in args.modes:
    if mode not in ['stores-loaded','stores-folded','stores-drop-0','stores-drop-0.25','stores-drop-0.75','stores-launch-9x','stores-launch-120']:parser.error(f'Unknown mode: {mode}')
    env = dict(os.environ, NORD_AIRCRAFT='fa18f', NORD_FLEET_CAPTURE=mode)
    for key in ['NORD_CARRIER_CAPTURE', 'NORD_CREW_CAPTURE']:
        env.pop(key, None)
    shot = ROOT / 'tests/visual-dry.png'
    shot.unlink(missing_ok=True)
    with (out / f'{mode}.log').open('w') as log:
        subprocess.run([str(ROOT / 'Nord Flight.app/Contents/MacOS/gripen-flight'), '--visual-check', 'dry'],
                       cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT, timeout=90, check=True)
    if not shot.is_file():
        raise RuntimeError(f'{mode}: screenshot missing')
    shutil.copyfile(shot, out / f'{mode}.png')
    print(f'Captured {mode}', flush=True)
