"""Focused, sequential near-terrain renderer comparisons (no concurrent builds).

The reference switches restore the original rendering paths in the same
executable. No physics or quality settings change.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tests/optimization3"))
from run import isolated, record_child

parser = argparse.ArgumentParser()
parser.add_argument("binary", type=Path)
parser.add_argument("output", type=Path)
parser.add_argument("--warmup", default=10, type=int)
parser.add_argument("--seconds", default=20, type=int)
parser.add_argument("--windowed", action="store_true")
parser.add_argument("--cockpit", action="store_true")
parser.add_argument("--profile", action="store_true", help="Enable intrusive GPU/CPU pass diagnostics; default measures shipping-style FPS only")
parser.add_argument("--pairs",type=int,choices=[1,2],default=2)
parser.add_argument("--reference-forest", action="store_true",
                    help="Keep original large forest batches in both cases")
args = parser.parse_args()
binary = args.binary.resolve()
args.output.mkdir(parents=True, exist_ok=True)
summary = {}
with isolated():
    cases=[("reference1", True), ("candidate1", False), ("candidate2", False), ("reference2", True)]
    for label, reference in cases[:2] if args.pairs==1 else cases:
        report = (args.output / (label + ".json")).resolve()
        if report.exists():
            raise SystemExit("Use a fresh output directory")
        env = os.environ.copy()
        for key in ["NORD_ENV_SERIAL_IRRADIANCE", "NORD_TERRAIN_LARGE_BATCHES",
                    "NORD_BENCH_FULLSCREEN", "NORD_TERRAIN_COCKPIT", "NORD_EARTH_DRAW_ZERO_ALPHA", "NORD_REFERENCE_LIGHTS", "NORD_TERRAIN_BATCH_SIZE"]:
            env.pop(key, None)
        env.update(NORD_BENCH_SCENE="cockpit" if args.cockpit else "terrain-forest", NORD_BENCH_SCRIPT="terrain-stream",
                   NORD_BENCH_LIVE="0", NORD_TERRAIN_AGL="50", NORD_TERRAIN_SPEED="100",
                   NORD_BENCH_WARMUP=str(args.warmup), NORD_BENCH_SECONDS=str(args.seconds),
                   NORD_BENCH_REPORT=str(report),
                   NORD_FLIGHT_AUDIO_SETTINGS=str(ROOT / "tests/optimization3/audio-settings.json"))
        if args.profile:
            env.pop("NORD_BENCH_FPS_ONLY",None)
        else:
            env["NORD_BENCH_FPS_ONLY"]="1"
        if not args.windowed:
            env["NORD_BENCH_FULLSCREEN"] = "1"
        if args.cockpit:
            env["NORD_TERRAIN_COCKPIT"] = "1"
        if reference:
            env["NORD_ENV_SERIAL_IRRADIANCE"] = "1"
            env["NORD_EARTH_DRAW_ZERO_ALPHA"] = "1"
            env["NORD_REFERENCE_LIGHTS"] = "1"
        if reference or args.reference_forest:
            env["NORD_TERRAIN_LARGE_BATCHES"] = "1"
        power = subprocess.check_output(["pmset","-g","batt"],text=True).strip()
        processes=subprocess.check_output(["ps","-axo","pcpu=,comm="],text=True).splitlines()
        busy=sorted([(float(line.split(None,1)[0]),line.split(None,1)[1]) for line in processes
                     if len(line.split(None,1))==2],reverse=True)[:5]
        with (args.output / (label + ".log")).open("w") as log:
            child = subprocess.Popen([str(binary), "--benchmark-render", "terrain"],
                                     cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT)
            record_child(child.pid)
            try:
                child.wait(timeout=args.warmup + args.seconds + 90)
            finally:
                if child.poll() is None:
                    child.terminate()
                    child.wait(timeout=10)
                record_child(None)
            if child.returncode:
                raise SystemExit(f"{label} failed: {child.returncode}")
        data = json.loads(report.read_text())
        assert all(e["forest"] for e in data["exercise_events"] if e["wall_s"] >= args.warmup)
        summary[label] = {k: data[k] for k in ["resolution", "mean_fps", "p50_ms", "p95_ms",
                                              "p99_ms", "over_25_ms", "over_50_ms"]}
        summary[label]["forest_batches"] = data["measurements"]["forest_vertices_in_visible_batches_mean"] if args.profile else None
        summary[label]["power_before"] = power
        summary[label]["busiest_processes_before"] = busy
        print(label, summary[label], flush=True)
        (args.output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
(args.output / "executable.json").write_text(json.dumps({
    "path": str(binary), "sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
    "note": "Same-binary ABBA. Paused authoritative physics; moving camera/terrain path. Potential batch vertices are not hardware draw or vertex counters."
}, indent=2) + "\n")
