"""Short Xcode captures; these profiled runs are not FPS benchmarks."""
import argparse
import hashlib
import json
import os
import shutil
import signal
from pathlib import Path
import subprocess
import sys
import time
from run import ROOT,OUT,isolated,record_child

parser=argparse.ArgumentParser()
parser.add_argument("binary",type=Path)
parser.add_argument("label")
parser.add_argument("--template",default="Metal System Trace")
parser.add_argument("--scene",default="cockpit")
parser.add_argument("--live",action="store_true")
parser.add_argument("--debuggable",action="store_true",help="Sign a disposable profiling copy with get-task-allow; never change the delivered binary")
args=parser.parse_args()
signal.signal(signal.SIGTERM, lambda *_: (_ for _ in ()).throw(KeyboardInterrupt()))
folder=OUT/args.label;folder.mkdir(parents=True,exist_ok=True)
trace=folder/"profile.trace"
if trace.exists():raise SystemExit("Use a fresh profile label")
binary = args.binary.resolve()
temporary_binary = None
if args.debuggable:
    temporary_binary = folder / "gripen-flight-profile"
    shutil.copy2(binary, temporary_binary)
    subprocess.run(["codesign", "--force", "--sign", "-", "--entitlements", str(OUT / "profiling.entitlements"), str(temporary_binary)], check=True)
    binary = temporary_binary.resolve()
env=os.environ.copy();env.update(NORD_BENCH_SCENE=args.scene,NORD_BENCH_LIVE=str(int(args.live)),NORD_BENCH_SECONDS="120",NORD_FLIGHT_AUDIO_SETTINGS=str(OUT/"audio-settings.json"))
(folder/"capture.json").write_text(json.dumps({"binary":str(args.binary.resolve()),"sha256":hashlib.sha256(args.binary.read_bytes()).hexdigest(),"profile_binary_sha256":hashlib.sha256(binary.read_bytes()).hexdigest(),"disposable_debug_entitlement":args.debuggable,"template":args.template,"scene":args.scene,"live":args.live,"warmup_seconds":20,"capture_seconds":5},indent=2)+"\n")
with isolated(),(folder/"game.log").open("w") as log:
    game=subprocess.Popen([str(binary),"--benchmark-render","profile"],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT)
    record_child(game.pid)
    try:
        time.sleep(20)
        with (folder/"capture.log").open("w") as output:
            result=subprocess.run(["xctrace","record","--template",args.template,"--time-limit","5s","--output",str(trace),"--attach",str(game.pid)],stdout=output,stderr=subprocess.STDOUT,timeout=180)
        print(f"Capture exit {result.returncode}: {folder}",flush=True)
        if result.returncode:raise SystemExit(result.returncode)
        with (folder/"toc.xml").open("w") as toc:
            subprocess.run(["xctrace","export","--input",str(trace),"--toc"],stdout=toc,check=True,timeout=60)
    finally:
        game.terminate()
        try:game.wait(timeout=10)
        except subprocess.TimeoutExpired:game.kill();game.wait()
        record_child(None)
        if temporary_binary is not None:
            temporary_binary.unlink(missing_ok=True)

# Export only after the game has stopped, and before the orchestrator starts
# an unprofiled benchmark. Export/analysis CPU load must not overlap FPS samples.
if args.template == "Metal System Trace":
    for schema, filename in [("metal-gpu-intervals", "gpu.xml"), ("metal-resource-allocations", "resources.xml")]:
        with (folder / (filename + ".log")).open("w") as output:
            subprocess.run(["xctrace", "export", "--input", str(trace), "--xpath",
                            '/trace-toc/run[@number="1"]/data/table[@schema="' + schema + '"]',
                            "--output", str(folder / filename)], stdout=output, stderr=subprocess.STDOUT,
                           check=True, timeout=120)
    with (folder / "summary-console.json").open("w") as output:
        subprocess.run([sys.executable, str(OUT / "trace_summary.py"), str(folder)], stdout=output,
                       check=True, timeout=60)
elif args.template == "Time Profiler":
    import xml.etree.ElementTree as ET
    schemas = [table.attrib.get("schema", "") for table in ET.parse(folder / "toc.xml").iter("table")]
    (folder / "schemas.json").write_text(json.dumps(schemas, indent=2) + "\n")
    if "time-profile" in schemas:
        with (folder / "cpu-export.log").open("w") as output:
            subprocess.run(["xctrace", "export", "--input", str(trace), "--xpath",
                            '/trace-toc/run[@number="1"]/data/table[@schema="time-profile"]',
                            "--output", str(folder / "cpu.xml")], stdout=output, stderr=subprocess.STDOUT,
                           check=True, timeout=120)
