"""Short sequential visual-update guardrail, not a full performance campaign."""
from pathlib import Path
import importlib.util,json,hashlib

root=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('native',root/'tests/optimization3/run.py')
native=importlib.util.module_from_spec(spec);spec.loader.exec_module(native)
folder=Path(__file__).parent/'render-check';folder.mkdir(exist_ok=False)
records=[]
with native.isolated():
    for label,binary in [('baseline',Path(__file__).parent/'baseline/gripen-flight'),
                         ('candidate',root/'Nord Flight.app/Contents/MacOS/gripen-flight')]:
        report=folder/(label+'.json')
        env={'NORD_AIRCRAFT':'f22','NORD_BENCH_SCENE':'air-hud','NORD_BENCH_LIVE':'0',
             'NORD_BENCH_FPS_ONLY':'1','NORD_BENCH_WARMUP':'10','NORD_BENCH_SECONDS':'15',
             'NORD_BENCH_REPORT':str(report)}
        native.run(binary,['--benchmark-render','f22-mainbay-check'],folder,label,env,timeout=120)
        data=json.loads(report.read_text())
        records.append({'label':label,'sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),
                        'mean_ms':data['mean_ms'],'p95_ms':data['p95_ms'],
                        'p99_ms':data['p99_ms'],'mean_fps':data['mean_fps'],
                        'resolution':data['resolution'],'over_50_ms':data['over_50_ms']})
summary={'workload':'paused F-22 atmospheric external view','warmup_seconds':10,'sample_seconds':15,
         'limitations':'One sequential pair on battery; a guardrail, not a statistically established optimization result.',
         'runs':records}
summary['mean_frame_time_change_percent']=(records[1]['mean_ms']/records[0]['mean_ms']-1)*100
(folder/'summary.json').write_text(json.dumps(summary,indent=2));print(json.dumps(summary,indent=2))
