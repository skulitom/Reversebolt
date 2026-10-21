"""Two frozen probes of one explicitly identified, already-launched Anode game."""
from collections import Counter
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
BIN = ROOT / "target/runtime-probe/debug"
pid, created, session, directory = sys.argv[1:]
out = Path(directory).resolve()
out.mkdir(parents=True, exist_ok=False)
helper = BIN / "reversebolt_runtime_probe.dll"
digest = hashlib.sha256(helper.read_bytes()).hexdigest()
summaries = []
reports = []
for index in range(2):
    report_path = out / f"probe-{index}.json"
    args = [str(BIN / "probe-loader.exe"),pid,created,session,str(helper),digest,str(report_path)]
    result = subprocess.run(args,capture_output=True,text=True,timeout=130)
    (out / f"probe-{index}.stdout.jsonl").write_text(result.stdout)
    (out / f"probe-{index}.stderr.jsonl").write_text(result.stderr)
    assert result.returncode == 0, result.stderr
    receipt = json.loads(result.stdout.splitlines()[-1])
    assert receipt["helper_absent"] and receipt["inspection_exit"] == 0
    report = json.loads(report_path.read_text(encoding="utf-8"))
    classes = {c["id"]:c for c in report["classes"]}
    per_image = []
    for image in report["images"]:
        fields = [f for f in report["fields"] if classes[f["class"]]["image"] == image["id"]]
        live = [f for f in fields if f["status"] == "live_static_read_attempt"]
        per_image.append({"image":image["name"],"reported_classes":image["reported_class_count"],"fields":len(fields),"live_static_attempts":len(live),"successful_static_reads":sum(all("value" in s for s in f["samples"]) for f in live)})
    summaries.append({"index":index,"sha256":hashlib.sha256(report_path.read_bytes()).hexdigest(),"bytes":report_path.stat().st_size,"enumeration_ms":report["enumeration_elapsed_ms"],"classes":len(classes),"fields":len(report["fields"]),"reasons":dict(Counter(f["reason"] for f in report["fields"])),"omissions":report["omissions"],"images":per_image,"loader":receipt})
    reports.append(report)
    time.sleep(0.25)
def values(report):
    classes = {c["id"]:c for c in report["classes"]}
    images = {i["id"]:i for i in report["images"]}
    data = {}
    for f in report["fields"]:
        if f["status"] != "live_static_read_attempt" or not all("value" in s for s in f["samples"]):
            continue
        c = classes[f["class"]]
        key = (str(images[c["image"]]["name"]),str(c["namespace"]),str(c["name"]),str(f["name"]),f["offset"],f["type_code"])
        assert key not in data, "ambiguous metadata identity"
        data[key] = f["samples"][-1]["value"]
    return data
before, after = map(values,reports)
common = before.keys() & after.keys()
summary = {"pid":int(pid),"created":created,"session":int(session),"helper_sha256":digest,"reports":summaries,"two_pass":{"common_successful_static_fields":len(common),"equal":sum(before[k]==after[k] for k in common),"different":sum(before[k]!=after[k] for k in common)},"captures":0,"gameplay_inputs":0,"game_field_writes":0,"edit_authority":False}
(out / "summary.json").write_text(json.dumps(summary,indent=2))
print(json.dumps({"completed":True,"classes":[s["classes"] for s in summaries],"fields":[s["fields"] for s in summaries],"two_pass":summary["two_pass"],"summary_path":str(out / "summary.json")}))
