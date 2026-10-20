"""Read images or submit separately authored agent interpretations to an isolated engine.
This helper is not a visual model and never generates a value or fills a gap.
Usage: python prediction-evidence.py read <data-dir> <new-output-dir>
       python prediction-evidence.py submit <data-dir> <annotations.json> <new-output.json>
Annotations: [{"frame": 1, "value": 42, "explanation": "What the inspected image shows"}, ...]
Use null for an uncertain value. Inspect exported images before authoring annotations.
"""
import base64
import hashlib
import json
from pathlib import Path
import sys
import urllib.request

operation, data = sys.argv[1], Path(sys.argv[2])
endpoint = json.loads((data / "endpoint.json").read_text(encoding="utf-8-sig"))

def call(action, args=None):
    request = urllib.request.Request(endpoint["url"]+"/command",
        data=json.dumps({"action":action,"args":args or {}}).encode(),
        headers={"Content-Type":"application/json","Authorization":"Bearer "+endpoint["token"]})
    with urllib.request.urlopen(request,timeout=120) as response:
        result=json.load(response)
    assert result["ok"], result.get("error")
    return result

live=call("status")["state"]["live"]
if operation == "read":
    output=Path(sys.argv[3])
    output.mkdir(parents=True,exist_ok=False)
    manifest=[]
    for frame in live["frames"]:
        result=call("read_capture",{"investigation_id":live["investigation_id"],"frame_id":frame["id"]})
        pixels=base64.b64decode(result["data"]["png_base64"],validate=True)
        assert hashlib.sha256(pixels).hexdigest()==frame["sha256"]
        image=output/f"frame-{frame['number']:03}.png"
        image.write_bytes(pixels)
        manifest.append({"frame":frame,"image":str(image.resolve())})
    (output/"manifest.json").write_text(json.dumps(manifest,indent=2),encoding="utf-8")
    print(json.dumps({"images":len(manifest),"output":str(output)}))
elif operation == "submit":
    annotations=json.loads(Path(sys.argv[3]).read_text(encoding="utf-8-sig"))
    output=Path(sys.argv[4])
    assert not output.exists(), "Preserve prior receipts."
    receipts=[]
    for annotation in annotations:
        frame=next(f for f in live["frames"] if f["number"]==annotation["frame"])
        value=annotation["value"]
        result=call("submit_observation",{"investigation_id":live["investigation_id"],
            "frame_id":frame["id"],"image_sha256":frame["sha256"],"observer":"Codex image inspection",
            "key":"inspected-visible-counter","label":"Visible counter (agent interpretation)","unit":"count",
            "value":None if value is None else {"kind":"number","value":value},
            "quality":"uncertain" if value is None else "provisional","bounds":None,
            "explanation":annotation["explanation"]})
        receipts.append({"frame":frame["number"],"ok":result["ok"],"data":result["data"]})
    output.write_text(json.dumps({"receipts":receipts,"state":result["state"]},indent=2),encoding="utf-8")
    print(json.dumps({"submitted":len(receipts),"output":str(output)}))
else:
    raise ValueError("Use read or submit")
