"""Short release-build rendering guardrail, with recorded host power state."""
from pathlib import Path
import hashlib
import importlib.util
import json

ROOT = Path(__file__).resolve().parents[2]
HERE = Path(__file__).parent
spec = importlib.util.spec_from_file_location("native", ROOT / "tests/optimization3/run.py")
native = importlib.util.module_from_spec(spec)
spec.loader.exec_module(native)


def main():
    out = HERE / "performance"
    out.mkdir(exist_ok=False)
    binaries = {
        "baseline": HERE / "baseline/gripen-flight",
        "candidate": ROOT / "Nord Flight.app/Contents/MacOS/gripen-flight",
    }
    (out / "executables.json").write_text(json.dumps({
        k: {"path": str(v), "sha256": hashlib.sha256(v.read_bytes()).hexdigest()}
        for k, v in binaries.items()
    }, indent=2))
    records = []
    with native.isolated():
        for version, binary in binaries.items():
            report = out / f"{version}.json"
            env = {
                "NORD_AIRCRAFT": "f22", "NORD_FLEET_CAPTURE": "front",
                "NORD_BENCH_SCENE": "air-hud", "NORD_BENCH_LIVE": "0",
                "NORD_BENCH_SCRIPT": "", "NORD_BENCH_WARMUP": "5",
                "NORD_BENCH_SECONDS": "12", "NORD_BENCH_REPORT": str(report),
            }
            native.run(binary, ["--benchmark-render", version], out, version, env, timeout=90)
            records.append({"version": version, **json.loads(report.read_text())})
    (out / "summary.json").write_text(json.dumps(records, indent=2))
    for r in records:
        print(f"{r['version']}: {r['mean_fps']:.2f} FPS, p95 {r['p95_ms']:.2f} ms")


if __name__ == "__main__":
    main()
