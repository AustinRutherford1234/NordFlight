"""One focused, sequential end-of-pass validation; never compiles or changes quality."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import sys
import time

ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tests/optimization3'))
from run import isolated, run, STUDIES, SCENES, command
from verify import physics

parser=argparse.ArgumentParser()
parser.add_argument('candidate',type=Path)
parser.add_argument('--phase',choices=['visual','final','terrain','bvr-repeat','shipping'],default='final')
args=parser.parse_args()
out=ROOT/'tests/optimization5'
baseline=out/'baseline/gripen-flight'
candidate=args.candidate.resolve()

def visual(binary,folder,scene,extra=None):
    source=ROOT/'tests'/f'visual-{scene}.png'
    before=source.stat().st_mtime_ns if source.exists() else 0
    run(binary,['--visual-check',scene],folder,'visual-'+scene,extra)
    assert source.exists() and source.stat().st_mtime_ns>before
    shutil.copy2(source,folder/source.name)

with isolated():
    if args.phase=='shipping':
        # Validate the accepted default without inherited developer overrides.
        os.environ.pop('NORD_SMOKE_GPU',None);os.environ.pop('NORD_SMOKE_CPU',None)
        folder=out/'shipping';folder.mkdir(exist_ok=True)
        (folder/'identity.json').write_text(json.dumps({'binary':str(candidate),'sha256':hashlib.sha256(candidate.read_bytes()).hexdigest(),'smoke':'default CPU'},indent=2))
        run(candidate,['--benchmark-render','shipping-bvr'],folder,'bvr',{
            'NORD_BENCH_SCENE':'bvr-cockpit-missiles','NORD_BENCH_LIVE':'1','NORD_BENCH_SCRIPT':'bvr',
            'NORD_BENCH_WARMUP':'10','NORD_BENCH_SECONDS':'25','NORD_BENCH_REPORT':str(folder/'bvr.json')})
        visual(candidate,folder,'cockpit',{'NORD_AIRCRAFT':'f22'})
    elif args.phase=='visual':
        for label,extra in [('gpu',{'NORD_SMOKE_GPU':'1'}),('cpu',{'NORD_SMOKE_CPU':'1'})]:
            folder=out/('visual-'+label);folder.mkdir(exist_ok=True)
            visual(candidate,folder,'combat-flight',dict(extra,NORD_AIRCRAFT='f22',NORD_MISSILE_KIND='radar'))
        for scene in ['cockpit','terrain-forest','lunar-node-map']:
            visual(candidate,out/'visual-gpu',scene,{'NORD_AIRCRAFT':'f22'} if scene=='cockpit' else None)
    elif args.phase in ('terrain','bvr-repeat'):
        # The ordinary forest preset starts stationary and can become a wreck.
        # Measure streaming with the existing deterministic presentation path.
        for label,binary in [('baseline',baseline),('candidate',candidate)]:
            folder=out/('render-'+label);folder.mkdir(exist_ok=True)
            tag='terrain-stream' if args.phase=='terrain' else 'bvr-repeat'
            extra={'NORD_BENCH_SCENE':'terrain-forest','NORD_BENCH_LIVE':'0',
                'NORD_BENCH_SCRIPT':'terrain-stream','NORD_TERRAIN_AGL':'50','NORD_TERRAIN_SPEED':'220'} if args.phase=='terrain' else {
                'NORD_BENCH_SCENE':'bvr-cockpit-missiles','NORD_BENCH_LIVE':'1','NORD_BENCH_SCRIPT':'bvr'}
            run(binary,['--benchmark-render',tag],folder,tag,dict(extra,
                NORD_SMOKE_GPU='1',NORD_BENCH_WARMUP='10',NORD_BENCH_SECONDS='25',NORD_BENCH_REPORT=str(folder/(tag+'.json'))))
    else:
        identity={'candidate':str(candidate),'sha256':hashlib.sha256(candidate.read_bytes()).hexdigest(),
                  'power':command(['pmset','-g','batt']),'thermal':command(['pmset','-g','therm']),
                  'date':command(['date','-u']), 'render_protocol':'experimental GPU candidate; one baseline/candidate pair per scene, 10s warmup and 25s sample; bounded check, not a broad campaign'}
        (out/'candidate.json').write_text(json.dumps(identity,indent=2))
        for label,binary in [('baseline',baseline),('candidate',candidate)]:
            folder=out/('cpu-'+label);folder.mkdir(exist_ok=True)
            for name,reports in STUDIES.items():
                run(binary,['--study-'+name],folder,name,timeout=600)
                for report in reports:shutil.copy2(ROOT/'tests'/report,folder/report)
        result=physics(out/'cpu-baseline',out/'cpu-candidate')
        (out/'equivalence.json').write_text(json.dumps(result,indent=2))
        assert all(r['equal'] for r in result.values()),result
        for scene in ['cockpit','bvr','terrain']:
            for label,binary in [('baseline',baseline),('candidate',candidate)]:
                folder=out/('render-'+label);folder.mkdir(exist_ok=True)
                preset,live=SCENES[scene]
                report=folder/(scene+'.json')
                run(binary,['--benchmark-render',scene],folder,scene,{'NORD_BENCH_SCENE':preset,
                    'NORD_SMOKE_GPU':'1',
                    'NORD_BENCH_LIVE':str(int(live)),'NORD_BENCH_SCRIPT':scene,
                    'NORD_BENCH_WARMUP':'10','NORD_BENCH_SECONDS':'25','NORD_BENCH_REPORT':str(report)})
                assert report.exists()
        folder=out/'stability';folder.mkdir(exist_ok=True)
        run(candidate,['--benchmark-render','soak'],folder,'soak',{'NORD_BENCH_SCENE':'bvr-cockpit-missiles',
            'NORD_SMOKE_GPU':'1',
            'NORD_BENCH_LIVE':'1','NORD_BENCH_WARMUP':'5','NORD_BENCH_SECONDS':'120',
            'NORD_BENCH_SOAK':'1','NORD_SOAK_SEGMENT_SECONDS':'10','NORD_BENCH_REPORT':str(folder/'soak.json')},timeout=190)
print('Focused validation complete',flush=True)
