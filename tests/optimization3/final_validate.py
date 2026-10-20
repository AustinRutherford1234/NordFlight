"""Final sequential validation; never runs compilers or overlapping workloads."""
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
from run import OUT, ROOT
import argparse

options = argparse.ArgumentParser()
options.add_argument("--prefix", default="accepted")
options = options.parse_args()
SESSION = OUT / options.prefix
SESSION.mkdir(parents=True, exist_ok=True)
def label(value):
    return str(Path(options.prefix) / value)

base = OUT / "baseline/gripen-flight-release"
candidate = OUT / "candidate/gripen-flight-release"
child = None

def stage(name, arguments, extra=None):
    global child
    print("START " + name, flush=True)
    (SESSION / "final-progress.json").write_text(json.dumps({"stage": name, "status": "running"}, indent=2))
    env = os.environ.copy()
    env.update(NORD_COMPUTE_THREADS="6")
    env.update(extra or {})
    try:
        child = subprocess.Popen([sys.executable, *map(str, arguments)], cwd=ROOT, env=env)
        code = child.wait()
        child = None
        if code:
            raise RuntimeError(f"{name} failed: {code}")
    except BaseException:
        if child is not None and child.poll() is None:
            child.terminate()
            child.wait(timeout=30)
        (SESSION / "final-progress.json").write_text(json.dumps({"stage": name, "status": "failed"}, indent=2))
        raise
    print("DONE " + name, flush=True)

def suite(binary, name, kind, *extra, env=None):
    stage(name + "/" + kind, [OUT / "run.py", binary, label(name), kind, *extra], env)

if __name__ == "__main__":
    signal.signal(signal.SIGTERM, lambda *_: (_ for _ in ()).throw(KeyboardInterrupt()))
    suite(base, "release-baseline", "cpu")
    suite(candidate, "release-candidate", "cpu")
    stage("physical-equivalence", [OUT / "verify.py", SESSION / "release-baseline", SESSION / "release-candidate", SESSION / "release-physics-equivalence.json"])
    stage("cpu-alternating", [OUT / "cpu_pairs.py", base, candidate, label("release-cpu-pairs")])
    suite(candidate, "release-candidate", "visual")
    suite(base, "release-baseline", "visual")
    suite(candidate, "release-cache-off", "visual", "--visuals", "earth-flight-12,earth-flight-101",
          env={"BEVY_DISABLE_STATIC_ATMOSPHERE_CACHE": "1", "BEVY_DISABLE_ENV_BIND_GROUP_CACHE": "1", "BEVY_DISABLE_ENV_CONTENT_CACHE": "1"})
    stage("native-image-comparison", [OUT / "visual_compare.py", SESSION / "release-baseline", SESSION / "release-candidate", SESSION / "release-visual-equivalence.json"])
    stage("worker-counts", [OUT / "workers.py", candidate, label("release-workers")])
    stage("metal-accepted", [OUT / "profile.py", candidate, label("metal-accepted")])
    stage("metal-orbit", [OUT / "profile.py", candidate, label("metal-orbit"), "--scene", "earth-flight-220"])
    try:
        stage("cpu-profile", [OUT / "profile.py", candidate, label("cpu-profile"), "--template", "Time Profiler", "--scene", "bvr-cockpit-missiles", "--live", "--debuggable"])
    except RuntimeError as error:
        # Profiling support is optional hardware/tool evidence, not an FPS or
        # correctness result. Retain the failure explicitly and continue those.
        (SESSION / "cpu-profile/unavailable.json").write_text(json.dumps({"available": False, "error": str(error)}, indent=2))
        print("CPU profile unavailable; inspect retained capture logs", flush=True)
    stage("render-key", [OUT / "matrix.py", base, candidate, label("release-matrix"), "--scenes", "cockpit,bvr,earth60,orbit"])
    for script, seconds in (("ascent", "90"), ("descent", "60")):
        for reference in (True, False):
            name = "motion-" + script + ("-cpu" if reference else "-gpu")
            env = {"NORD_CAPTURE_SEQUENCE": str(SESSION / name / "frames"), "NORD_BENCH_SECONDS": seconds, "NORD_BENCH_WARMUP": "1"}
            if reference:
                env["NORD_EARTH_CPU"] = "1"
            suite(candidate, name, "render", "--scenes", script, env=env)
    suite(candidate, "release-soak", "soak")
    stage("render-remaining", [OUT / "matrix.py", base, candidate, label("release-matrix"), "--scenes",
                             "runway,airborne,combat,terrain,earthmap,moonmap,ascent,descent,landing,warp"])
    stage("render-paused", [OUT / "matrix.py", base, candidate, label("release-paused-matrix"), "--scenes", "bvr,earth60,orbit", "--paused"])
    stage("report", [OUT / "report.py", SESSION])
    (SESSION / "final-progress.json").write_text(json.dumps({"stage": "all-suites", "status": "complete"}, indent=2))
