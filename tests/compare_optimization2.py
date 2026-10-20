"""Matched source-optimization checks. Run CPU and renderer passes separately."""
import hashlib
import json
from pathlib import Path
import statistics
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]


def physical(case, data):
    if case == "flight":
        return {"polars": data["polars"], "maneuvers": data["maneuvers"],
                "jet_steps": data["benchmark"]["jet_steps"],
                "orbit_steps": data["benchmark"]["orbit_steps"]}
    if case == "workloads":
        return data["checks"]
    return data


def main():
    if len(sys.argv) not in (3, 4):
        raise SystemExit("usage: compare_optimization2.py BASELINE CANDIDATE [render]")
    binaries = dict(zip(("before", "after"), (Path(p).resolve() for p in sys.argv[1:3])))
    renderer = len(sys.argv) == 4 and sys.argv[3] == "render"
    runs = []
    references = {}
    cases = {"flight": "aerodynamic-study.json", "air-combat": "air-combat-study.json",
             "bvr": "bvr-study.json", "workloads": "optimization2-workloads.json"}
    repeats = range(4) if renderer else range(3)
    for repeat in repeats:
        order = [("before", "after", "after", "before")[repeat]] if renderer else (
            ("before", "after") if repeat % 2 == 0 else ("after", "before"))
        for case, report in ({"render": ""} if renderer else cases).items():
            for version in order:
                tag = f"optimization2-{version}-{case}-{repeat + 1}"
                args = ["--benchmark-render", tag] if renderer else [f"--study-{case}"]
                with (ROOT / "tests" / f"{tag}.log").open("w") as log:
                    start = time.perf_counter()
                    subprocess.run([str(binaries[version]), *args], cwd=ROOT,
                                   stdout=log, stderr=subprocess.STDOUT, check=True)
                    seconds = time.perf_counter() - start
                source = next((ROOT / "tests").glob(f"render-*-{tag}.json")) if renderer else ROOT / "tests" / report
                data = json.loads(source.read_text())
                (ROOT / "tests" / f"{tag}.json").write_text(json.dumps(data, indent=2) + "\n")
                check = physical(case, data) if not renderer else None
                references.setdefault(case, check)
                equal = check == references[case]
                runs.append({"case": case, "version": version, "repeat": repeat + 1,
                             "wall_seconds": seconds, "physical_equal": equal,
                             **({"mean_fps": data["mean_fps"], "p95_ms": data["p95_ms"]}
                                if renderer else {})})
                print(f"{version} {case} #{repeat + 1}: {seconds:.3f}s / equal={equal}"
                      + (f" / {data['mean_fps']:.2f} FPS" if renderer else ""), flush=True)
    summary = {}
    for case in ("render",) if renderer else cases:
        values = {v: [r["mean_fps" if renderer else "wall_seconds"] for r in runs
                      if r["case"] == case and r["version"] == v] for v in binaries}
        medians = {v: statistics.median(samples) for v, samples in values.items()}
        summary[case] = {"before": medians["before"], "after": medians["after"],
                         "ratio": medians["after"] / medians["before"] if renderer else medians["before"] / medians["after"],
                         "samples": values}
    result = {"profile": "matched development builds", "render": renderer,
              "binaries": {v: {"path": str(p), "sha256": hashlib.sha256(p.read_bytes()).hexdigest()}
                           for v, p in binaries.items()},
              "all_physical_equal": all(r["physical_equal"] for r in runs),
              "summary": summary, "runs": runs}
    (ROOT / "tests" / f"optimization2-{'render' if renderer else 'cpu'}-comparison.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(summary, indent=2), flush=True)
    if not result["all_physical_equal"]:
        raise SystemExit("Behavior changed: inspect reports before accepting the optimizations.")


if __name__ == "__main__":
    main()
