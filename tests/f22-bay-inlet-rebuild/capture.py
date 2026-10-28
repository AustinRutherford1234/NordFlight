"""Native, source-built F-22 detail captures. Never substitutes generated art."""
from pathlib import Path
import importlib.util, argparse, hashlib, json, shutil, time

ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location("native",ROOT/"tests/optimization3/run.py")
native=importlib.util.module_from_spec(spec);spec.loader.exec_module(native)

def main():
    p=argparse.ArgumentParser();p.add_argument("label");p.add_argument("views",nargs="*");p.add_argument("--binary",default="target/debug/gripen-flight")
    p.add_argument("--quality",action="store_true");p.add_argument("--day-phase",type=float)
    args=p.parse_args();binary=ROOT/args.binary;out=Path(__file__).parent/args.label;out.mkdir(parents=True,exist_ok=False)
    views=args.views or ["detail-bay-right","detail-bay-left","detail-inlet-front","detail-inlet-right"]
    records=[]
    with native.isolated():
        for i,view in enumerate(views):
            fields=view.split(":");mode=fields[0];env={"NORD_AIRCRAFT":"f22","NORD_FLEET_CAPTURE":mode}
            if args.quality:env["NORD_DETAIL_QUALITY"]="1"
            if args.day_phase is not None:env["NORD_DETAIL_DAY_PHASE"]=str(args.day_phase)
            if (mode.startswith("detail-launch") or mode.startswith("detail-mainbay-launch")) and len(fields)>1:env["NORD_BAY_LAUNCH_TIME"]=fields[1]
            if len(fields)>1:env["NORD_BAY_DOOR"]=fields[1]
            if len(fields)>2:env["NORD_BAY_RAIL"]=fields[2]
            if len(fields)>3:env["NORD_BAY_EMPTY" if fields[3]=="empty" else "NORD_BAY_INDEPENDENT"]="1"
            tag=f"{i+1:02d}-{mode}";source=ROOT/"tests/visual-dry.png";before=native.artifact_stamp(source);started=time.time_ns()
            native.run(binary,["--visual-check","dry"],out,tag,env,timeout=120)
            if native.artifact_stamp(source)==before or source.stat().st_mtime_ns<started:raise RuntimeError("stale capture")
            target=out/(tag+".png");shutil.copy2(source,target);records.append({"file":target.name,"mode":view,"environment":env})
    (out/"manifest.json").write_text(json.dumps({"binary":str(binary),"sha256":hashlib.sha256(binary.read_bytes()).hexdigest(),"captures":records},indent=2))

if __name__=="__main__":main()
