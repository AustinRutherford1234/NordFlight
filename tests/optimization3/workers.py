"""Sequential worker-count experiments with complete physical-output checks."""
import argparse
import hashlib
import json
from pathlib import Path
import statistics
from run import OUT, ROOT, isolated, run, artifact_stamp, copy_fresh
from verify import physical
import time

parser = argparse.ArgumentParser()
parser.add_argument("binary", type=Path)
parser.add_argument("label")
args = parser.parse_args()
binary = args.binary.resolve()
folder = OUT / args.label
folder.mkdir(parents=True, exist_ok=True)
(folder / "executable.json").write_text(json.dumps({"binary": str(binary), "sha256": hashlib.sha256(binary.read_bytes()).hexdigest()}, indent=2))
reference = None
results = {n: [] for n in (4, 6, 8)}
with isolated():
    for index, count in enumerate((4, 6, 8, 8, 6, 4, 6, 4, 8)):
        tag = f"workers-{count}-{index}"
        source = ROOT / "tests/bvr-study.json"
        before = artifact_stamp(source)
        started = time.time_ns()
        run(binary, ["--study-bvr"], folder, tag, {"NORD_COMPUTE_THREADS": str(count)}, timeout=300)
        temporary = folder / tag
        temporary.mkdir()
        copy_fresh(source, temporary, before, started)
        report = json.loads((temporary / source.name).read_text())
        state = physical(source.name, report)
        if reference is None:
            reference = state
        if state != reference:
            raise RuntimeError(f"Worker count changed physical outputs: {tag}")
        process = json.loads((folder / (tag + "-process.json")).read_text())
        results[count].append(process["seconds"])
        summary = {str(n): {"seconds": values, "median_seconds": statistics.median(values)} for n, values in results.items() if values}
        (folder / "summary.json").write_text(json.dumps({"all_physical_outputs_equal": True, "workers": summary}, indent=2) + "\n")
