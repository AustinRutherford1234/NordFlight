"""Run real-terrain diagnostics in separate processes for immutable environments."""
import json,os,shutil,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];OUT=ROOT/'tests/mountain-combat'
# This executable is produced by cargo test; choose the current package test target.
programs=[p for p in (ROOT/'target/debug/deps').glob('gripen_flight-*') if p.is_file() and os.access(p,os.X_OK)]
binary=max(programs,key=lambda p:p.stat().st_mtime_ns)
cases=[(d,'earth',{})for d in ['training','balanced','hard']]+[('balanced','scifi-earth',{'NORD_MOUNTAIN_CASE':'4:0'}),('balanced','earth',{'NORD_MOUNTAIN_CASE':'4:0','NORD_MOUNTAIN_WEAPONS':'1'})]
for difficulty,planet,extra in cases:
 tag=planet+'-'+difficulty+('-weapons'if extra.get('NORD_MOUNTAIN_WEAPONS')else'')
 env=os.environ.copy();env.update(NORD_ENVIRONMENT=planet,NORD_MOUNTAIN_DIFFICULTY=difficulty,NORD_MOUNTAIN_SECONDS='120');env.update(extra)
 with (OUT/(tag+'.log')).open('w')as log:
  result=subprocess.run([str(binary),'real_pack_mountain_survival_study','--ignored','--nocapture','--test-threads=1'],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT)
 shutil.copy2(OUT/'terrain-study.json',OUT/('terrain-study-'+tag+'.json'))
 if result.returncode:raise SystemExit(f'{tag} failed: inspect its log')
 rows=json.loads((OUT/'terrain-study.json').read_text())
 print(tag,':',len(rows),'cases; minimum AI clearance',min(r['minimum_ai_agl']for r in rows),'m; AI losses',sum(len(r['crashed_ai'])for r in rows),'missiles',sum(r['missiles_launched']for r in rows),flush=True)
