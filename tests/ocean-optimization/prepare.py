"""Prepare equivalent app bundles before an approved native comparison."""
import argparse,shutil,hashlib,json
from pathlib import Path
root=Path(__file__).resolve().parents[2];out=root/'tests/ocean-optimization'
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--baseline',required=True,type=Path);args=p.parse_args()
paths={'A':args.baseline.resolve(),'B':root/'Nord Flight.app/Contents/MacOS/gripen-flight'}
for path in paths.values():assert path.is_file(),f'Missing executable: {path}'
record={}
for variant,source in paths.items():
 bundle=out/variant/'Nord Flight.app/Contents';(bundle/'MacOS').mkdir(parents=True,exist_ok=True);(bundle/'Resources').mkdir(exist_ok=True)
 shutil.copy2(source,bundle/'MacOS/gripen-flight');shutil.copy2(root/'packaging/Info.plist',bundle/'Info.plist');shutil.copy2(root/'packaging/NordFlight.icns',bundle/'Resources/NordFlight.icns')
 link=bundle/'Resources/assets'
 if not link.exists():link.symlink_to(root/'assets',target_is_directory=True)
 assert link.resolve()==(root/'assets').resolve(),'Comparison must share the exact asset files'
 record[variant]={'executable_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'metadata_sha256':hashlib.sha256((bundle/'Info.plist').read_bytes()).hexdigest(),'assets':str(link.resolve())}
(out/'prepared.json').write_text(json.dumps(record,indent=2)+'\n');print(json.dumps(record,indent=2))
