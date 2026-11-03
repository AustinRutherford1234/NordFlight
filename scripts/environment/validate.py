#!/usr/bin/env python3
"""Sequential native environment measurements. Never runs a compiler.

Reuses the project's recoverable app-process isolation; no other applications
are suspended. Baseline and candidate resolutions are explicit in reports.
"""
import argparse,hashlib,json,os,subprocess,sys,time,threading
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tests/optimization3'))
from run import isolated,record_child
p=argparse.ArgumentParser()
p.add_argument('binary',type=Path);p.add_argument('output',type=Path)
p.add_argument('--seconds',type=int,default=30);p.add_argument('--warmup',type=int,default=15)
p.add_argument('--environment',default='earth');p.add_argument('--scale',default='0.5')
p.add_argument('--case',action='append',choices=['forest-external','forest-cockpit','coast','mountain','airfield','carrier','combat','soak'])
p.add_argument('--windowed',action='store_true');p.add_argument('--profile',action='store_true')
a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True)
binary=a.binary.resolve();summary={}
cases=a.case or ['forest-external','forest-cockpit','coast','mountain','airfield','carrier','combat']
with isolated():
 for name in cases:
    env=os.environ.copy();report=(a.output/(name+'.json')).resolve()
    if report.exists():raise SystemExit('Use a fresh output directory')
    env.update(NORD_ENVIRONMENT=a.environment,NORD_RENDER_SCALE=a.scale,NORD_BENCH_REPORT=str(report),
        NORD_BENCH_WARMUP=str(a.warmup),NORD_BENCH_SECONDS=str(a.seconds),
        NORD_BENCH_SCENE='terrain-forest',NORD_BENCH_SCRIPT='terrain-stream',NORD_TERRAIN_AGL='100',
        NORD_TERRAIN_SPEED='180',NORD_BENCH_LIVE='0',NORD_FLIGHT_AUDIO_SETTINGS=str(ROOT/'tests/optimization3/audio-settings.json'))
    if not a.profile:env['NORD_BENCH_FPS_ONLY']='1'
    if not a.windowed:env['NORD_BENCH_FULLSCREEN']='1'
    if name=='forest-cockpit':env['NORD_TERRAIN_COCKPIT']='1'
    if name=='coast':env.update(NORD_TERRAIN_X='106800',NORD_TERRAIN_Z='-88200')
    if name=='mountain':env.update(NORD_TERRAIN_X='292700',NORD_TERRAIN_Z='301150',NORD_TERRAIN_AGL='500')
    if name=='airfield':env.update(NORD_TERRAIN_X='0',NORD_TERRAIN_Z='-3000',NORD_TERRAIN_AGL='100')
    if name=='carrier':env.update(NORD_BENCH_SCRIPT='carrier-live',NORD_BENCH_SCENE='cockpit',NORD_BENCH_LIVE='1')
    if name=='combat':env.update(NORD_BENCH_SCRIPT='combat',NORD_BENCH_SOAK='1',NORD_SOAK_SEGMENT_SECONDS='100000',NORD_SOAK_ATMOSPHERE='1',NORD_BENCH_SCENE='air-component-fire')
    if name=='soak':env.update(NORD_BENCH_SOAK='1',NORD_SOAK_ATMOSPHERE='1',NORD_SOAK_SEGMENT_SECONDS='100')
    power=subprocess.run(['pmset','-g','batt'],capture_output=True,text=True).stdout
    with (a.output/(name+'.log')).open('w')as log:
        child=subprocess.Popen([str(binary),'--benchmark-render','terrain'],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT)
        record_child(child.pid)
        stop=threading.Event()
        def telemetry():
            with (a.output/(name+'-process.jsonl')).open('w')as samples:
                start=time.monotonic()
                while not stop.is_set():
                    process=subprocess.run(['ps','-p',str(child.pid),'-o','rss=,pcpu='],capture_output=True,text=True).stdout.strip()
                    power=subprocess.run(['pmset','-g','batt'],capture_output=True,text=True).stdout.strip()
                    samples.write(json.dumps(dict(wall_s=time.monotonic()-start,process=process,power=power))+'\n');samples.flush()
                    stop.wait(5)
        monitor=threading.Thread(target=telemetry,daemon=True);monitor.start()
        try:code=child.wait(timeout=a.warmup+a.seconds+180)
        finally:
            if child.poll()is None:child.terminate();child.wait(timeout=15)
            stop.set();monitor.join(timeout=10);record_child(None)
    if code:raise SystemExit(f'{name} failed: {code}; inspect {log.name}')
    if not report.exists():raise SystemExit(f'{name} exited without a benchmark report; inspect {log.name}')
    data=json.loads(report.read_text())
    summary[name]={k:data.get(k)for k in ['resolution','world_render_scale','environment','mean_fps','p50_ms','p95_ms','p99_ms','over_25_ms','over_50_ms','simulation_timing','asset_memory']}
    summary[name]['power']=power
    (a.output/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
    print(name,summary[name],flush=True)
(a.output/'executable.json').write_text(json.dumps(dict(path=str(binary),sha256=hashlib.sha256(binary.read_bytes()).hexdigest()),indent=2)+'\n')
