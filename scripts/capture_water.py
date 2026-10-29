"""Capture gentle ditch, high-energy plume, flooded hull and submerged aircraft."""
import os,subprocess,shutil
from pathlib import Path
root=Path(__file__).resolve().parents[1];out=root/'tests/water';out.mkdir(exist_ok=True,parents=True)
for mode in ['water-gentle','water-fast','water-settled','water-sunk']:
 env=dict(os.environ,NORD_AIRCRAFT='fa18f',NORD_FLEET_CAPTURE=mode)
 for key in ['NORD_CARRIER_CAPTURE','NORD_CREW_CAPTURE']:env.pop(key,None)
 shot=root/'tests/visual-dry.png';shot.unlink(missing_ok=True)
 with (out/(mode+'.log')).open('w') as log:subprocess.run([str(root/'Nord Flight.app/Contents/MacOS/gripen-flight'),'--visual-check','dry'],cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT,timeout=120,check=True)
 shutil.copyfile(shot,out/(mode+'.png'));print(mode,flush=True)
