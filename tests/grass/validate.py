"""Isolated 2D ground-cover visuals and sequential release on/off profiles."""
import hashlib,json,shutil,sys,time
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];sys.path.insert(0,str(ROOT/'tests/optimization3'))
from run import isolated,run
out=ROOT/'tests/grass'/sys.argv[1];out.mkdir(parents=True,exist_ok=True)
binary=ROOT/'Nord Flight.app/Contents/MacOS/gripen-flight'
config={'version':1,'render_scale':0.5,'frame_rate_limit':0,'terrain_error_pixels':4.,'vegetation_distance':4500.,'grass_distance':112.,'grass_density':0.65,'shadow_distance':1800.,'texture_detail':512,'cpu_cache_mib':384,'gpu_cache_mib':384,'staging_mib':128}
(out/'graphics.json').write_text(json.dumps(config,indent=2))
(out/'executable.json').write_text(json.dumps({'sha256':hashlib.sha256(binary.read_bytes()).hexdigest()},indent=2))
common={'NORD_ENVIRONMENT':'earth','NORD_GRAPHICS_CONFIG':str(out/'graphics.json'),'NORD_TERRAIN_X':'-8000','NORD_TERRAIN_Z':'0','NORD_TERRAIN_AGL':'2','RUST_LOG':'warn'}
with isolated():
 run(binary,['--study-ground-cover'],out,'placement',{'NORD_ENVIRONMENT':'earth'},timeout=60)
 shutil.copy2(ROOT/'tests/grass/placement.json',out/'placement.json')
 for label,extra in [('earth',{}),('off',{'NORD_GRASS_OFF':'1'}),('scifi',{'NORD_ENVIRONMENT':'scifi-earth'}),('settings',{'NORD_MENU_CAPTURE':'settings'})]:
  env={**common,'NORD_RENDER_SCALE':'1',**extra};stamp=time.time_ns()
  run(binary,['--visual-check','terrain-forest'],out,label,env,timeout=180)
  source=ROOT/'tests/visual-terrain-forest.png'
  if source.stat().st_mtime_ns<stamp:raise RuntimeError('Stale capture')
  shutil.copy2(source,out/(label+'.png'))
 for label,extra in [('profile-off',{'NORD_GRASS_OFF':'1'}),('profile-on',{}),('stream-on',{'NORD_BENCH_SCRIPT':'terrain-stream','NORD_TERRAIN_SPEED':'230','NORD_TERRAIN_AGL':'10'})]:
  env={**common,'NORD_BENCH_FULLSCREEN':'1','NORD_BENCH_SCENE':'terrain-forest','NORD_BENCH_WARMUP':'15','NORD_BENCH_SECONDS':'20','NORD_BENCH_REPORT':str(out/(label+'.json')),**extra}
  run(binary,['--benchmark-render','base'],out,label,env,timeout=180)
