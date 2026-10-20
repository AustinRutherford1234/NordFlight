"""Paired CPU studies; run with no renderer, compiler, or other benchmark active.

python3 tests/compare_performance.py /path/to/baseline /path/to/candidate
The executables must support --study-flight and --study-air-combat.
"""
import hashlib
import json
from pathlib import Path
import statistics
import subprocess
import sys
import time


def main():
    root = Path(__file__).resolve().parents[1]
    if len(sys.argv) != 3:
        raise SystemExit("usage: compare_performance.py BASELINE CANDIDATE")
    binaries = dict(zip(("before", "after"), map(lambda p: Path(p).resolve(), sys.argv[1:])))
    cases = {"flight": "aerodynamic-study.json", "air-combat": "air-combat-study.json"}
    runs = []
    reference = {}
    equivalent = True
    for repeat in range(3):
        # Alternate order to reduce systematic warmup/thermal bias.
        order = ("before", "after") if repeat % 2 == 0 else ("after", "before")
        for case, output in cases.items():
            for version in order:
                tag = f"performance-{version}-{case}-{repeat + 1}"
                with (root / "tests" / f"{tag}.log").open("w") as log:
                    start = time.perf_counter()
                    subprocess.run([str(binaries[version]), f"--study-{case}"], cwd=root,
                                   stdout=log, stderr=subprocess.STDOUT, check=True)
                    seconds = time.perf_counter() - start
                data = json.loads((root / "tests" / output).read_text())
                (root / "tests" / f"{tag}.json").write_text(json.dumps(data, indent=2) + "\n")
                physical = ({"polars": data["polars"], "maneuvers": data["maneuvers"]}
                            if case == "flight" else data)
                if case not in reference:
                    reference[case] = physical
                equivalent &= physical == reference[case]
                runs.append({"case": case, "version": version, "repeat": repeat + 1,
                             "wall_seconds": seconds, "physics_equal": physical == reference[case]})
                print(f"{version} {case} #{repeat + 1}: {seconds:.3f}s", flush=True)
    summary = {}
    for case in cases:
        median = {version: statistics.median(r["wall_seconds"] for r in runs
                                            if r["case"] == case and r["version"] == version)
                  for version in binaries}
        summary[case] = {**median, "speedup": median["before"] / median["after"]}
    report = {"description": "Sequential paired release CPU studies, alternating version order",
              "binaries": {v: {"path": str(p), "sha256": hashlib.sha256(p.read_bytes()).hexdigest()}
                           for v, p in binaries.items()},
              "all_physics_equal": equivalent, "summary": summary, "runs": runs}
    (root / "tests" / "performance-comparison.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(summary, indent=2), flush=True)
    if not equivalent:
        raise SystemExit("Physics outputs changed; inspect the saved reports before accepting speedups.")


if __name__ == "__main__":
    main()
