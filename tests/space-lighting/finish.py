"""Bounded delivery checks for the Sun/Nordic and cockpit-director fixes."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(__file__).resolve().parent

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def main():
    # The caller starts exactly one final release build; do not race its output.
    while subprocess.run(["kill", "-0", "51506"], capture_output=True).returncode == 0:
        time.sleep(1)
    log = (OUT / "release-build.log").read_text()
    assert 'Finished `release` profile' in log, log
    candidate = OUT / "candidate"
    shutil.copy2(ROOT / "target/release/gripen-flight", candidate)
    source = ["Cargo.toml", "Cargo.lock", "src/main.rs", "src/celestial.rs",
              "src/world.rs", "src/earth_region.rs", "src/earth_gpu.rs",
              "src/earth_vertex.wgsl", "src/cockpit.rs", "src/combat/hud.rs",
              "src/combat/dogfight.rs"]
    source_hashes = {p: sha(ROOT / p) for p in source}
    (OUT / "source-hashes.json").write_text(json.dumps(source_hashes, indent=2) + "\n")
    app = ROOT / "Nord Flight.app"
    executable = app / "Contents/MacOS/gripen-flight"
    assert sha(executable) == sha(OUT / "baseline"), "Installed build changed during work"
    stage = executable.with_name("gripen-flight.validated-stage")
    shutil.copy2(candidate, stage)
    subprocess.run(["strip", "-x", str(stage)], check=True)
    os.replace(stage, executable)
    subprocess.run(["codesign", "--force", "--sign", "-", str(app)], check=True)
    check = subprocess.run(["codesign", "--verify", "--deep", "--strict", "--verbose=2", str(app)],
                           check=True, capture_output=True, text=True)
    delivery = {"installed_sha256": sha(executable), "candidate_sha256": sha(candidate),
                "baseline_sha256": sha(OUT / "baseline"),
                "signature": check.stdout + check.stderr,
                "uuid": subprocess.check_output(["dwarfdump", "--uuid", str(executable)], text=True).strip(),
                "running_instance_requires_restart": True, "application_tests_passed": 304,
                "native_checks": [], "state": "native-checks"}
    record = OUT / "delivery.json"
    record.write_text(json.dumps(delivery, indent=2) + "\n")
    spec = importlib.util.spec_from_file_location("native", ROOT / "tests/optimization3/run.py")
    native = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(native)
    folder = OUT / "release"
    folder.mkdir(exist_ok=True)
    scenes = ["earth-flight-220-sun-external", "earth-flight-220-sun-external-night",
              "earth-flight-220", "earth-flight-220-map", "earth-flight-99", "earth-flight-101",
              "air-hud-cockpit", "air-hud-cockpit-simple"]
    with native.isolated():
        for scene in scenes:
            native.run(executable, ["--visual-check", scene], folder, scene, timeout=120)
            shutil.copy2(ROOT / f"tests/visual-{scene}.png", folder / f"{scene}.png")
            if "cockpit" in scene:
                shutil.copy2(ROOT / "tests/jet-panel-3.png", folder / f"{scene}-combiner.png")
            delivery["native_checks"].append(scene)
            record.write_text(json.dumps(delivery, indent=2) + "\n")
    assert all(sha(ROOT / p) == digest for p, digest in source_hashes.items())
    delivery["state"] = "complete"
    record.write_text(json.dumps(delivery, indent=2) + "\n")
    print("Delivery complete; prior running flight preserved.", flush=True)

if __name__ == "__main__":
    main()
