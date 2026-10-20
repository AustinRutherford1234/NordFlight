"""Sequential ABBA comparison of the existing 45-second native runway benchmark."""
import hashlib
import json
from pathlib import Path
import statistics
import subprocess
import sys


def main():
    if len(sys.argv) != 3:
        raise SystemExit("usage: compare_render.py BASELINE CANDIDATE")
    root = Path(__file__).resolve().parents[1]
    binaries = dict(zip(("before", "after"), (Path(p).resolve() for p in sys.argv[1:])))
    counts = dict.fromkeys(binaries, 0)
    runs = []
    for version in ("before", "after", "after", "before"):
        counts[version] += 1
        mode = f"performance-{version}-repeat-{counts[version]}"
        with (root / "tests" / f"render-{mode}.log").open("w") as log:
            subprocess.run([str(binaries[version]), "--benchmark-render", mode], cwd=root,
                           stdout=log, stderr=subprocess.STDOUT, check=True)
        report = json.loads((root / "tests" / f"render-release-{mode}.json").read_text())
        runs.append({"version": version, **report})
        print(f"{version} #{counts[version]}: {report['mean_fps']:.2f} FPS, "
              f"p95 {report['p95_ms']:.2f} ms", flush=True)
    means = {v: statistics.mean(r["mean_fps"] for r in runs if r["version"] == v)
             for v in binaries}
    report = {"description": "ABBA order, unchanged paused runway, VSync, 5s warmup + 40s each",
              "binaries": {v: {"path": str(p), "sha256": hashlib.sha256(p.read_bytes()).hexdigest()}
                           for v, p in binaries.items()},
              "mean_fps_by_version": means, "fps_ratio": means["after"] / means["before"],
              "runs": runs}
    (root / "tests" / "performance-render-comparison.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(means, indent=2), flush=True)


if __name__ == "__main__":
    main()
