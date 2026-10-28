#!/usr/bin/env python3
"""Capture the Hornet HUD from the native Bevy app, one disposable process per view."""
import argparse
import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--binary', type=Path, default=ROOT / 'Nord Flight.app/Contents/MacOS/gripen-flight')
parser.add_argument('--modes', nargs='+', default=['nav', 'air', 'air-radar', 'ground', 'landing', 'bank', 'reject', 'off'])
args = parser.parse_args()
out = ROOT / 'tests/hornet-hud'
out.mkdir(parents=True, exist_ok=True)
for mode in args.modes:
    if mode not in ['nav', 'air', 'air-radar', 'ground', 'landing', 'bank', 'reject', 'off']:
        parser.error(f'Unknown capture preset: {mode}')
    env = dict(os.environ, NORD_AIRCRAFT='fa18f', NORD_FLEET_CAPTURE=f'hud-{mode}')
    # Remove inherited unrelated capture selectors without changing app settings.
    for key in ['NORD_CARRIER_CAPTURE', 'NORD_CREW_CAPTURE']:
        env.pop(key, None)
    screenshot = ROOT / 'tests/visual-dry.png'
    screenshot.unlink(missing_ok=True)
    with (out / f'native-{mode}.log').open('w') as log:
        subprocess.run([str(args.binary.resolve()), '--visual-check', 'dry'], cwd=ROOT,
                       env=env, stdout=log, stderr=subprocess.STDOUT, timeout=90, check=True)
    if not screenshot.is_file():
        raise RuntimeError(f'{mode}: app exited without a screenshot')
    shutil.copyfile(screenshot, out / f'native-{mode}.png')
    print(f'Captured {mode}', flush=True)
