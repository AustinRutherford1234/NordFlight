"""Sequential, bounded acceptance checks for the focused atmospheric pass."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time

ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'tests/atmospheric6'
sys.path.insert(0,str(ROOT/'tests/optimization3'))
from run import isolated, run, STUDIES, command, record_child
from verify import physics

p=argparse.ArgumentParser()
p.add_argument('binary',type=Path)
p.add_argument('phase',choices=['visual','matrix','repeat','physics','transitions','stability','trace','pacing'])
p.add_argument('--scenes',default='cockpit,bvr,terrain')
args=p.parse_args()
candidate=args.binary.resolve();baseline=OUT/'baseline/gripen-flight'
OUT=OUT/'final';OUT.mkdir(exist_ok=True)

def folder(name):
    path=OUT/name;path.mkdir(exist_ok=True);return path

def render(binary,label,scene,extra=None):
    target=folder(label)
    env={'NORD_BENCH_SCENE':{'cockpit':'cockpit','bvr':'bvr-cockpit-missiles','terrain':'terrain-forest'}[scene],
         'NORD_BENCH_LIVE':'1' if scene=='bvr' else '0','NORD_BENCH_SCRIPT':{'cockpit':'','bvr':'bvr','terrain':'terrain-stream'}[scene],
         'NORD_TERRAIN_AGL':'50','NORD_TERRAIN_SPEED':'220',
         'NORD_BENCH_WARMUP':'10','NORD_BENCH_SECONDS':'25','NORD_BENCH_REPORT':str(target/(scene+'.json'))}
    env.update(extra or {})
    run(binary,['--benchmark-render',scene],target,scene,env)

with isolated():
    if args.phase in ('matrix','repeat'):
        suffix='-repeat' if args.phase=='repeat' else ''
        for scene in args.scenes.split(','):
            versions=[('baseline',baseline),('candidate',candidate)]
            if suffix:versions.reverse()
            for label,binary in versions:render(binary,label+suffix,scene)
    elif args.phase=='visual':
        for label,binary,extra in [('visual-baseline',baseline,{}),('visual-candidate',candidate,{}),('visual-reference',candidate,{'NORD_RENDER_REFERENCE':'1'})]:
            target=folder(label)
            for scene in ['cockpit','air-component-fire','terrain-forest']:
                source=ROOT/'tests'/('visual-'+scene+'.png');before=source.stat().st_mtime_ns if source.exists() else 0
                run(binary,['--visual-check',scene],target,scene,dict(extra,NORD_AIRCRAFT='f22'))
                assert source.exists() and source.stat().st_mtime_ns>before
                shutil.copy2(source,target/source.name)
    elif args.phase=='physics':
        for label,binary in [('physics-baseline',baseline),('physics-candidate',candidate)]:
            target=folder(label)
            for name,reports in STUDIES.items():
                run(binary,['--study-'+name],target,name,timeout=600)
                for report in reports:shutil.copy2(ROOT/'tests'/report,target/report)
        result=physics(OUT/'physics-baseline',OUT/'physics-candidate')
        (OUT/'equivalence.json').write_text(json.dumps(result,indent=2))
        assert all(r['equal'] for r in result.values()),result
    elif args.phase=='transitions':
        for label,binary in [('transitions-baseline',baseline),('transitions-candidate',candidate)]:
            target=folder(label)
            run(binary,['--benchmark-render','transition-captures'],target,'capture',
                {'NORD_TRANSITION_CAPTURE':str(target/'frames'),'NORD_BENCH_SECONDS':'1800',
                 'NORD_BENCH_SCENE':'cockpit','NORD_BENCH_LIVE':'0'},timeout=450)
        subprocess.run([sys.executable,'tests/optimization4/compare_transitions.py',
            str(OUT/'transitions-baseline/frames'),str(OUT/'transitions-candidate/frames'),str(OUT/'transition-review')],cwd=ROOT,check=True)
    elif args.phase=='stability':
        target=folder('stability')
        run(candidate,['--benchmark-render','soak'],target,'soak',
            {'NORD_BENCH_SCENE':'bvr-cockpit-missiles','NORD_BENCH_LIVE':'1','NORD_BENCH_WARMUP':'5',
             'NORD_BENCH_SECONDS':'120','NORD_BENCH_SOAK':'1','NORD_SOAK_ATMOSPHERE':'1',
             'NORD_SOAK_SEGMENT_SECONDS':'100','NORD_BENCH_REPORT':str(target/'soak.json')},timeout=210)
    elif args.phase=='pacing':
        for latency in ['1','2']:
            for scene in ['cockpit','bvr']:render(candidate,'latency-'+latency,scene,{'NORD_FRAME_LATENCY':latency})
    elif args.phase=='trace':
        target=folder('metal')
        options=target/'recording-options.json'
        options.write_text(json.dumps({'Hangs':{'detectPriorityInversions':False,'hangsThreshold':100},
            'Time Profiler':{'recordWaitingThreads':True,'contextSwitchSampling':True,
                'highFrequencySampling':False,'recordKernelStacks':False}}))
        for scene in ['cockpit','bvr']:
            argv=['xcrun','xctrace','record','--template','Metal System Trace','--time-limit','18s',
                '--window','10s','--recording-options',str(options),
                '--output',str(target/(scene+'.trace'))]
            env={'NORD_BENCH_SCENE':'cockpit' if scene=='cockpit' else 'bvr-cockpit-missiles',
                 'NORD_BENCH_LIVE':'0' if scene=='cockpit' else '1','NORD_BENCH_WARMUP':'5','NORD_BENCH_SECONDS':'30'}
            with (target/(scene+'-app.log')).open('w') as app_log, (target/(scene+'-trace.log')).open('w') as log:
                app=subprocess.Popen([str(candidate),'--benchmark-render','metal-'+scene],cwd=ROOT,
                    env=dict(os.environ,**env),stdout=app_log,stderr=subprocess.STDOUT)
                record_child(app.pid)
                try:
                    time.sleep(6)
                    process=subprocess.Popen(argv+['--attach',str(app.pid)],cwd=ROOT,stdout=log,stderr=subprocess.STDOUT)
                    try:result=process.wait(timeout=100)
                    except subprocess.TimeoutExpired:process.terminate();process.wait(timeout=10);result=-1
                finally:
                    if app.poll() is None:
                        app.terminate()
                        try:app.wait(timeout=5)
                        except subprocess.TimeoutExpired:app.kill();app.wait()
                    record_child(None)
            (target/(scene+'-result.json')).write_text(json.dumps({'exit':result}))
            if result:break
    (OUT/(args.phase+'-identity.json')).write_text(json.dumps({'candidate':str(candidate),
        'sha256':hashlib.sha256(candidate.read_bytes()).hexdigest(),'power':command(['pmset','-g','batt']),
        'thermal':command(['pmset','-g','therm']),'date':command(['date','-u'])},indent=2))
