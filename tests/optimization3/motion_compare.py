"""Compare native motion only at exactly matching saved simulation states."""
import argparse
import json
from pathlib import Path
import numpy as np
from PIL import Image

p = argparse.ArgumentParser()
p.add_argument("reference", type=Path)
p.add_argument("candidate", type=Path)
p.add_argument("output", type=Path)
args = p.parse_args()
rows = []
for reference in sorted(args.reference.glob("frame-*.json")):
    candidate = args.candidate / reference.name
    if not candidate.exists():
        continue
    a = json.loads(reference.read_text())
    b = json.loads(candidate.read_text())
    row = {"frame": reference.stem, "state_equal": a == b, "simulation_s": a["simulation_s"], "altitude_m": a["altitude_m"]}
    if a == b and reference.with_suffix(".png").exists() and candidate.with_suffix(".png").exists():
        x = np.asarray(Image.open(reference.with_suffix(".png")).convert("RGB"), dtype=np.int16)
        y = np.asarray(Image.open(candidate.with_suffix(".png")).convert("RGB"), dtype=np.int16)
        assert x.shape == y.shape
        delta = np.abs(x - y)
        delta[:75, 1200:] = 0
        row.update(mean_abs_rgb=float(delta.mean()), p99_abs_rgb=float(np.percentile(delta,99)), max_abs_channel=int(delta.max()))
    rows.append(row)
result = {"reference_frames": len(list(args.reference.glob("frame-*.json"))),
          "candidate_frames": len(list(args.candidate.glob("frame-*.json"))),
          "common_frames": len(rows), "exact_physical_states": sum(row["state_equal"] for row in rows),
          "mask": "x >= 1200 and y < 75, dynamic telemetry header only; buttons remain compared", "frames": rows,
          "note": "Wall-clock presentation effects can differ even at identical simulation states. Readback timings are not FPS measurements. Missing or unequal-state images are not scored as equal."}
args.output.write_text(json.dumps(result,indent=2)+"\n")
print(json.dumps({key:result[key] for key in ('reference_frames','candidate_frames','common_frames','exact_physical_states')},indent=2))
