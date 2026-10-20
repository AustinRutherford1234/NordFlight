"""Sequential native validation with fresh artifacts and recoverable isolation."""
import argparse
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import signal
import subprocess
import sys
import threading
import time

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "tests/optimization3"
JOURNAL = OUT / "isolation.json"
SCENES = {
    "runway": ("dry", False), "cockpit": ("cockpit", False),
    "airborne": ("air-hud", True), "combat": ("air-component-fire", True),
    "bvr": ("bvr-cockpit-missiles", True), "terrain": ("terrain-forest", True),
    "earth60": ("earth-flight-60", True), "orbit": ("earth-flight-220", True),
    "earthmap": ("lunar-earth-map", False), "moonmap": ("lunar-node-map", False),
    "ascent": ("earth-flight-12", True), "descent": ("capsule-entry", True),
    "landing": ("lunar-landing", True), "warp": ("earth-flight-220", True),
}
VISUALS = ["cockpit", "bvr-cockpit-missiles", "air-component-fire", "terrain-forest",
           "air-defenses",
           "earth-flight-12", "earth-flight-60", "earth-flight-99", "earth-flight-101",
           "earth-flight-220", "lunar-earth-map", "lunar-node-map"]
STUDIES = {
    "flight": ["aerodynamic-study.json"],
    "ai": ["ai-flight-study.json", "ai-combat-study.json", "ai-flight-study.csv", "ai-combat-flight.csv"],
    "bvr": ["bvr-study.json"], "air-combat": ["air-combat-study.json"],
    "missiles": ["missile-gnc-study.json"], "audio": ["audio-study.json"],
    "lunar": ["lunar-study.json"], "workloads": ["optimization2-workloads.json"],
}

def command(args):
    return subprocess.run(args, capture_output=True, text=True).stdout.strip()

def identity(pid):
    value=command(["ps","-o","lstart=,comm=","-p",str(pid)])
    return {"pid":pid,"identity":value} if value else None

def same_process(record):
    return record and identity(record["pid"])==record

def save_journal(data):
    temporary=JOURNAL.with_suffix(".tmp")
    temporary.write_text(json.dumps(data,indent=2))
    temporary.replace(JOURNAL)

def recover(data):
    child=data.get("child")
    if same_process(child):
        try: os.kill(child["pid"],signal.SIGTERM)
        except ProcessLookupError: pass
        for _ in range(30):
            if not same_process(child): break
            time.sleep(0.1)
        if same_process(child):
            try: os.kill(child["pid"],signal.SIGKILL)
            except ProcessLookupError: pass
    for record in data.get("suspended",[]):
        if same_process(record):
            try: os.kill(record["pid"],signal.SIGCONT)
            except ProcessLookupError: pass

def watchdog(parent):
    while JOURNAL.exists():
        try: data=json.loads(JOURNAL.read_text())
        except (OSError,json.JSONDecodeError): time.sleep(0.2); continue
        if not same_process(parent):
            recover(data)
            JOURNAL.unlink(missing_ok=True)
            return
        time.sleep(1)

@contextmanager
def isolated():
    OUT.mkdir(parents=True,exist_ok=True)
    if JOURNAL.exists():
        data=json.loads(JOURNAL.read_text())
        if same_process(data.get("owner")): raise RuntimeError("Another runner owns isolation")
        recover(data); JOURNAL.unlink()
    data={"owner":identity(os.getpid()),"suspended":[],"child":None}
    save_journal(data)
    guard=subprocess.Popen([sys.executable,str(Path(__file__).resolve()),"watchdog",json.dumps(data["owner"])],
                           stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,start_new_session=True)
    # A display sleeping halfway through a native run invalidates its FPS data.
    # The assertion also ends if the runner is killed, because it watches this PID.
    awake=subprocess.Popen(["caffeinate","-di","-w",str(os.getpid())],
                           stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
    try:
        for line in command(["ps","-axo","pid=,state=,comm="]).splitlines():
            fields=line.strip().split(None,2)
            if len(fields)==3 and fields[2]==str(ROOT/"Nord Flight.app/Contents/MacOS/gripen-flight") and "T" not in fields[1]:
                record=identity(int(fields[0]))
                if record:
                    data["suspended"].append(record); save_journal(data)
                    try: os.kill(record["pid"],signal.SIGSTOP)
                    except ProcessLookupError: pass
        yield
    finally:
        if JOURNAL.exists():
            recover(json.loads(JOURNAL.read_text())); JOURNAL.unlink(missing_ok=True)
        awake.terminate(); awake.wait(timeout=5)
        guard.wait(timeout=5)

def record_child(pid):
    if JOURNAL.exists():
        data=json.loads(JOURNAL.read_text()); data["child"]=identity(pid) if pid else None
        save_journal(data)

def artifact_stamp(path):
    return path.stat().st_mtime_ns if path.exists() else None

def copy_fresh(path,folder,before,started_ns):
    if not path.exists() or artifact_stamp(path)==before or path.stat().st_mtime_ns < started_ns:
        raise RuntimeError(f"Missing/stale artifact: {path}")
    shutil.copy2(path,folder/path.name)

def run(binary,args,folder,tag,extra=None,timeout=300):
    env=os.environ.copy(); env.update(NORD_FLIGHT_AUDIO_SETTINGS=str(OUT/"audio-settings.json")); env.update(extra or {})
    logpath=folder/(tag+".log")
    if logpath.exists(): raise RuntimeError(f"Output exists; choose a fresh label: {logpath}")
    samples=[]; stop=threading.Event()
    power_begin=command(["pmset","-g","batt"])
    start=time.perf_counter()
    with logpath.open("w") as log:
        process=subprocess.Popen([str(binary),*args],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT)
        record_child(process.pid)
        def sample():
            next_status=300
            while not stop.is_set():
                ps=command(["ps","-o","rss=,%cpu=","-p",str(process.pid)]).split()
                if len(ps)==2:
                    record={"wall_s":time.perf_counter()-start,"rss_kib":int(ps[0]),"cpu_percent":float(ps[1])}
                    if record["wall_s"]>=next_status:
                        record["thermal"]=command(["pmset","-g","therm"])
                        record["power"]=command(["pmset","-g","batt"])
                        next_status+=300
                    samples.append(record)
                stop.wait(2)
        sampler=threading.Thread(target=sample,daemon=True); sampler.start()
        try:
            process.wait(timeout=timeout); seconds=time.perf_counter()-start
        finally:
            if process.poll() is None:
                process.terminate()
                try: process.wait(timeout=10)
                except subprocess.TimeoutExpired: process.kill(); process.wait()
            stop.set(); sampler.join(timeout=5); record_child(None)
    result={"seconds":seconds,"exit":process.returncode,"memory":samples,
            "power_begin":power_begin,"power_end":command(["pmset","-g","batt"]),
            "thermal_end":command(["pmset","-g","therm"]),
            "peak_rss_kib":max((s["rss_kib"] for s in samples),default=0)}
    (folder/(tag+"-process.json")).write_text(json.dumps(result,indent=2))
    errors=logpath.read_text(errors="replace")
    if process.returncode or "ERROR " in errors or "panicked at" in errors:
        raise RuntimeError(f"Failed {tag}: inspect {logpath}")
    print(f"{tag}: {seconds:.3f}s, RSS {result['peak_rss_kib']/1024:.0f} MiB",flush=True)

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument("binary",type=Path); parser.add_argument("label")
    parser.add_argument("suite",choices=["cpu","visual","render","soak"])
    parser.add_argument("--scenes",default=",".join(SCENES)); parser.add_argument("--repeat",type=int,default=1)
    parser.add_argument("--visuals",default=",".join(VISUALS))
    parser.add_argument("--studies",default=",".join(STUDIES))
    parser.add_argument("--paused",action="store_true",help="Render the fixed visual preset without scripted controls or simulation")
    parser.add_argument("--quality",action="store_true",help="Use existing benchmark high-scattering/4x MSAA mode; startup shadow map stays 1024 (in-game F9 also changes shadows)")
    args=parser.parse_args(); binary=args.binary.resolve(); folder=OUT/args.label
    folder.mkdir(parents=True,exist_ok=True)
    manifest={"binary":str(binary),"sha256":hashlib.sha256(binary.read_bytes()).hexdigest()}
    target=folder/"executable.json"
    if target.exists() and json.loads(target.read_text())!=manifest: raise RuntimeError("Label belongs to another binary")
    target.write_text(json.dumps(manifest,indent=2))
    (folder/(args.suite+"-host.json")).write_text(json.dumps({
        "platform":platform.platform(),"date":command(["date","-u"]),
        "cpu":command(["sysctl","-n","machdep.cpu.brand_string"]),"memory_bytes":command(["sysctl","-n","hw.memsize"]),
        "power":command(["pmset","-g","batt"]),"thermal":command(["pmset","-g","therm"]),
        "environment":{k:v for k,v in os.environ.items() if k.startswith("NORD_") or k in ("BEVY_DISABLE_STATIC_ATMOSPHERE_CACHE", "BEVY_DISABLE_ENV_BIND_GROUP_CACHE", "BEVY_DISABLE_ENV_CONTENT_CACHE")}},indent=2))
    if args.suite=="cpu":
        for name,reports in STUDIES.items():
            if name not in args.studies.split(","): continue
            before={p:artifact_stamp(ROOT/"tests"/p) for p in reports}; started=time.time_ns()
            run(binary,["--study-"+name],folder,name,timeout=600)
            for p in reports: copy_fresh(ROOT/"tests"/p,folder,before[p],started)
    elif args.suite=="visual":
        for scene in args.visuals.split(","):
            p=ROOT/"tests"/("visual-"+scene+".png"); before=artifact_stamp(p); started=time.time_ns()
            run(binary,["--visual-check",scene],folder,"visual-"+scene)
            copy_fresh(p,folder,before,started)
    elif args.suite=="render":
        for repeat in range(args.repeat):
            for name in args.scenes.split(","):
                scene,live=SCENES[name]; live=live and not args.paused
                tag=f"render-{name}-{repeat+1}"; report=folder/(tag+".json")
                before=artifact_stamp(report); started=time.time_ns()
                run(binary,["--benchmark-render",tag + ("-quality" if args.quality else "")],folder,tag,
                    {"NORD_BENCH_SCENE":scene,"NORD_BENCH_LIVE":str(int(live)),"NORD_BENCH_SCRIPT":"" if args.paused else name,"NORD_BENCH_REPORT":str(report)})
                if artifact_stamp(report)==before or report.stat().st_mtime_ns < started: raise RuntimeError("Stale render report")
    else:
        run(binary,["--benchmark-render","soak"],folder,"soak",
            {"NORD_BENCH_SCENE":"bvr-cockpit-missiles","NORD_BENCH_LIVE":"1",
             "NORD_BENCH_SECONDS":os.environ.get("NORD_BENCH_SECONDS","1800"),"NORD_BENCH_SOAK":"1",
             "NORD_BENCH_REPORT":str(folder/"soak.json")},timeout=1950)

if __name__=="__main__":
    if len(sys.argv)>1 and sys.argv[1]=="watchdog": watchdog(json.loads(sys.argv[2]))
    else:
        signal.signal(signal.SIGTERM,lambda *_: (_ for _ in ()).throw(KeyboardInterrupt()))
        with isolated(): main()
