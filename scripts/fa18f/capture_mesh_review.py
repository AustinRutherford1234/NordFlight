"""Native close views of both gear assemblies through extension and retraction."""
import os, subprocess, shutil, argparse
from pathlib import Path
root=Path(__file__).resolve().parents[2]
out=root/'tests/hornet-mesh-review';out.mkdir(exist_ok=True,parents=True)
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--steps',nargs='+',type=int,default=[0,25,60,100,150,200,260,330,420])
parser.add_argument("--views",nargs="+",choices=["main","nose"],default=["main","nose"])
args=parser.parse_args()
for direction in ['extend','retract']:
 for view in args.views:
  for t in args.steps:
   mode=f'gear-cycle-{t}-{direction}-{view}'
   env=dict(os.environ,NORD_AIRCRAFT='fa18f',NORD_FLEET_CAPTURE=mode)
   for key in ['NORD_CARRIER_CAPTURE','NORD_CREW_CAPTURE']:env.pop(key,None)
   shot=root/'tests/visual-dry.png';shot.unlink(missing_ok=True)
   with (out/(mode+'.log')).open('w') as log:
    subprocess.run([str(root/'Nord Flight.app/Contents/MacOS/gripen-flight'),'--visual-check','dry'],cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=90)
   if not shot.exists():raise RuntimeError(f'No capture: {mode}')
   shutil.copyfile(shot,out/(mode+'.png'));print(mode,flush=True)
