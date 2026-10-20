"""Repeat the affected paused comparison after OS background compilation ends."""
import json
from pathlib import Path
import signal
import subprocess
import sys
import threading
import time
import focused

OUT=Path(__file__).resolve().parent
records=[];stop=threading.Event();started=time.monotonic()
def sample():
    while not stop.is_set():
        rows=[]
        text=subprocess.check_output(['ps','-axo','%cpu=,comm='],text=True)
        for line in text.splitlines():
            fields=line.strip().split(None,1)
            if len(fields)==2 and fields[1].endswith(('/ANECompilerService','/rustc','/cargo','/WindowServer')):
                rows.append({'cpu_percent':float(fields[0]),'process':fields[1]})
        records.append({'wall_s':time.monotonic()-started,'processes':rows})
        stop.wait(10)
signal.signal(signal.SIGTERM,lambda *_: (_ for _ in ()).throw(KeyboardInterrupt()))
thread=threading.Thread(target=sample,daemon=True);thread.start()
sys.argv=[sys.argv[0],str(OUT/'candidate/gripen-flight-release'),'release-paused-quiet','matrix','--scenes','cockpit-paused']
try:
    focused.main()
finally:
    stop.set();thread.join(timeout=5)
    (OUT/'quiet-paused-host.json').write_text(json.dumps(records,indent=2)+'\n')
