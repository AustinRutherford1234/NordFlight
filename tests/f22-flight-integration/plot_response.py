"""Plot measured simulator traces; no real-aircraft validation is implied."""
import json
import os
from pathlib import Path

os.environ.setdefault("MPLCONFIGDIR", "/tmp/gripen-flight-model-mpl")
os.environ.setdefault("XDG_CACHE_HOME", "/tmp/gripen-flight-font-cache")
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

root = Path(__file__).resolve().parent
release = json.loads((root / "release.json").read_text())
fig, axes = plt.subplots(2, 1, figsize=(10, 7.4), sharex=True)
colors = {"IR": "#d97929", "AIM-9X": "#2869b0", "Radar": "#187d6a"}
for case in release:
    if case["release_alpha_deg"] != 20:
        continue
    samples = case["samples"]
    x = [s["t"] for s in samples]
    style = "-" if case["altitude_m"] == 3000 else "--"
    label = f'{case["weapon"]}, {case["altitude_m"]/1000:g} km'
    axes[0].plot(x, [s["incidence_deg"] for s in samples], style,
                 color=colors[case["weapon"]], label=label, linewidth=1.8)
    if case["weapon"] == "AIM-9X":
        axes[1].plot(x, [s["body_rate_deg_s"] for s in samples], style,
                     color=colors[case["weapon"]], label=label, linewidth=1.8)
for ax in axes:
    ax.axvspan(0, .22, color="#dce9ef", alpha=.55)
    ax.axvline(.22, color="#737d86", linewidth=1, linestyle=":")
    ax.axvline(.38, color="#737d86", linewidth=1, linestyle=":")
    ax.grid(alpha=.20)
    ax.spines[["top", "right"]].set_visible(False)
axes[0].set(ylabel="Body / air-velocity incidence (deg)", ylim=(0, 22))
axes[0].legend(ncol=3, frameon=False, fontsize=9, loc="upper right")
axes[1].set(ylabel="AIM-9X angular speed (deg/s)", xlabel="Time since release (s)", xlim=(0, .65))
axes[1].legend(frameon=False)
fig.suptitle("Missile release: finite weathercocking before steering handover", fontsize=14)
fig.text(.12, .015, "Production simulation path • initial incidence 20° • 250 m/s airspeed • dt = 1/240 s\n"
         "Shading: passive separation; dotted lines: control starts / finishes blending. Estimated coefficients.", fontsize=9, color="#4b5563")
fig.tight_layout(rect=(0, .07, 1, .95))
fig.savefig(root / "missile-release-response.png", dpi=180)
plt.close(fig)

baseline_path = root / "before-f22.json"
before = {r["case"]: r for r in json.loads(baseline_path.read_text())} if baseline_path.exists() else None
after = {r["case"]: r for r in json.loads((root / "after-f22.json").read_text())}
fig, axes = plt.subplots(1, 2, figsize=(11, 4.6))
for ax, case in zip(axes, ["low_speed_pull", "high_alpha_hold"]):
    series = [(after, "Updated model", "#2869b0")]
    if before is not None:
        series.insert(0, (before, "Previous model", "#8b929a"))
    for data, label, color in series:
        samples = data[case]["samples"]
        ax.plot([s["t"] for s in samples], [s["alpha"] for s in samples], label=label, color=color, linewidth=2)
    ax.axvline(8, color="#737d86", linestyle=":", linewidth=1)
    ax.set(title=case.replace("_", " ").title(), xlabel="Simulation time (s)", ylabel="Angle of attack (deg)")
    ax.grid(alpha=.20)
    ax.spines[["top", "right"]].set_visible(False)
axes[0].legend(frameon=False)
fig.suptitle("F-22 incidence response and recovery", fontsize=14)
fig.text(.10, .015, "Fixed-step simulator trials; stick released at 8 s. These curves show simulator behavior, not real F-22 flight data.", fontsize=9)
fig.tight_layout(rect=(0, .06, 1, .95))
fig.savefig(root / "f22-incidence-response.png", dpi=180)
