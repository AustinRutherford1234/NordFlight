"""Additional controlled checks, strictly after the primary timing suite."""
import argparse
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time
from run import OUT, ROOT, identity, same_process

p = argparse.ArgumentParser()
p.add_argument("--prefix", default="accepted-v2")
p.add_argument("--wait", action="store_true")
p.add_argument("--primary-pid", type=int)
args = p.parse_args()
folder = OUT / args.prefix
base = OUT / "baseline/gripen-flight-release"
candidate = OUT / "candidate/gripen-flight-release"
child = None
signal.signal(signal.SIGTERM, lambda *_: (_ for _ in ()).throw(KeyboardInterrupt()))
def label(name): return str(Path(args.prefix) / name)
def stage(name, arguments, extra=None):
    global child
    print("START " + name, flush=True)
    (folder / "post-progress.json").write_text(json.dumps({"stage":name,"status":"running"},indent=2))
    env = os.environ.copy();env["NORD_COMPUTE_THREADS"]="6";env.update(extra or {})
    child = subprocess.Popen([sys.executable, *map(str,arguments)],cwd=ROOT,env=env)
    code = child.wait();child = None
    if code: raise RuntimeError(f"{name} failed: {code}")
    print("DONE " + name, flush=True)
def suite(binary,name,kind,*extra,env=None):
    stage(name,[OUT/"run.py",binary,label(name),kind,*extra],env)

try:
    primary = identity(args.primary_pid) if args.primary_pid else None
    while True:
        try:
            progress=json.loads((folder/"final-progress.json").read_text())
        except (OSError,json.JSONDecodeError):
            if not args.wait or not same_process(primary): raise
            time.sleep(1)
            continue
        if progress == {"stage":"all-suites","status":"complete"}: break
        if not args.wait or progress["status"] == "failed" or not same_process(primary):
            raise RuntimeError("Primary validation is not complete; no additional benchmark was started")
        (folder/"post-progress.json").write_text(json.dumps({"stage":"waiting-primary","status":"running"},indent=2))
        time.sleep(30)
    stage("high-scattering-msaa",[OUT/"matrix.py",base,candidate,label("release-high-quality"),"--scenes","bvr,earth60,orbit","--paused","--quality"])
    for report in (folder/"release-high-quality").glob("*/render-*-1.json"):
        data=json.loads(report.read_text());assert data["quality"] and data["paused"] and data["resolution"] == [1440,900]
    stage("bindings-disabled",[OUT/"profile.py",candidate,label("metal-bindings-disabled")],{"BEVY_DISABLE_ENV_BIND_GROUP_CACHE":"1"})
    name="motion-ascent-cpu-extended"
    suite(candidate,name,"render","--scenes","ascent",env={"NORD_EARTH_CPU":"1","NORD_BENCH_SECONDS":"130","NORD_BENCH_WARMUP":"1","NORD_CAPTURE_SEQUENCE":str(folder/name/"frames")})
    for binary,name in ((base,"extra-effects-baseline"),(candidate,"extra-effects-candidate")):
        suite(binary,name,"visual","--visuals","combat-impact,air-component-collision,air-component-cockpit-simple")
    stage("effects-compare",[OUT/"visual_compare.py",folder/"extra-effects-baseline",folder/"extra-effects-candidate",folder/"effects-visual-equivalence.json"])
    for script,reference in (("ascent","motion-ascent-cpu-extended"),("descent","motion-descent-cpu")):
        stage("motion-compare-"+script,[OUT/"motion_compare.py",folder/reference/"frames",folder/("motion-"+script+"-gpu")/"frames",folder/("motion-"+script+"-comparison.json")])
    stage("harness-tests",["-m","unittest","discover","-s",OUT,"-p","test_harness.py"])
    (folder/"post-progress.json").write_text(json.dumps({"stage":"additional-checks","status":"complete"},indent=2))
    print("Additional checks complete. Review regressions and native captures before packaging.",flush=True)
except BaseException:
    if child is not None and child.poll() is None:
        child.terminate();child.wait(timeout=30)
    (folder/"post-progress.json").write_text(json.dumps({"stage":"additional-checks","status":"failed"},indent=2))
    raise
