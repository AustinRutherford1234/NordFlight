"""Approved native ocean review; pauses and restores any existing game."""
import os,signal,subprocess,shutil,argparse
from pathlib import Path
root=Path(__file__).resolve().parents[2];out=root/'tests/ocean-optimization'
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--modes',nargs='+',default=['ocean-approach'])
p.add_argument('--baseline',action='store_true')
args=p.parse_args();paused=[]
try:
    result=subprocess.run(['pgrep','-x','gripen-flight'],capture_output=True,text=True)
    for pid in map(int,result.stdout.split()):
        state=subprocess.run(['ps','-o','stat=','-p',str(pid)],capture_output=True,text=True).stdout.strip()
        if state and not state.startswith('T'):os.kill(pid,signal.SIGSTOP);paused.append(pid)
    for mode in args.modes:
        env=dict(os.environ,NORD_CARRIER_CAPTURE=mode)
        for key in ['NORD_FLEET_CAPTURE','NORD_CREW_CAPTURE','NORD_CATAPULT_TANKS']:env.pop(key,None)
        shot=root/'tests/visual-dry.png';shot.unlink(missing_ok=True)
        with (out/(mode+'.log')).open('w') as log:
            subprocess.run([str(out/('A' if args.baseline else 'B')/'Nord Flight.app/Contents/MacOS/gripen-flight'),'--visual-check','dry'],cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=120)
        shutil.copyfile(shot,out/('baseline.png' if args.baseline else 'optimized.png'));print(mode,flush=True)
finally:
    for pid in paused:
        try:os.kill(pid,signal.SIGCONT)
        except ProcessLookupError:pass
    print('Original game sessions resumed.',flush=True)
