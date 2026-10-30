"""Native Hornet crash review. Run only with permission to open test flights.

Pauses and resumes an existing game; test processes exit after saving each image.
The paused inspection poses use the same debris integrator as normal flight.
"""
from pathlib import Path
import os, signal, subprocess, shutil, argparse

root=Path(__file__).resolve().parents[2]
out=root/'tests/hornet-crash'

def capture(modes):
    for mode in modes:
        capture_one(mode)

def capture_one(mode):
    env=dict(os.environ,NORD_AIRCRAFT='fa18f',NORD_FLEET_CAPTURE=mode)
    for key in ['NORD_CARRIER_CAPTURE','NORD_CREW_CAPTURE']:env.pop(key,None)
    shot=root/'tests/visual-dry.png';shot.unlink(missing_ok=True)
    with (out/(mode+'.log')).open('w') as log:
        subprocess.run([str(root/'Nord Flight.app/Contents/MacOS/gripen-flight'),'--visual-check','dry'],cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=120)
    shutil.copyfile(shot,out/(mode+'.png'))
    print(mode,flush=True)

if __name__=='__main__':
    paused=[]
    try:
        result=subprocess.run(['pgrep','-x','gripen-flight'],capture_output=True,text=True)
        for pid in map(int,result.stdout.split()):
            status=subprocess.run(['ps','-o','stat=','-p',str(pid)],capture_output=True,text=True).stdout.strip()
            if status and not status.startswith('T'):
                os.kill(pid,signal.SIGSTOP);paused.append(pid)
        parser=argparse.ArgumentParser(description=__doc__)
        parser.add_argument("--modes",nargs="+",default=["crash-0.1","crash-0.7","crash-3","crash-12","crash-45","crash-dry-3","water-fast"])
        capture(parser.parse_args().modes)
    finally:
        for pid in paused:
            try:os.kill(pid,signal.SIGCONT)
            except ProcessLookupError:pass
        print('Original game sessions resumed.',flush=True)
