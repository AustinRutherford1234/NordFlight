"""Short, explicitly approved native salvo profile; always restore user sessions."""
from pathlib import Path
import argparse
import json
import os
import signal
import subprocess
import time
import re

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "tests/missile-spikes"
parser = argparse.ArgumentParser()
parser.add_argument("--label", choices=["before", "after", "cpu-warmed", "cpu-soak", "cpu-check", "cpu-live-repeat", "cpu-cached", "memory-after", "memory-validated"], default="after")
parser.add_argument("--seconds", type=float, default=30.)
parser.add_argument("--warmup", type=float, default=20.)
parser.add_argument("--view", choices=["cockpit", "external"], default="cockpit")
parser.add_argument("--fps-only", action="store_true", help="Disable GPU/CPU diagnostic counters; retain frame intervals")
args = parser.parse_args()
if not 0 < args.seconds <= 180 or not 0 < args.warmup <= 30:
    parser.error("Sample must be <=180 seconds and warm-up <=30 seconds")
OUT.mkdir(parents=True, exist_ok=True)
paused = []
try:
    pids = subprocess.run(["pgrep", "-x", "gripen-flight"], capture_output=True, text=True).stdout.split()
    for pid in map(int, pids):
        state = subprocess.run(["ps", "-o", "stat=", "-p", str(pid)], capture_output=True, text=True).stdout.strip()
        if state and not state.startswith("T"):
            os.kill(pid, signal.SIGSTOP)
            paused.append(pid)
    env = dict(os.environ)
    for key in ["NORD_BENCH_SCRIPT", "NORD_BENCH_FPS_ONLY", "NORD_BENCH_SOAK",
                "NORD_CARRIER_CAPTURE", "NORD_FLEET_CAPTURE", "NORD_CREW_CAPTURE", "NORD_RADAR_CAPTURE"]:
        env.pop(key, None)
    report = OUT / f"native-{args.label}.json"
    previous_report = report.stat().st_mtime_ns if report.exists() else None
    env.update(NORD_NAVAL_CAPTURE="missile-profile", NORD_NAVAL_LIVE="1",
               NORD_BENCH_SCENE="dry", NORD_BENCH_FULLSCREEN="1", NORD_PRESENT_MODE="vsync",
               NORD_BENCH_WARMUP=str(args.warmup), NORD_BENCH_SECONDS=str(args.seconds),
               NORD_BENCH_REPORT=str(report), NORD_SPIKE_LOG="1", NORD_PROFILE_CPU_ONLY="1",
               NORD_MISSILE_PROFILE_VIEW=args.view)
    if args.fps_only:
        env["NORD_BENCH_FPS_ONLY"] = "1"
    with (OUT / f"native-{args.label}.log").open("w") as log:
        process=subprocess.Popen([str(ROOT / "Nord Flight.app/Contents/MacOS/gripen-flight"),
                                  "--benchmark-render", "dry"], cwd=ROOT, env=env,
                                 stdout=log, stderr=subprocess.STDOUT)
        start=time.monotonic()
        memory=[]
        footprint_at=15.
        try:
            while process.poll() is None:
                rss=subprocess.run(["ps","-o","rss=","-p",str(process.pid)],capture_output=True,text=True).stdout.strip()
                if rss:
                    row={"wall_s":time.monotonic()-start,"rss_kib":int(rss)}
                    vm=subprocess.run(["vm_stat"],capture_output=True,text=True).stdout
                    row["vm_pages"]={key:int(value) for key,value in re.findall(r"^([^:\n]+):\s+(\d+)\.",vm,re.M)}
                    if row["wall_s"] >= footprint_at:
                        footprint_at+=30.
                        fp=subprocess.run(["footprint","-p",str(process.pid),"--swapped","--noCategories"],capture_output=True,text=True).stdout
                        row["footprint_report"]=fp
                    memory.append(row)
                if time.monotonic()-start > args.warmup+args.seconds+45:
                    raise TimeoutError("Bounded native profile exceeded its deadline")
                try: process.wait(timeout=5)
                except subprocess.TimeoutExpired: pass
            if process.returncode:
                raise subprocess.CalledProcessError(process.returncode,process.args)
        finally:
            if process.poll() is None:
                process.kill();process.wait()
            (OUT / f"native-{args.label}-process.json").write_text(json.dumps(memory,indent=2))
    if not report.exists() or report.stat().st_mtime_ns == previous_report:
        raise RuntimeError("Profile exited before writing a fresh timing report")
    data = json.loads(report.read_text())
    print(json.dumps({key: data[key] for key in
                      ["mean_fps", "p50_ms", "p95_ms", "p99_ms", "sample_seconds", "simulated_seconds"]}))
finally:
    for pid in paused:
        try:
            os.kill(pid, signal.SIGCONT)
        except ProcessLookupError:
            pass
    print("Original game sessions restored.")
