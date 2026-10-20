"""Compare authoritative diagnostics, keeping exclusions explicit and narrow."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import statistics

REPORTS=["aerodynamic-study.json","ai-flight-study.json","ai-combat-study.json",
         "bvr-study.json","air-combat-study.json","missile-gnc-study.json",
         "audio-study.json","lunar-study.json","optimization2-workloads.json"]

def physical(name,data):
    data=copy.deepcopy(data)
    def timing_only(obj):
        if isinstance(obj,dict):
            return {k:timing_only(v) for k,v in obj.items() if k!="wall_seconds"}
        if isinstance(obj,list): return [timing_only(v) for v in obj]
        return obj
    data=timing_only(data)
    if name=="aerodynamic-study.json":
        data.pop("elapsed_seconds",None)
        for k in ["jet_seconds","jet_us_per_step","orbit_seconds"]: data["benchmark"].pop(k,None)
    elif name=="audio-study.json": data.pop("render_wall_seconds",None)
    elif name=="optimization2-workloads.json":
        data.pop("seconds",None); data.pop("profile",None)
    return data

def difference(a,b,path=""):
    if type(a)!=type(b): return path+": type differs"
    if isinstance(a,dict):
        if a.keys()!=b.keys(): return path+": keys differ"
        for k in a:
            result=difference(a[k],b[k],path+"/"+str(k))
            if result:return result
    elif isinstance(a,list):
        if len(a)!=len(b):return path+": lengths differ"
        for i,(x,y) in enumerate(zip(a,b)):
            result=difference(x,y,path+"/"+str(i))
            if result:return result
    elif a!=b:return f"{path}: {a!r} != {b!r}"
    return None

def physics(base,candidate):
    result={}
    for name in REPORTS:
        a=physical(name,json.loads((base/name).read_text()))
        b=physical(name,json.loads((candidate/name).read_text()))
        result[name]={"equal":a==b,"first_difference":difference(a,b)}
    for name in ["ai-flight-study.csv","ai-combat-flight.csv"]:
        a=(base/name).read_bytes();b=(candidate/name).read_bytes()
        result[name]={"equal":a==b,"baseline_sha256":hashlib.sha256(a).hexdigest(),"candidate_sha256":hashlib.sha256(b).hexdigest()}
    return result

def render(folders):
    results={}
    for label,folder in folders.items():
        for p in folder.glob("render-*-1.json"):
            d=json.loads(p.read_text()); scene=p.stem.removeprefix("render-").removesuffix("-1")
            results.setdefault(scene,{}).setdefault(label[0],[]).append(d)
    summary={}
    for scene,versions in results.items():
        summary[scene]={}
        for version,runs in versions.items():
            summary[scene][version]={k:statistics.median(d[k] for d in runs) for k in ["mean_fps","p50_ms","p95_ms","p99_ms","simulation_rate"]}
            summary[scene][version]["samples"]=len(runs)
        if "a" in versions and "b" in versions:
            a=summary[scene]["a"];b=summary[scene]["b"]
            summary[scene]["fps_ratio"]=b["mean_fps"]/a["mean_fps"]
            summary[scene]["p95_ratio"]=b["p95_ms"]/a["p95_ms"]
    return summary

if __name__=="__main__":
    parser=argparse.ArgumentParser();parser.add_argument("base",type=Path);parser.add_argument("candidate",type=Path);parser.add_argument("output",type=Path)
    args=parser.parse_args();result=physics(args.base,args.candidate)
    args.output.write_text(json.dumps(result,indent=2)+"\n")
    print(json.dumps(result,indent=2))
    if not all(r["equal"] for r in result.values()):raise SystemExit("Physical behavior differs")
