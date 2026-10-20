"""Compare matched native captures; mask only the changing top-right FPS text."""
import argparse
import json
from pathlib import Path
import numpy as np
from PIL import Image

parser = argparse.ArgumentParser()
parser.add_argument("baseline", type=Path)
parser.add_argument("candidate", type=Path)
parser.add_argument("output", type=Path)
args = parser.parse_args()
results = {}
for candidate in sorted(args.candidate.glob("visual-*.png")):
    baseline = args.baseline / candidate.name
    if not baseline.exists():
        continue
    a = np.asarray(Image.open(baseline).convert("RGB"), dtype=np.int16)
    b = np.asarray(Image.open(candidate).convert("RGB"), dtype=np.int16)
    if a.shape != b.shape:
        raise RuntimeError(f"Capture dimensions changed: {candidate.name}")
    delta = np.abs(a - b)
    # FPS and run metadata occupy this fixed telemetry corner at 1440 x 900.
    delta[:75, 1200:] = 0
    results[candidate.stem] = {
        "mean_abs_rgb": float(delta.mean()),
        "p99_abs_rgb": float(np.percentile(delta, 99)),
        "max_abs_channel": int(delta.max()),
        "pixels_different": int(np.any(delta, axis=2).sum()),
        "width": int(a.shape[1]), "height": int(a.shape[0]),
    }
args.output.write_text(json.dumps({"mask": "x >= 1200 and y < 75; dynamic FPS/telemetry header only; buttons remain compared", "captures": results}, indent=2) + "\n")
print(json.dumps(results, indent=2))
