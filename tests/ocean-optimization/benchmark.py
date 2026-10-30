"""Approved alternating 3x30-second native ocean comparison; no concurrent builds."""
import os,signal,subprocess,json,hashlib,statistics,argparse
from pathlib import Path
root=Path(__file__).resolve().parents[2];base=root/'tests/ocean-optimization'
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--final-pair',action='store_true');args=p.parse_args()
out=base/('final' if args.final_pair else 'controlled');out.mkdir(exist_ok=True)
executables={'baseline':base/'A/Nord Flight.app/Contents/MacOS/gripen-flight','optimized':base/'B/Nord Flight.app/Contents/MacOS/gripen-flight'}
for p in executables.values():assert p.is_file(),f'Missing executable: {p}'
summary={'executable_sha256':{k:hashlib.sha256(p.read_bytes()).hexdigest()for k,p in executables.items()},'runs':[]};paused=[]
try:
 for pid in map(int,subprocess.run(['pgrep','-x','gripen-flight'],capture_output=True,text=True).stdout.split()):
  state=subprocess.run(['ps','-o','stat=','-p',str(pid)],capture_output=True,text=True).stdout.strip()
  if state and not state.startswith('T'):os.kill(pid,signal.SIGSTOP);paused.append(pid)
 for index,variant in enumerate(['baseline','optimized'] if args.final_pair else ['baseline','optimized','optimized','baseline','baseline','optimized']):
  name=f'{index+1}-{variant}';report=out/(name+'.json')
  env=dict(os.environ,NORD_CARRIER_CAPTURE='ocean-approach',NORD_BENCH_SCENE='dry',NORD_BENCH_SCRIPT='ocean',NORD_BENCH_FULLSCREEN='1',NORD_PRESENT_MODE='vsync',NORD_BENCH_WARMUP='20',NORD_BENCH_SECONDS='30',NORD_BENCH_FPS_ONLY='1',NORD_BENCH_REPORT=str(report))
  for key in ['NORD_OCEAN_REFERENCE','NORD_FLEET_CAPTURE','NORD_CREW_CAPTURE']:env.pop(key,None)
  with (out/(name+'.log')).open('w')as log:subprocess.run([str(executables[variant]),'--benchmark-render','dry'],cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=140)
  data=json.loads(report.read_text());data['window_fps']=data['frames']/data['sample_seconds'];data['window_mean_ms']=1000./data['window_fps'];summary['runs'].append({'variant':variant,'report':report.name,**{k:data[k]for k in ['frames','resolution','quality','mean_ms','mean_fps','window_mean_ms','window_fps','p50_ms','p95_ms','p99_ms']}})
  (out/'summary.json').write_text(json.dumps(summary,indent=2)+'\n');print(name,summary['runs'][-1],flush=True)
 for variant in executables:
  rows=[r for r in summary['runs']if r['variant']==variant];summary[variant]={k:statistics.mean(r[k]for r in rows)for k in ['mean_ms','mean_fps','window_mean_ms','window_fps','p50_ms','p95_ms','p99_ms']}
 (out/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
finally:
 for pid in paused:
  try:os.kill(pid,signal.SIGCONT)
  except ProcessLookupError:pass
 print('Original game sessions resumed.',flush=True)
