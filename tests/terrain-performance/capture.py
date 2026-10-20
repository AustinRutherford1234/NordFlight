"""Native reference/candidate screenshots; run separately from timing tests."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
from PIL import Image, ImageChops, ImageStat

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tests/optimization3"))
from run import isolated, record_child

p = argparse.ArgumentParser()
p.add_argument("binary", type=Path)
p.add_argument("output", type=Path)
p.add_argument("--only",choices=["forest","cockpit","earth12","earth60"])
args = p.parse_args()
args.output.mkdir(parents=True, exist_ok=True)
results = {}
with isolated():
    for label, scene, cockpit in [("forest", "terrain-forest", False),
                                   ("cockpit", "terrain-forest", True),
                                   ("earth12", "earth-flight-12", False),
                                   ("earth60", "earth-flight-60", False)]:
        if args.only and args.only != label:
            continue
        for reference in [True, False]:
            name = label + ("-reference" if reference else "-candidate")
            env = os.environ.copy()
            for key in ["NORD_ENV_SERIAL_IRRADIANCE", "NORD_TERRAIN_LARGE_BATCHES",
                        "NORD_EARTH_DRAW_ZERO_ALPHA", "NORD_TERRAIN_COCKPIT", "NORD_REFERENCE_LIGHTS",
                        "NORD_BENCH_SCRIPT", "NORD_BENCH_SCENE"]:
                env.pop(key, None)
            env.update(NORD_BENCH_FULLSCREEN="1", NORD_TERRAIN_AGL="50",
                       NORD_FLIGHT_AUDIO_SETTINGS=str(ROOT / "tests/optimization3/audio-settings.json"))
            if cockpit:
                env["NORD_TERRAIN_COCKPIT"]="1"
            if reference:
                for key in ["NORD_ENV_SERIAL_IRRADIANCE", "NORD_TERRAIN_LARGE_BATCHES",
                            "NORD_EARTH_DRAW_ZERO_ALPHA", "NORD_REFERENCE_LIGHTS"]:
                    env[key] = "1"
            source = ROOT / f"tests/visual-{scene}.png"
            before = source.stat().st_mtime_ns if source.exists() else None
            with (args.output / (name + ".log")).open("w") as log:
                command=[str(args.binary.resolve()), "--visual-check", scene]
                child = subprocess.Popen(command,
                                         cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT)
                record_child(child.pid)
                try:
                    child.wait(timeout=90)
                finally:
                    if child.poll() is None:
                        child.terminate(); child.wait(timeout=10)
                    record_child(None)
                assert child.returncode == 0
            assert source.exists() and source.stat().st_mtime_ns != before
            shutil.copy2(source, args.output / (name + ".png"))
            print(name, flush=True)
        a = Image.open(args.output / (label + "-reference.png")).convert("RGB")
        b = Image.open(args.output / (label + "-candidate.png")).convert("RGB")
        delta = ImageChops.difference(a, b)
        results[label] = {"resolution": a.size, "mean_absolute_rgb_255": ImageStat.Stat(delta).mean,
                          "max_rgb_255": [hi for lo, hi in delta.getextrema()],
                          "different_pixel_bounds": delta.getbbox()}
        (args.output / "differences.json").write_text(json.dumps(results, indent=2) + "\n")
