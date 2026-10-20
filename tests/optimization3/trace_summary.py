"""Summarize exported Metal intervals without conflating parallel GPU channels."""
import argparse
from collections import Counter, defaultdict
import json
from pathlib import Path
import statistics
import re
import xml.etree.ElementTree as ET

parser = argparse.ArgumentParser()
parser.add_argument("folder", type=Path)
args = parser.parse_args()
root = ET.parse(args.folder / "gpu.xml").getroot()
ids = {e.attrib["id"]: e for e in root.iter() if "id" in e.attrib}
columns = [c.findtext("mnemonic") for c in root.find(".//schema").findall("col")]

def resolve(e):
    while "ref" in e.attrib:
        e = ids[e.attrib["ref"]]
    return e

totals = defaultdict(float)
per_frame = defaultdict(lambda: defaultdict(float))
frames = set()
for row in root.iter("row"):
    values = {name: resolve(e) for name, e in zip(columns, row)}
    if "gripen" not in values["process"].attrib.get("fmt", "") or values["state"].text != "Active":
        continue
    label = values["event-label"].attrib.get("fmt", "").split("   ")[0]
    channel = values["channel-name"].text
    frame = values["frame-number"].text
    if not label or frame is None:
        continue
    name = f"{label} [{channel}]"
    ms = int(values["duration"].text) / 1e6
    totals[name] += ms
    per_frame[name][frame] += ms
    frames.add(frame)

summary = {
    "distinct_frame_labels": len(frames),
    "note": "Active event intervals grouped by GPU channel. Channels and nested intervals can overlap; these totals are not frame wall time. Profiled capture is not an uninstrumented FPS run.",
    "events": {
        name: {"total_active_ms": totals[name], "frames_with_event": len(per_frame[name]),
               "median_ms_per_observed_frame": statistics.median(per_frame[name].values())}
        for name in sorted(totals, key=totals.get, reverse=True)
    },
}
(args.folder / "aggregate.json").write_text(json.dumps(summary, indent=2) + "\n")
print(json.dumps({"frames": len(frames), "top": list(summary["events"].items())[:12]}, indent=2))

resources = args.folder / "resources.xml"
if resources.exists():
    resource_root = ET.parse(resources).getroot()
    resource_ids = {e.attrib["id"]: e for e in resource_root.iter() if "id" in e.attrib}
    resource_columns = [c.findtext("mnemonic") for c in resource_root.find(".//schema").findall("col")]
    counts, sizes = Counter(), Counter()
    timestamps = []
    for row in resource_root.iter("row"):
        values = {}
        for name, e in zip(resource_columns, row):
            while "ref" in e.attrib:
                e = resource_ids[e.attrib["ref"]]
            values[name] = e
        if "gripen" not in values["process"].attrib.get("fmt", "") or values["event-type"].text != "Allocation":
            continue
        label = re.sub(r"Resource 0x[0-9a-fA-F]+", "Unlabeled", values["label"].text or "Unlabeled")
        name = (values["resource-type"].text or "Unknown") + ":" + label
        counts[name] += 1
        sizes[name] += int(values["resource-size"].text or 0)
        timestamps.append(int(values["timestamp"].text) / 1e9)
    observed = len(per_frame.get("main_opaque_pass_3d:main_opaque_pass_3d [Vertex]", {}))
    allocations = {"note": "Metal resource creation turnover, not resident memory or general CPU malloc counts. Texture-view events may have zero bytes. Window edges can include partial frames.",
                   "observed_opaque_frames": observed, "allocation_events": sum(counts.values()),
                   "allocated_bytes": sum(sizes.values()), "window_s": [min(timestamps), max(timestamps)] if timestamps else [],
                   "events_per_observed_frame": sum(counts.values()) / max(1, observed),
                   "by_label": {name: {"events": n, "bytes": sizes[name], "events_per_observed_frame": n / max(1, observed)} for name, n in counts.most_common()}}
    (args.folder / "resource-summary.json").write_text(json.dumps(allocations, indent=2) + "\n")
