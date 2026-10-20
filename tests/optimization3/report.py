"""Summarize accepted measurements without converting missing metrics to zero."""
import argparse
import json
from pathlib import Path
import statistics
from run import SCENES

parser = argparse.ArgumentParser()
parser.add_argument("folder", type=Path)
args = parser.parse_args()
root = args.folder
matrix = json.loads((root / "release-matrix/summary.json").read_text())
for name in SCENES:
    assert name in matrix and all(matrix[name][v]["runs"] == 2 for v in ("a", "b")), name
physical = json.loads((root / "release-physics-equivalence.json").read_text())
assert all(result["equal"] for result in physical.values()), "Authoritative state changed"
soak = json.loads((root / "release-soak/soak.json").read_text())
process = json.loads((root / "release-soak/soak-process.json").read_text())
assert process["exit"] == 0 and soak["sample_seconds"] >= 1800
samples = [event for event in soak["exercise_events"] if event["event"] == "sample"]
phases = {}
for name in dict.fromkeys(event["script"] for event in samples):
    rows = [event for event in samples if event["script"] == name]
    phases[name] = {"samples": len(rows), "wall_range_s": [rows[0]["wall_s"], rows[-1]["wall_s"]],
                    "resource_maxima": {key: max(row[key] for row in rows) for key in ("entities", "meshes", "images", "missiles", "particles")},
                    "last_resources": {key: rows[-1][key] for key in ("entities", "meshes", "images", "missiles", "particles")}}
assert len(phases) == 12 and max(event["particles"] for event in samples) <= 256
memory = process["memory"]
regressions = {name: {key: d["b"][key] / d["a"][key] for key in ("p50_ms", "p95_ms", "p99_ms")
                      if d["b"][key] > d["a"][key] * 1.05} for name, d in matrix.items()}
regressions = {name: values for name, values in regressions.items() if values}
summary = {"correctness_reports": len(physical), "soak_sample_seconds": soak["sample_seconds"],
           "soak_peak_rss_mib": process["peak_rss_kib"] / 1024,
           "soak_first_rss_mib": memory[0]["rss_kib"] / 1024,
           "soak_last_rss_mib": memory[-1]["rss_kib"] / 1024,
           "soak_phases": phases, "percentile_regressions_over_5_percent": regressions,
           "note": "Cross-scenario RSS includes terrain/Earth/effect resources; overall RSS change alone does not establish a leak. Examine per-phase resource maxima and recovery. Live evolving combat can vary; use paused controls and repeat inconclusive results."}
(root / "acceptance-summary.json").write_text(json.dumps(summary, indent=2) + "\n")
lines = ["| Workload | FPS before → after | p95 ms before → after | p99 ms before → after | Sim s / wall s before → after |",
         "| --- | ---: | ---: | ---: | ---: |"]
for name, d in matrix.items():
    a,b = d["a"],d["b"]
    lines.append(f"| {name} | {a['mean_fps']:.2f} → {b['mean_fps']:.2f} | {a['p95_ms']:.2f} → {b['p95_ms']:.2f} | {a['p99_ms']:.2f} → {b['p99_ms']:.2f} | {a['simulation_rate']:.3f} → {b['simulation_rate']:.3f} |")
(root / "before-after.md").write_text("\n".join(lines) + "\n")
print(json.dumps(summary, indent=2))
