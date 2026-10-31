"""Approved aggregate three-minute native radar/HUD review; restores running flights."""
from pathlib import Path
import os, signal, subprocess, shutil, time, json
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'tests/hornet-radar-review';OUT.mkdir(parents=True,exist_ok=True)
budget=OUT/'review-budget.json'
used=json.loads(budget.read_text()).get('seconds',0.) if budget.exists() else 0.
started=time.monotonic();paused=[];captured=[]
try:
    for pid in map(int,subprocess.run(['pgrep','-x','gripen-flight'],capture_output=True,text=True).stdout.split()):
        state=subprocess.run(['ps','-o','stat=','-p',str(pid)],capture_output=True,text=True).stdout.strip()
        if state and not state.startswith('T'):
            os.kill(pid,signal.SIGSTOP);paused.append(pid)
    for mode in ['long','nez','outside']:
        remaining=175.-used-(time.monotonic()-started)
        if remaining<15:raise TimeoutError('Approved three-minute review budget exhausted')
        env={k:v for k,v in os.environ.items() if not k.startswith('NORD_')}
        env.update(NORD_AIRCRAFT='fa18f',NORD_RADAR_CAPTURE=mode)
        with (OUT/(mode+'.log')).open('w') as log:
            subprocess.run([str(ROOT/'Nord Flight.app/Contents/MacOS/gripen-flight'),'--visual-check','dry'],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=min(50.,remaining))
        shutil.copyfile(ROOT/'tests/visual-dry.png',OUT/(mode+'.png'))
        for panel in (ROOT/'tests').glob('jet-panel-*.png'):
            shutil.copyfile(panel,OUT/(mode+'-'+panel.name))
        captured.append(mode);print(mode,flush=True)
finally:
    for pid in paused:
        try:os.kill(pid,signal.SIGCONT)
        except ProcessLookupError:pass
    seconds=used+time.monotonic()-started
    budget.write_text(json.dumps({'seconds':seconds,'latest_captures':captured},indent=2)+'\n')
    print('Sessions restored; aggregate diagnostic seconds',round(seconds,1),flush=True)
