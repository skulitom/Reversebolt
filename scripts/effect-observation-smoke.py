"""Automated native effect-observation matrix. Run inside Anode's seat.

The external fixture owns private truth. No address, field layout, effect mode
or control file is passed to the discovery engine. This is synthetic integration
evidence, not independent-game validation.
"""
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import uuid

ROOT=Path(__file__).resolve().parents[1]
BIN=Path(os.environ.get("REVERSEBOLT_BIN_DIR",ROOT/"target/release"))
OUTPUT=ROOT/"test-results"/("effect-observations-"+uuid.uuid4().hex[:8])
OUTPUT.mkdir(parents=True)
report={"scope":__doc__,"cases":[]}
failed=False
for mode in ("coupled","drift","hidden"):
    folder=OUTPUT/mode
    folder.mkdir()
    data=folder/"session"
    control=folder/"control.txt"
    audit=folder/"private-audit.json"
    fixture=engine=None
    case={"mode":mode}
    try:
        fixture=subprocess.Popen(["powershell.exe","-NoProfile","-WindowStyle","Hidden","-ExecutionPolicy","Bypass","-File",str(ROOT/"scripts/effect-fixture.ps1"),"-ControlFile",str(control),"-Mode",mode,"-AuditFile",str(audit)],creationflags=0x08000000)
        engine=subprocess.Popen([str(BIN/"Reversebolt.exe"),"--headless","--data-dir",str(data)],creationflags=0x08000000)
        deadline=time.monotonic()+20
        while time.monotonic()<deadline:
            if engine.poll() is not None or fixture.poll() is not None:
                raise RuntimeError("Owned fixture or engine exited at startup.")
            if (data/"endpoint.json").exists():
                time.sleep(1.5) # Let the owned WinForms compilation/window finish.
                break
            time.sleep(.05)
        argument="--other-unchanged" if mode=="hidden" else "--other-changed"
        result=subprocess.run([sys.executable,str(ROOT/"scripts/trial-edit-smoke.py"),str(data),str(control),str(fixture.pid),str(BIN),argument],capture_output=True,text=True,encoding="utf-8",timeout=120,creationflags=0x08000000)
        case["driver_exit"]=result.returncode
        (folder/"driver-output.txt").write_text(result.stdout+result.stderr,encoding="utf-8")
        trial=json.loads((data/"trial-result.json").read_text(encoding="utf-8"))
        case["outcome"]=trial.get("outcome")
        case["error"]=trial.get("error")
        assert result.returncode==0 and case["outcome"]=="write and restore observed through MCP",case
        case["comparison_after_edit"]=trial["checked"]["data"]["observations"][-1]["comparison"]
        case["comparison_after_restore"]=trial["checked_restore"]["data"]["observations"][-1]["comparison"]
        case["images"]=trial["image_evidence"]["hashes_verified"]
    except Exception as error:
        case["failure"]=str(error)
        failed=True
    finally:
        if fixture and fixture.poll() is None:
            control.write_text("close",encoding="ascii")
            try: fixture.wait(timeout=8)
            except subprocess.TimeoutExpired:
                fixture.terminate();fixture.wait(timeout=5)
                case["cleanup_warning"]="Owned fixture did not acknowledge close; audit may be missing."
                failed=True
        if engine and engine.poll() is None:
            engine.terminate();engine.wait(timeout=8)
        if audit.exists():
            case["private_audit"]=json.loads(audit.read_text(encoding="utf-8"))
            if case.get("outcome")=="write and restore observed through MCP":
                assert case["private_audit"]["quantity"]==50
                assert case["private_audit"]["writes"]==2
                if mode=="hidden": assert case["private_audit"]["hidden"]==2
        else:
            case["cleanup_warning"]="Private audit missing."
            failed=True
        report["cases"].append(case)
        (OUTPUT/"report.json").write_text(json.dumps(report,indent=2)+"\n",encoding="utf-8")
    print(mode,case.get("outcome",case.get("failure")),flush=True)
print("Evidence:",OUTPUT/"report.json",flush=True)
sys.exit(1 if failed else 0)
