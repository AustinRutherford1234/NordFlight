#!/usr/bin/env python3
"""Capture the actual app's rail alignment and full stroke, light and tank-loaded."""
import os
import subprocess
import shutil
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];out=ROOT/'tests/carrier-catapult';out.mkdir(parents=True,exist_ok=True)
for tanks in [False,True]:
    for mode in ['catapult-0-close','catapult-0','catapult-25','catapult-50','catapult-75','catapult-100']:
        name=('tanks-' if tanks else 'light-')+mode
        env=dict(os.environ,NORD_CARRIER_CAPTURE=mode)
        for key in ['NORD_FLEET_CAPTURE','NORD_CREW_CAPTURE','NORD_CATAPULT_TANKS']:env.pop(key,None)
        if tanks:env['NORD_CATAPULT_TANKS']='1'
        shot=ROOT/'tests/visual-dry.png';shot.unlink(missing_ok=True)
        with (out/f'{name}.log').open('w') as log:
            subprocess.run([str(ROOT/'Nord Flight.app/Contents/MacOS/gripen-flight'),'--visual-check','dry'],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=90)
        if not shot.exists():raise RuntimeError(f'{name}: screenshot missing')
        shutil.copyfile(shot,out/f'{name}.png');print('Captured',name,flush=True)
