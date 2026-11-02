"""Short, explicitly approved native salvo profile; always restore user sessions."""
from pathlib import Path
import argparse
import json
import os
import signal
import subprocess

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "tests/missile-spikes"
parser = argparse.ArgumentParser()
parser.add_argument("--label", choices=["before", "after"], default="after")
parser.add_argument("--seconds", type=float, default=15.)
parser.add_argument("--fps-only", action="store_true", help="Disable GPU/CPU diagnostic counters; retain frame intervals")
args = parser.parse_args()
if not 0 < args.seconds <= 25:
    parser.error("Sample must be between zero and 25 seconds; warm-up adds five seconds")
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
    env.update(NORD_NAVAL_CAPTURE="missile-profile", NORD_NAVAL_LIVE="1",
               NORD_BENCH_SCENE="dry", NORD_BENCH_FULLSCREEN="1", NORD_PRESENT_MODE="vsync",
               NORD_BENCH_WARMUP="5", NORD_BENCH_SECONDS=str(args.seconds),
               NORD_BENCH_REPORT=str(report), NORD_SPIKE_LOG="1")
    if args.fps_only:
        env["NORD_BENCH_FPS_ONLY"] = "1"
    with (OUT / f"native-{args.label}.log").open("w") as log:
        subprocess.run([str(ROOT / "Nord Flight.app/Contents/MacOS/gripen-flight"),
                        "--benchmark-render", "dry"], cwd=ROOT, env=env,
                       stdout=log, stderr=subprocess.STDOUT, check=True, timeout=100)
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
