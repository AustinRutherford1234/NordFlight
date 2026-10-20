"""Focused, sequential atmospheric validation; reuses the identity-safe runner."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import statistics
import sys

ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'tests/optimization4'
sys.path.insert(0,str(ROOT/'tests/optimization3'))
import run as native

SCENES={
    'cockpit-paused':('cockpit',False),
    'cockpit-live':('cockpit',True),
    'airborne':('air-hud',True),
    'bvr':('bvr-cockpit-missiles',True),
    'terrain':('terrain-forest',True),
    'earth60':('earth-flight-60',True),
}

def render(binary,folder,name,scene,live,extra=None,warmup=15,seconds=30):
    folder.mkdir(parents=True,exist_ok=True)
    report=folder/(name+'.json')
    config={'binary':str(binary),'sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),
            'scene':scene,'live':live,'warmup':warmup,'seconds':seconds,'extra':extra or {}}
    manifest=folder/(name+'-config.json')
    if report.exists():
        if json.loads(manifest.read_text())!=config: raise RuntimeError('Resume configuration changed')
        if json.loads((folder/(name+'-process.json')).read_text())['exit']!=0: raise RuntimeError('Failed prior run')
        return json.loads(report.read_text())
    manifest.write_text(json.dumps(config,indent=2)+'\n')
    env={'NORD_BENCH_SCENE':scene,'NORD_BENCH_LIVE':str(int(live)),
         'NORD_BENCH_SCRIPT':'','NORD_BENCH_WARMUP':str(warmup),'NORD_BENCH_SECONDS':str(seconds),
         'NORD_BENCH_REPORT':str(report)}
    env.update(extra or {})
    native.run(binary,['--benchmark-render',name],folder,name,env,timeout=warmup+seconds+90)
    return json.loads(report.read_text())

def main():
    p=argparse.ArgumentParser()
    p.add_argument('binary',type=Path)
    p.add_argument('label')
    p.add_argument('suite',choices=['smoke','matrix','transitions','soak','pacing'])
    p.add_argument('--scenes',default=','.join(SCENES))
    p.add_argument('--reference',action='store_true')
    p.add_argument('--legacy',action='store_true')
    args=p.parse_args(); binary=args.binary.resolve(); folder=OUT/args.label
    folder.mkdir(parents=True,exist_ok=True)
    env={'NORD_ENV_STAGES':'1'} if args.reference else {}
    if args.legacy: env.update(NORD_ENV_LEGACY_ORDER='1',BEVY_DISABLE_ENV_CONTENT_CACHE='1',
        BEVY_DISABLE_ENV_BIND_GROUP_CACHE='1',BEVY_DISABLE_STATIC_ATMOSPHERE_CACHE='1')
    with native.isolated():
        if args.suite=='transitions':
            native.run(binary,['--benchmark-render','transition-captures'],folder,'transitions',
                dict(env,NORD_TRANSITION_CAPTURE=str(folder/'frames'),NORD_BENCH_SECONDS='1800',
                     NORD_BENCH_SCENE='cockpit',NORD_BENCH_LIVE='0'),timeout=450)
        elif args.suite=='soak':
            render(binary,folder,'soak','bvr-cockpit-missiles',True,
                # The inherited exercise uses camera windows at 60s and timestep
                # windows at 80s. A 100s segment reaches those checks, including
                # cockpit/map views, then returns to BVR at the end of the soak.
                dict(env,NORD_BENCH_SOAK='1',NORD_SOAK_ATMOSPHERE='1',NORD_SOAK_SEGMENT_SECONDS='100'),warmup=15,seconds=600)
        elif args.suite=='matrix':
            baseline=OUT/'baseline/gripen-flight-release'
            summary={}
            for scene in args.scenes.split(','):
                preset,live=SCENES[scene]; versions={'a':[],'b':[]}
                for version,repeat in [('a',1),('b',1),('b',2),('a',2)]:
                    executable=baseline if version=='a' else binary
                    result=render(executable,folder,f'{scene}-{version}{repeat}',preset,live,env)
                    versions[version].append(result)
                summary[scene]={v:{key:statistics.median(r[key] for r in records)
                    for key in ['mean_fps','p50_ms','p95_ms','p99_ms','simulation_rate','over_25_ms','over_50_ms']}
                    for v,records in versions.items()}
                (folder/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
                print(scene,summary[scene],flush=True)
        else:
            for scene in args.scenes.split(','):
                preset,live=SCENES[scene]
                settings=dict(env)
                if args.suite=='pacing': settings['NORD_PRESENT_MODE']='no-vsync'
                render(binary,folder,scene,preset,live,settings,warmup=8,seconds=12)

if __name__=='__main__':
    signal.signal(signal.SIGTERM,lambda *_: (_ for _ in ()).throw(KeyboardInterrupt()))
    main()
