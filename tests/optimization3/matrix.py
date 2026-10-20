"""Run isolated, alternating baseline/candidate render pairs, one scene at a time."""
import argparse
import hashlib
import json
from pathlib import Path
import signal
import statistics
import subprocess
import sys
from run import OUT, SCENES


def summarize(folder):
    result = {}
    for scene in SCENES:
        versions = {}
        for version in ("a", "b"):
            reports = []
            for repeat in (1, 2):
                p = folder / f"{scene}-{version}{repeat}" / f"render-{scene}-1.json"
                if p.exists():
                    reports.append(json.loads(p.read_text()))
            if reports:
                versions[version] = {
                    key: statistics.median(r[key] for r in reports)
                    for key in ("mean_fps", "p50_ms", "p95_ms", "p99_ms", "simulation_rate", "over_25_ms", "over_50_ms")
                }
                versions[version]["runs"] = len(reports)
                versions[version]["stutter_25_percent"] = statistics.median(100 * r["over_25_ms"] / r["frames"] for r in reports)
                versions[version]["stutter_50_percent"] = statistics.median(100 * r["over_50_ms"] / r["frames"] for r in reports)
                versions[version]["upload_mb_per_second"] = statistics.median(r["measurements"]["upload_bytes"] / r["sample_seconds"] / 1e6 for r in reports)
                versions[version]["cpu_ms_per_frame"] = {
                    span: statistics.median(r["measurements"]["cpu_spans"][span]["total_ms"] / r["frames"] for r in reports)
                    for span in reports[0]["measurements"]["cpu_spans"]
                }
                versions[version]["asset_modifications_per_frame"] = {
                    kind: statistics.median(r["measurements"][kind] / r["frames"] for r in reports)
                    for kind in ("mesh_modifications", "material_modifications", "image_modifications")
                }
        if versions:
            result[scene] = versions
            if all(v in versions for v in ("a", "b")):
                result[scene]["fps_ratio"] = versions["b"]["mean_fps"] / versions["a"]["mean_fps"]
                result[scene]["p95_ratio"] = versions["b"]["p95_ms"] / versions["a"]["p95_ms"]
    (folder / "summary.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("baseline", type=Path)
    parser.add_argument("candidate", type=Path)
    parser.add_argument("label")
    parser.add_argument("--scenes", default=",".join(SCENES))
    parser.add_argument("--paused", action="store_true")
    parser.add_argument("--quality", action="store_true")
    args = parser.parse_args()
    folder = OUT / args.label
    folder.mkdir(parents=True, exist_ok=True)
    child = None
    try:
        for scene in args.scenes.split(","):
            for version, repeat in (("a", 1), ("b", 1), ("b", 2), ("a", 2)):
                binary = (args.baseline if version == "a" else args.candidate).resolve()
                label = f"{args.label}/{scene}-{version}{repeat}"
                completed = OUT / label / f"render-{scene}-1.json"
                if completed.exists():
                    # Resume only results that passed the runner's exit/error checks.
                    process = json.loads((completed.parent / f"render-{scene}-1-process.json").read_text())
                    manifest = json.loads((completed.parent / "executable.json").read_text())
                    if manifest != {"binary": str(binary), "sha256": hashlib.sha256(binary.read_bytes()).hexdigest()}:
                        raise RuntimeError(f"Resume executable changed: {label}")
                    report = json.loads(completed.read_text())
                    if report["scene"] != SCENES[scene][0] or report["paused"] != (args.paused or not SCENES[scene][1]):
                        raise RuntimeError(f"Resume scene/pause configuration changed: {label}")
                    if report["quality"] != args.quality:
                        raise RuntimeError(f"Resume graphics quality changed: {label}")
                    if process["exit"] != 0:
                        raise RuntimeError(f"Incomplete previous run: {label}")
                    continue
                print(f"Starting {scene}: {'baseline' if version == 'a' else 'candidate'} {repeat}/2", flush=True)
                command = [sys.executable, str(OUT / "run.py"), str(binary.resolve()), label, "render", "--scenes", scene]
                if args.paused:
                    command.append("--paused")
                if args.quality:
                    command.append("--quality")
                child = subprocess.Popen(command)
                code = child.wait()
                child = None
                if code:
                    raise RuntimeError(f"Benchmark failed: {label} (exit {code})")
                summarize(folder)
    finally:
        if child is not None and child.poll() is None:
            child.terminate()
            child.wait(timeout=20)
    print(json.dumps(summarize(folder), indent=2), flush=True)


if __name__ == "__main__":
    signal.signal(signal.SIGTERM, lambda *_: (_ for _ in ()).throw(KeyboardInterrupt()))
    main()
