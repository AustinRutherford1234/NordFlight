"""Bound generated optimization artifacts after acceptance; sources/logs remain.

Defaults to a dry run. Only this harness's registered PNGs, profiling traces and
explicitly named intermediate executable/source snapshots are eligible.
"""
import argparse
import json
from pathlib import Path
import shutil
from run import OUT

def plan(root, accepted):
    root, accepted = root.resolve(), accepted.resolve()
    if accepted.parent != root or not accepted.name.startswith("accepted"):
        raise ValueError("Accepted run must be an immediate accepted* child of the artifact root")
    preserved = (root / "baseline", accepted, root / "metal-short-baseline", root / "allocations-debuggable-pre")
    actions = set()
    for image in root.rglob("*.png"):
        if not image.name.startswith(("visual-", "frame-")):
            continue
        if any(image.is_relative_to(folder) for folder in preserved):
            continue
        registered = any((parent / "executable.json").is_file() or (parent / "capture.json").is_file()
                         for parent in image.parents if parent.is_relative_to(root))
        if registered:
            actions.add(image)
    for trace in root.rglob("profile.trace"):
        if any(trace.is_relative_to(folder) for folder in preserved):
            continue
        # These exact trace folders were produced by this optimization pass.
        if (trace.parent / "capture.json").is_file() or trace.parent.name in ("metal-baseline", "metal-accepted", "metal-pre-bindgroups"):
            actions.add(trace)
    explicit = {
        "baseline/gripen-flight-instrumented", "baseline/gripen-flight-aligned-dev",
        "candidate/gripen-flight-dev", "candidate/gripen-flight-profile",
        "candidate/gripen-flight-pre-bindgroups-release", "candidate/gripen-flight-pre-content-release",
        "candidate/source-pre-content.tar.gz", "candidate/source-before-copy-fix.tar.gz",
    }
    actions.update(root / relative for relative in explicit if (root / relative).exists())
    return [path for path in sorted(actions) if not any(parent in actions for parent in path.parents)]

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("accepted")
    parser.add_argument("--apply", action="store_true")
    args = parser.parse_args()
    accepted = (OUT / args.accepted).resolve()
    # Acceptance and the same-executable resource control must exist before
    # obsolete intermediate evidence can be retired.
    evidence = json.loads((accepted / "acceptance-summary.json").read_text())
    assert evidence["soak_sample_seconds"] >= 1800
    assert (accepted / "metal-bindings-disabled/resource-summary.json").is_file()
    actions = plan(OUT, accepted)
    rows = []
    for path in actions:
        size = sum(p.stat().st_size for p in path.rglob("*") if p.is_file()) if path.is_dir() else path.stat().st_size
        rows.append({"path":str(path.relative_to(OUT)), "bytes":size})
        if args.apply:
            if path.is_dir(): shutil.rmtree(path)
            else: path.unlink()
    result = {"applied":args.apply, "retired_bytes":sum(row["bytes"] for row in rows), "artifacts":rows,
              "preserved":"Initial baseline, measured baseline, accepted sources/executable/captures/profiles, baseline GPU/Allocations evidence, JSON/log/documentation and unregistered files"}
    if args.apply:
        (accepted / "artifact-retention.json").write_text(json.dumps(result,indent=2)+"\n")
    print(json.dumps(result,indent=2))
