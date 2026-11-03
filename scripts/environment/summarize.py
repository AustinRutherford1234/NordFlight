#!/usr/bin/env python3
"""Summarize native timing/soak evidence without claiming GPU-memory accounting."""
import argparse,json
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('directory',type=Path)
a=p.parse_args();result={}
for path in sorted(a.directory.glob('*.json')):
    try:data=json.loads(path.read_text())
    except json.JSONDecodeError:continue
    if 'p95_ms' not in data:continue
    events=data.get('exercise_events',[])
    groups={}
    for event in events:
        if event.get('event')!='sample':continue
        phase=str(event['phase']);group=groups.setdefault(phase,{'script':event['script'],'samples':0})
        group['samples']+=1
        for key in ['entities','meshes','images','particles','missiles','aircraft']:
            if key in event:
                bounds=group.setdefault(key,[event[key],event[key]])
                bounds[0]=min(bounds[0],event[key]);bounds[1]=max(bounds[1],event[key])
    records=[]
    telemetry=path.with_name(path.stem+'-process.jsonl')
    if telemetry.exists():records=[json.loads(line)for line in telemetry.read_text().splitlines()]
    rss=[int(r['process'].split()[0])for r in records if r.get('process')]
    power=list(dict.fromkeys(r['power'].splitlines()[0]for r in records if r.get('power')))
    result[path.stem]={
        'frame_target_met':data['p95_ms']<=18 and data['p99_ms']<=25,
        'mean_fps':data['mean_fps'],'p95_ms':data['p95_ms'],'p99_ms':data['p99_ms'],
        'sample_seconds':data['sample_seconds'],'framebuffer':data['resolution'],
        'world_render_scale':data.get('world_render_scale'),
        'discarded_simulated_s':data.get('simulation_timing',{}).get('whole_run_dropped_simulated_s'),
        'telemetry_duration_s':records[-1]['wall_s'] if records else None,
        'rss_mib_range':[min(rss)/1024,max(rss)/1024]if rss else None,
        'rss_mib_last':rss[-1]/1024 if rss else None,'power_sources':power,
        'phases':groups,
        'note':'App intervals and retained assets are not display-presentation timings or a total GPU allocation measurement.'}
print(json.dumps(result,indent=2))
