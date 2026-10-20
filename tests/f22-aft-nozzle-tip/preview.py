"""Plot the exported runtime nozzle meshes, not a second model implementation.

Usage: target/debug/gripen-flight --export-f22-nozzle > tests/f22-aft-nozzle-tip/mesh.json
       python3 tests/f22-aft-nozzle-tip/preview.py
"""
import json
import time
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import numpy as np
from mpl_toolkits.mplot3d.art3d import Poly3DCollection

root = Path(__file__).resolve().parent
start = time.perf_counter()
data = json.loads((root / 'mesh.json').read_text())
fig = plt.figure(figsize=(15, 8), facecolor='#161d25')
light = np.array([-.35, .8, .7]); light /= np.linalg.norm(light)
for n, pose in enumerate(data['poses']):
    for row, (elev, azim, title) in enumerate([(55, 42, 'Upper / linked sections'), (0, 90, 'Side / fixed wall')]):
        ax = fig.add_subplot(2, 3, row*3+n+1, projection='3d', facecolor='#161d25')
        triangles, facecolors = [], []
        for part in pose['parts']:
            p = np.asarray(part['positions'])
            # Display engine aft along X, width along Y, vertical along Z.
            p = p[:, [2, 0, 1]]
            tri = p[np.asarray(part['indices']).reshape(-1, 3)]
            normal = np.cross(tri[:, 1]-tri[:, 0], tri[:, 2]-tri[:, 0])
            normal /= np.maximum(np.linalg.norm(normal, axis=1, keepdims=True), 1e-12)
            shade = .35 + .65*np.abs(normal @ light)
            rgb = np.asarray(part['color'][:3])
            colors = np.clip(rgb[None, :]*shade[:, None]*1.7, 0, 1)
            triangles.extend(tri); facecolors.extend(colors)
        # One collection allows per-triangle depth sorting. Separate mesh
        # collections incorrectly painted broad plates over raised panels.
        ax.add_collection3d(Poly3DCollection(triangles, facecolors=facecolors, edgecolors='none', linewidths=0))
        ax.set(xlim=(-1.5, 2.0), ylim=(-1.15, 1.15), zlim=(-1.2, 1.2))
        ax.set_box_aspect((3.5, 2.3, 2.4)); ax.view_init(elev=elev, azim=azim)
        ax.set_axis_off()
        ax.set_title(f"{pose['name'].upper()} · {title}", color='#dbe8f4', fontsize=11)
fig.suptitle('Actual F-22 nozzle meshes — geometry inspection only; flat plot is not PBR material validation', color='white', fontsize=13)
fig.tight_layout()
fig.savefig(root/'geometry-preview.png', dpi=130, facecolor=fig.get_facecolor())
fixed = [[p['positions'] for p in pose['parts'] if p['fixed']] for pose in data['poses']]
assert fixed[0] == fixed[1] == fixed[2], 'Fixed sidewalls/duct moved between vector commands'
print(f'Plotted actual runtime meshes in {time.perf_counter()-start:.2f} s')
