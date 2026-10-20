"""Summarize Time Profiler statistical samples; run after timing workloads stop."""
import argparse
from collections import Counter
import json
from pathlib import Path
import xml.etree.ElementTree as ET

parser = argparse.ArgumentParser()
parser.add_argument("folder", type=Path)
args = parser.parse_args()
root = ET.parse(args.folder / "cpu.xml").getroot()
ids = {element.attrib["id"]: element for element in root.iter() if "id" in element.attrib}
columns = [column.findtext("mnemonic") for column in root.find(".//schema").findall("col")]
def resolve(element):
    while "ref" in element.attrib:
        element = ids[element.attrib["ref"]]
    return element

leaf, inclusive, states, threads = Counter(), Counter(), Counter(), Counter()
samples = 0
missing_ms = 0.0
for row in root.iter("row"):
    values = {name: resolve(element) for name, element in zip(columns, row)}
    if "gripen" not in values["process"].attrib.get("fmt", ""):
        continue
    samples += 1
    weight = int(values["weight"].text) / 1e6
    state = values["thread-state"].attrib.get("fmt", values["thread-state"].text or "Unknown")
    states[state] += weight
    threads[values["thread"].attrib.get("fmt", "Unknown")] += weight
    frames = [resolve(element).attrib.get("name", "Unknown") for element in values["stack"].iter("frame")]
    if not frames:
        missing_ms += weight
        continue
    leaf[frames[0]] += weight
    for name in set(frames):
        inclusive[name] += weight

keywords = ("gripen", "physics", "aero", "collision", "lake", "submit", "wait", "park", "prepare", "raster")
result = {
    "samples": samples, "sample_weight_ms": sum(states.values()), "missing_backtrace_weight_ms": missing_ms,
    "states_weight_ms": dict(states.most_common()), "threads_weight_ms": dict(threads.most_common()),
    "top_leaf_weight_ms": dict(leaf.most_common(40)),
    "application_and_sync_inclusive_weight_ms": {name: value for name,value in inclusive.most_common() if any(word in name.lower() for word in keywords)},
    "note": "Statistical CPU samples across all sampled threads. Inclusive stacks overlap and cannot be summed. CPU weight is not frame wall time or GPU time. Missing stacks are retained explicitly; unsampled waiting is not measured as zero work.",
}
(args.folder / "summary.json").write_text(json.dumps(result, indent=2) + "\n")
print(json.dumps({key: result[key] for key in ("samples", "sample_weight_ms", "missing_backtrace_weight_ms", "top_leaf_weight_ms")}, indent=2))
