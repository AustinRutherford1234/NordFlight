"""Short sequential, battery-powered matched-development-build comparison."""
from pathlib import Path
import hashlib, importlib.util, json

ROOT=Path(__file__).resolve().parents[2]
OUT=Path(__file__).parent/"performance"
spec=importlib.util.spec_from_file_location("native",ROOT/"tests/optimization3/run.py")
native=importlib.util.module_from_spec(spec);spec.loader.exec_module(native)

def main():
    OUT.mkdir(exist_ok=False)
    binaries={"baseline":Path(__file__).parent/"baseline/gripen-flight-baseline",
              "candidate":ROOT/"target/debug/gripen-flight"}
    (OUT/"executables.json").write_text(json.dumps({k:{"path":str(v),"sha256":hashlib.sha256(v.read_bytes()).hexdigest()}
        for k,v in binaries.items()},indent=2))
    records=[]
    with native.isolated():
        for scene,repeat in [("bays",1),("cockpit",1)]:
            for version,binary in binaries.items():
                tag=f"{scene}-{repeat}-{version}";report=OUT/(tag+".json")
                env={"NORD_AIRCRAFT":"f22","NORD_FLEET_CAPTURE":scene,
                     "NORD_BENCH_SCENE":"air-hud","NORD_BENCH_LIVE":"0",
                     "NORD_BENCH_SCRIPT":"","NORD_BENCH_WARMUP":"5",
                     "NORD_BENCH_SECONDS":"12","NORD_BENCH_REPORT":str(report)}
                native.run(binary,["--benchmark-render",tag],OUT,tag,env,timeout=90)
                records.append({**json.loads(report.read_text()),"version":version,"workload":scene,"repeat":repeat})
    (OUT/"summary.json").write_text(json.dumps(records,indent=2))
    for r in records:
        print(f"{r['workload']} {r['repeat']} {r['version']}: {r['mean_fps']:.2f} FPS; p95 {r['p95_ms']:.2f} ms")

if __name__=="__main__":main()
