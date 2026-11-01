"""Run the existing headless mission study while preserving earlier study output."""
from pathlib import Path
import os,subprocess,json
root=Path(__file__).resolve().parents[2];out=root/'tests/sam-review';out.mkdir(parents=True,exist_ok=True)
report=root/'tests/naval-review/study.json';previous=report.read_bytes()if report.exists()else None
try:
 with (out/'mission-study.log').open('w')as log:
  subprocess.run([str(root/'Nord Flight.app/Contents/MacOS/gripen-flight'),'--study-naval'],cwd=root,env=dict(os.environ,NORD_NAVAL_STUDY_SECONDS='300',NORD_NAVAL_STUDY_PRESET='1'),stdout=log,stderr=subprocess.STDOUT,check=True,timeout=600)
 data=json.loads(report.read_text());(out/'mission-study.json').write_text(json.dumps(data,indent=2)+'\n');print(json.dumps(data),flush=True)
finally:
 if previous is not None:report.write_bytes(previous)
 else:report.unlink(missing_ok=True)
