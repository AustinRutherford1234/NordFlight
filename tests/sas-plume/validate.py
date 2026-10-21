"""Two native presentation checks; unit tests cover state transitions and motion."""
from pathlib import Path
import shutil
import sys

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT / 'tests/optimization3'))
from run import isolated, run

binary = ROOT / 'Nord Flight.app/Contents/MacOS/gripen-flight'
with isolated():
    for label, env in [
        ('sas-indicator', {}),
        ('f22-exhaust', {'NORD_AIRCRAFT': 'f22', 'NORD_FLEET_CAPTURE': 'exhaust-wet'}),
    ]:
        source = ROOT / 'tests/visual-dry.png'
        before = source.stat().st_mtime_ns if source.exists() else 0
        run(binary, ['--visual-check', 'dry'], OUT, label, env)
        assert source.stat().st_mtime_ns > before
        shutil.copy2(source, OUT / (label + '.png'))
