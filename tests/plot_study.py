"""Render the numerical study as scientific figures; no simulator images fabricated."""
import json
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

ROOT = Path(__file__).resolve().parent
study = json.loads((ROOT / "aerodynamic-study.json").read_text())
plt.rcParams.update({"font.size": 10, "axes.spines.top": False, "axes.spines.right": False})
fig, axes = plt.subplots(2, 2, figsize=(12, 8), constrained_layout=True)
for mach in [0.15, 0.4, 0.8, 1.0, 1.2, 1.8, 2.2]:
    rows = [p for p in study["polars"] if p["mach"] == mach and p["flaps"] == 0.0 and -30 <= p["alpha_deg"] <= 45]
    axes[0, 0].plot([r["alpha_deg"] for r in rows], [r["cl"] for r in rows], marker=".", label=f"M {mach:g}")
    axes[0, 1].plot([r["alpha_deg"] for r in rows], [r["cd"] for r in rows], marker=".", label=f"M {mach:g}")
for flaps in [0.0, 0.3, 0.65]:
    rows = [p for p in study["polars"] if p["mach"] == 0.4 and p["flaps"] == flaps and -10 <= p["alpha_deg"] <= 30]
    axes[1, 0].plot([r["alpha_deg"] for r in rows], [r["cl"] for r in rows], marker="o", label=f"Flaps {flaps:.0%}")
for fbw, offset, color in [(False, -0.18, "#da714d"), (True, 0.18, "#317da3")]:
    cases = [m for m in study["maneuvers"] if m["fbw"] == fbw]
    axes[1, 1].bar([i + offset for i in range(len(cases))], [m["max_g"] for m in cases], width=0.34, label="FBW" if fbw else "Raw surfaces", color=color)
axes[0, 0].set(title="Steady lift vs incidence — clean configuration", xlabel="Angle of attack (deg)", ylabel="Total CL")
axes[0, 1].set(title="Steady drag vs incidence — clean configuration", xlabel="Angle of attack (deg)", ylabel="Total CD")
axes[1, 0].set(title="Flap lift increment at Mach 0.4", xlabel="Angle of attack (deg)", ylabel="Total CL")
axes[1, 1].set(title="Peak absolute G in maneuver study", ylabel="Peak |G|", xticks=[0, 1, 2], xticklabels=["Pitch", "Yaw", "Roll"])
for ax in axes.flat:
    ax.grid(alpha=0.2)
    ax.legend(fontsize=8)
fig.suptitle("Nord Flight numerical aerodynamic study\nGame-model evidence — not Saab flight-data validation", fontsize=15)
fig.savefig(ROOT / "aerodynamic-study.png", dpi=180)
plt.close(fig)
print(ROOT / "aerodynamic-study.png")
