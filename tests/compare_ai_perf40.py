"""Full-rate AI flight/combat equivalence; run separately from render/CPU timing."""
from pathlib import Path
import hashlib
import json
import shutil
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]


def without_wall(value):
    if isinstance(value, dict):
        return {k: without_wall(v) for k, v in value.items() if k != "wall_seconds"}
    if isinstance(value, list):
        return [without_wall(v) for v in value]
    return value


reports = {}
for version, binary in zip(("before", "after"), sys.argv[1:3]):
    start = time.perf_counter()
    with (ROOT / "tests" / f"perf40-ai-{version}.log").open("w") as log:
        subprocess.run([str(Path(binary).resolve()), "--study-ai"], cwd=ROOT,
                       stdout=log, stderr=subprocess.STDOUT, check=True, timeout=900)
    result = {"wall_seconds": time.perf_counter() - start, "data": {}, "csv": {}}
    for name in ("ai-flight-study.json", "ai-combat-study.json", "ai-flight-study.csv", "ai-combat-flight.csv"):
        source = ROOT / "tests" / name
        shutil.copy2(source, ROOT / "tests" / f"perf40-{version}-{name}")
        if source.suffix == ".json":
            result["data"][name] = without_wall(json.loads(source.read_text()))
        else:
            result["csv"][name] = hashlib.sha256(source.read_bytes()).hexdigest()
    reports[version] = result
    print(version, round(result["wall_seconds"], 3), "seconds", flush=True)
equal = (reports["before"]["data"] == reports["after"]["data"]
         and reports["before"]["csv"] == reports["after"]["csv"])
reports["physical_equal"] = equal
(ROOT / "tests" / "perf40-ai-comparison.json").write_text(json.dumps(reports, indent=2) + "\n")
print("all AI flight/combat states and CSVs equal:", equal, flush=True)
if not equal:
    raise SystemExit("AI behavior changed; inspect saved reports.")
