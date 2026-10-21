"""Focused native AIM-9X visual checks, with existing-game preservation."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tests/optimization3"))
from run import isolated, record_child

parser = argparse.ArgumentParser()
parser.add_argument("--binary", default="target/debug/gripen-flight")
parser.add_argument("--output", default="tests/aim9x-visual")
parser.add_argument("--weapon", choices=["ir", "radar"], default="ir")
args = parser.parse_args()
binary = ROOT / args.binary
out = ROOT / args.output
out.mkdir(parents=True, exist_ok=True)
captures = []
with isolated():
    for name, mode, close in [
        ("detail", "combat-launch", "detail"),
        ("vectoring-rear", "combat-vector", "rear"),
        ("vectoring-plume", "combat-vector", "plume"),
        ("smoke", "combat-flight", ""),
    ]:
        env = dict(os.environ, NORD_AIRCRAFT="f22", NORD_MISSILE_CLOSE=close,
                   NORD_MISSILE_KIND=args.weapon,
                   NORD_FLIGHT_AUDIO_SETTINGS=str(ROOT / "tests/optimization3/audio-settings.json"))
        if not close:
            env.pop("NORD_MISSILE_CLOSE", None)
        with (out / f"{name}.log").open("w") as log:
            started = time.time_ns()
            child = subprocess.Popen([str(binary), "--visual-check", mode], cwd=ROOT,
                                     env=env, stdout=log, stderr=subprocess.STDOUT)
            record_child(child.pid)
            try:
                child.wait(timeout=100)
            finally:
                if child.poll() is None:
                    child.terminate()
                    child.wait(timeout=10)
                record_child(None)
        text = (out / f"{name}.log").read_text()
        assert child.returncode == 0 and "panicked at" not in text, text[-2000:]
        source = ROOT / f"tests/visual-{mode}.png"
        assert source.exists() and source.stat().st_mtime_ns > started, "No fresh native frame"
        shutil.copy2(source, out / f"{name}.png")
        captures.append({"name": name, "mode": mode, "close": close})
(out / "captures.json").write_text(json.dumps({
    "executable": str(binary), "sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
    "captures": captures,
}, indent=2) + "\n")
print(f"Verified {len(captures)} fresh native captures in {out}")
