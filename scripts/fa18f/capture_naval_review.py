"""Approved bounded carrier-defense visual/performance review. Restores existing sessions."""
import os,signal,subprocess,shutil,time,json,argparse
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'tests/naval-review';OUT.mkdir(parents=True,exist_ok=True)
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--modes',nargs='*',default=['hud','carrier','port','starboard','damage','sinking']);p.add_argument('--performance',action='store_true');p.add_argument('--quick',action='store_true');p.add_argument('--budget-seconds',type=float,default=570.);args=p.parse_args()
paused=[];start=time.monotonic()
try:
    for pid in map(int,subprocess.run(['pgrep','-x','gripen-flight'],capture_output=True,text=True).stdout.split()):
        state=subprocess.run(['ps','-o','stat=','-p',str(pid)],capture_output=True,text=True).stdout.strip()
        if state and not state.startswith('T'):os.kill(pid,signal.SIGSTOP);paused.append(pid)
    for mode in args.modes:
        env=dict(os.environ,NORD_NAVAL_CAPTURE=mode,NORD_AIRCRAFT='fa18f')
        shot=ROOT/'tests/visual-dry.png';shot.unlink(missing_ok=True)
        with (OUT/(mode+'.log')).open('w') as log:
            subprocess.run([str(ROOT/'Nord Flight.app/Contents/MacOS/gripen-flight'),'--visual-check','dry'],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=max(1.,min(65.,args.budget_seconds-(time.monotonic()-start))))
        shutil.copyfile(shot,OUT/(mode+'.png'));print('Captured',mode,flush=True)
    if args.performance:
        for mode in (['port'] if args.quick else ['baseline','port']):
            tag='final-smoke' if args.quick else mode
            env=dict(os.environ,NORD_NAVAL_CAPTURE=mode,NORD_NAVAL_LIVE='1',NORD_BENCH_FULLSCREEN='1',NORD_BENCH_WARMUP='5' if args.quick else '10',NORD_BENCH_SECONDS='15' if args.quick else ('60' if mode=='port' else '30'),NORD_BENCH_REPORT=str(OUT/(tag+'-performance.json')))
            with (OUT/(tag+'-performance.log')).open('w') as log:
                proc=subprocess.Popen([str(ROOT/'Nord Flight.app/Contents/MacOS/gripen-flight'),'--benchmark-render','naval-live'],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT)
                memory=[]
                try:
                    while proc.poll() is None:
                        if time.monotonic()-start>args.budget_seconds:proc.terminate();raise TimeoutError('Approved review budget nearly exhausted')
                        rss=subprocess.run(['ps','-o','rss=','-p',str(proc.pid)],capture_output=True,text=True).stdout.strip()
                        if rss:memory.append({'wall_s':time.monotonic()-start,'rss_kib':int(rss)})
                        time.sleep(1)
                    if proc.returncode:raise RuntimeError(f'{mode} exited {proc.returncode}')
                finally:
                    if proc.poll() is None:proc.terminate();proc.wait(timeout=10)
                (OUT/(tag+'-memory.json')).write_text(json.dumps(memory,indent=2))
            print('Measured',mode,flush=True)
finally:
    for pid in paused:
        try:os.kill(pid,signal.SIGCONT)
        except ProcessLookupError:pass
    print('Restored previous sessions; elapsed',round(time.monotonic()-start,1),'seconds.',flush=True)
