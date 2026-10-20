"""Alternating flight/BVR throughput controls, preserving full diagnostic files."""
import argparse
import json
from pathlib import Path
import statistics
import subprocess
import sys
import signal
import atexit
from run import OUT
from verify import physical

p = argparse.ArgumentParser()
p.add_argument("baseline", type=Path)
p.add_argument("candidate", type=Path)
p.add_argument("label")
args = p.parse_args()
folder = OUT / args.label
rows = {"a": [], "b": []}
child = None
def cleanup():
    if child is not None and child.poll() is None:
        child.terminate()
        child.wait(timeout=30)
atexit.register(cleanup)
signal.signal(signal.SIGTERM, lambda *_: (_ for _ in ()).throw(KeyboardInterrupt()))
for version, repeat in (("a", 1), ("b", 1), ("b", 2), ("a", 2)):
    name = f"{args.label}/{version}{repeat}"
    binary = args.baseline if version == "a" else args.candidate
    child = subprocess.Popen([sys.executable, str(OUT / "run.py"), str(binary), name, "cpu", "--studies", "flight,bvr"])
    code = child.wait()
    child = None
    if code: raise RuntimeError(f"CPU control failed: {name}")
    flight = json.loads((OUT / name / "aerodynamic-study.json").read_text())
    bvr = json.loads((OUT / name / "bvr-study.json").read_text())
    rows[version].append({"flight": flight, "bvr": bvr, "folder": name})
reference = rows["a"][0]
for version in rows:
    for run in rows[version]:
        assert physical("aerodynamic-study.json", run["flight"]) == physical("aerodynamic-study.json", reference["flight"])
        assert physical("bvr-study.json", run["bvr"]) == physical("bvr-study.json", reference["bvr"])
summary = {"physical_outputs_equal": True, "versions": {}}
for version in rows:
    summary["versions"][version] = {
        "jet_seconds_median": statistics.median(run["flight"]["benchmark"]["jet_seconds"] for run in rows[version]),
        "flight_process_seconds_median": statistics.median(json.loads((OUT / run["folder"] / "flight-process.json").read_text())["seconds"] for run in rows[version]),
        "bvr_process_seconds_median": statistics.median(json.loads((OUT / run["folder"] / "bvr-process.json").read_text())["seconds"] for run in rows[version]),
    }
(folder / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
print(json.dumps(summary, indent=2))
