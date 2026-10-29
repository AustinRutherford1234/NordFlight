"""Capture the fitted cockpit and consecutive escape-system visual stages."""
from pathlib import Path
import os,subprocess,shutil,argparse
root=Path(__file__).resolve().parents[2];out=root/'tests/hornet-cockpit';out.mkdir(exist_ok=True,parents=True)
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--modes',nargs='+',default=['cockpit','cockpit-seat','cockpit-floor','cockpit-rear','detail-hornet-seat','escape-0','escape-0.4','escape-0.65','escape-1.2','escape-2.2','escape-3.6','escape-8'])
args=parser.parse_args()
for mode in args.modes:
 env=dict(os.environ,NORD_AIRCRAFT='fa18f',NORD_FLEET_CAPTURE=mode)
 for k in ['NORD_CARRIER_CAPTURE','NORD_CREW_CAPTURE']:env.pop(k,None)
 shot=root/'tests/visual-dry.png';shot.unlink(missing_ok=True)
 with (out/(mode+'.log')).open('w') as log:subprocess.run([str(root/'Nord Flight.app/Contents/MacOS/gripen-flight'),'--visual-check','dry'],cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=120)
 shutil.copyfile(shot,out/(mode+'.png'));print(mode,flush=True)
