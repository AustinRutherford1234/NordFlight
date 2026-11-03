#!/usr/bin/env python3
"""Inspect, verify, install or remove offline terrain packs outside flight."""
import argparse,hashlib,json,shutil,subprocess,os
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
DEST=ROOT/'assets/environment/packs'
def verify(path):
    m=json.loads((path/'manifest.json').read_text())
    if m['version']!=1 or m['id']!=path.name:raise ValueError('invalid pack identity')
    total=0
    for t in m['tiles']:
        p=Path(t['file'])
        if len(p.parts)!=1 or p.name in ['.','..']:raise ValueError('unsafe tile path')
        payload=(path/p).read_bytes()
        if len(payload)!=t['bytes'] or hashlib.sha256(payload).hexdigest()!=t['sha256']:raise ValueError(f'corrupt tile {p}')
        total+=len(payload)
    return m,total
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('action',choices=['list','verify','install','remove'])
p.add_argument('pack',nargs='?');a=p.parse_args()
if a.action=='list':
    for path in sorted(DEST.glob('*/manifest.json')):
        m,n=verify(path.parent);print(m['id'],f'{n/1048576:.1f} MiB',len(m['tiles']),'tiles')
else:
    if not a.pack:p.error('pack path or id is required')
    if a.action in ['install','remove']:
        if os.name=='nt':
            listing=subprocess.run(['tasklist','/FO','CSV','/NH'],capture_output=True,text=True).stdout.lower()
            running='"nordflight.exe"' in listing or '"gripen-flight.exe"' in listing
        else:
            running=subprocess.run(['pgrep','-f','[/]gripen-flight'],capture_output=True).returncode==0
        if running:raise SystemExit('Close Nord Flight before installing or removing packs')
    if a.action=='verify':print(verify(DEST/a.pack)[1],'bytes verified')
    elif a.action=='install':
        source=Path(a.pack).resolve();m,_=verify(source);dest=DEST/m['id']
        if dest.exists():raise SystemExit('Pack already installed; remove it first')
        tmp=dest.with_name(dest.name+'.installing');shutil.copytree(source,tmp);tmp.rename(dest)
    elif a.action=='remove':
        target=(DEST/a.pack).resolve()
        if target.parent!=DEST.resolve() or not target.is_dir():raise SystemExit('Unknown regional pack')
        verify(target)
        shutil.rmtree(target)
