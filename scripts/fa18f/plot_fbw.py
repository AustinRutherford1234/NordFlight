"""Plot deterministic Hornet FBW response evidence from the headless tests.
Run the FBW response_matrix test first, then invoke this script with a Python
installation containing matplotlib. Captures are generated under tests/.
"""
import argparse
import json
from pathlib import Path
from math import degrees
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

parser = argparse.ArgumentParser()
parser.add_argument('--input', type=Path, default=Path('tests/hornet-fbw/response.json'))
parser.add_argument('--output', type=Path, default=Path('tests/hornet-fbw/response.png'))
args = parser.parse_args()
rows = json.loads(args.input.read_text())
groups = {}
for row in rows:
    key = (row['speed_case'], row['flaps'], row['pitch_input'], row['roll_input'])
    groups.setdefault(key, []).append(row)
fig, axes = plt.subplots(2, 2, figsize=(13, 8), constrained_layout=True)
for key, data in groups.items():
    speed, flaps, pitch, roll = key
    time = [r['time'] for r in data]
    label = f'{speed:g} m/s, flap {flaps:.0%}, pitch {pitch:g}, roll {roll:g}'
    axes[0, 0].plot(time, [r['g'] for r in data], label=label)
    axes[0, 1].plot(time, [r['alpha_deg'] for r in data], label=label)
    if roll:
        line, = axes[1, 0].plot(time, [degrees(r['rates'][2]) for r in data], label=f'roll input {roll:+g}')
        axes[1, 0].plot(time, [degrees(r['command_rates'][2]) for r in data], '--', color=line.get_color(), alpha=.6)
    if pitch:
        line, = axes[1, 1].plot(time, [degrees(r['rates'][0]) for r in data], label=f'{speed:g} m/s, flap {flaps:.0%}')
        axes[1, 1].plot(time, [degrees(r['command_rates'][0]) for r in data], '--', color=line.get_color(), alpha=.6)
for ax, title, ylabel in zip(axes.flat,
        ['Normal load response', 'Angle of attack', 'Roll rate: solid achieved, dashed requested', 'Pitch rate: solid achieved, dashed requested'],
        ['Normal load (g)', 'Angle of attack (degrees)', 'Body Z rate (degrees/s)', 'Body X rate (degrees/s)']):
    ax.axvspan(1, 4, color='gray', alpha=.1)
    ax.set(xlabel='Simulation time (s)', ylabel=ylabel, title=title)
    ax.grid(alpha=.25)
    ax.legend(fontsize=7, loc='best')
fig.suptitle('NordFlight F/A-18F FBW — simulated response, shaded interval = pilot input\nEngineering estimates; these are not real-aircraft flight-test comparisons', fontsize=12)
args.output.parent.mkdir(parents=True, exist_ok=True)
fig.savefig(args.output, dpi=150)
print(args.output)
