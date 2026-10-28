"""Compare production FBW traces using identical starts and pilot inputs."""
import json
import os
from pathlib import Path

os.environ.setdefault("MPLCONFIGDIR", "/tmp/gripen-fbw-mpl")
os.environ.setdefault("XDG_CACHE_HOME", "/tmp/gripen-fbw-font-cache")
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

root = Path(__file__).resolve().parent
before = json.loads((root / "before-bvr.json").read_text())
after = json.loads((root / "after.json").read_text())
fig, axes = plt.subplots(3, 2, figsize=(11, 8), sharex=True)
cases = [(290., [0.5, 0., 0.3], "290 m/s: combined pull / roll"),
         (362.5, [1., 0., 0.], "362.5 m/s: full aft stick")]
summary = []
for col, (speed, command, title) in enumerate(cases):
    for data, label, color in [(before, "Previous FBW", "#858d96"),
                               (after, "Updated FBW", "#2477b5")]:
        row = next(r for r in data if r.get("case") == "bvr" and r["speed"] == speed
                   and r["command"] == command and r["engines"] == [True, True])
        samples = row["samples"]
        for ax, key, ylabel in zip(axes[:, col], ["alpha", "beta", "g"],
                                   ["Incidence (deg)", "Sideslip (deg)", "Normal load (G)"]):
            ax.plot([s["t"] for s in samples], [s[key] for s in samples],
                    label=label, color=color, linewidth=1.8)
            ax.set_ylabel(ylabel)
        summary.append({"version": label, "speed": speed, "command": command,
                        **{k: row[k] for k in ["peak_alpha", "peak_beta", "peak_rate", "final_alpha", "final_rate", "crashed"]}})
    axes[0, col].set_title(title)
    axes[2, col].set_xlabel("Simulation time (s)")
for ax in axes.flat:
    ax.axvline(4, color="#6d7882", linestyle=":", linewidth=1)
    ax.grid(alpha=.2)
    ax.spines[["top", "right"]].set_visible(False)
axes[0, 0].legend(frameon=False)
fig.suptitle("F-22 FBW at the BVR start altitude (11 km)", fontsize=15)
fig.text(.10, .015, "Identical initial states and inputs; stick released at 4 s. Both engines running, dt = 1/120 s.\n"
         "Simulator behavior with estimated control laws; these are not real F-22 flight-test measurements.",
         fontsize=9, color="#46525b")
fig.tight_layout(rect=(0, .07, 1, .96))
fig.savefig(root / "bvr-response.png", dpi=180)
(root / "comparison.json").write_text(json.dumps(summary, indent=2) + "\n")
