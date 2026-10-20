"""Sequential final validation for the focused cockpit/atmosphere pass."""
from pathlib import Path
import hashlib,json,os,signal,subprocess,sys

ROOT=Path(__file__).resolve().parents[2]; OUT=ROOT/'tests/optimization4'
PY=sys.executable; candidate=OUT/'candidate/gripen-flight-release'
baseline=OUT/'baseline/gripen-flight-release'
status=OUT/'final-progress.json'
identity={str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in [baseline,candidate]}
completed=[]
if status.exists():
    old=json.loads(status.read_text())
    if old['executables']!=identity: raise RuntimeError('Executable pair changed')
    completed=old['completed']
child=None
def save(stage,state):
    status.write_text(json.dumps({'pid':os.getpid(),'stage':stage,'state':state,
        'completed':completed,'executables':identity},indent=2)+'\n')
def stage(name,args,extra=None):
    global child
    if name in completed: return
    print('Starting '+name,flush=True);save(name,'running')
    env=os.environ.copy();env.update(extra or {})
    with (OUT/(name+'-driver.log')).open('w') as log:
        child=subprocess.Popen(args,cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT)
        try: code=child.wait()
        finally:
            if child.poll() is None: child.terminate();child.wait(timeout=30)
            child=None
    if code: save(name,'failed');raise RuntimeError(f'{name} failed: {code}')
    completed.append(name);save(name,'complete');print('Completed '+name,flush=True)

signal.signal(signal.SIGTERM,lambda *_: (_ for _ in ()).throw(KeyboardInterrupt()))
try:
    for name,binary in [('baseline',baseline),('candidate',candidate)]:
        stage('cpu-'+name,[PY,'tests/optimization3/run.py',str(binary),'../optimization4/release-'+name,'cpu'])
    stage('physical-equivalence',[PY,'tests/optimization3/verify.py',str(OUT/'release-baseline'),
        str(OUT/'release-candidate'),str(OUT/'release-physics-equivalence.json')])
    stage('transitions-six',[PY,'tests/optimization4/focused.py',str(candidate),'release-transitions-six','transitions'])
    stage('transitions-one',[PY,'tests/optimization4/focused.py',str(candidate),'release-transitions-one','transitions','--reference'])
    stage('transition-images',[PY,'tests/optimization4/compare_transitions.py',
        str(OUT/'release-transitions-one/frames'),str(OUT/'release-transitions-six/frames'),str(OUT/'release-transition-review')])
    stage('render-matrix',[PY,'tests/optimization4/focused.py',str(candidate),'release-matrix','matrix'])
    for name,stages in [('reference','1'),('six','6')]:
        stage('metal-'+name,[PY,'tests/optimization3/profile.py',str(candidate),
            '../optimization4/metal-'+name,'--scene','bvr-cockpit-missiles','--live'],{'NORD_ENV_STAGES':stages})
    stage('pacing-control',[PY,'tests/optimization4/focused.py',str(candidate),'release-pacing','pacing','--scenes','cockpit-paused,bvr'])
    stage('atmosphere-soak',[PY,'tests/optimization4/focused.py',str(candidate),'release-soak','soak'])
    manifest=json.loads((OUT/'candidate/source-hashes.json').read_text())
    assert all(hashlib.sha256((ROOT/p).read_bytes()).hexdigest()==h for p,h in manifest.items()),'Source drift'
    save('all-suites','complete')
except BaseException:
    if child is not None and child.poll() is None: child.terminate();child.wait(timeout=30)
    save('interrupted','stopped')
    raise
