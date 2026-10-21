"""Post-run artifact audit, added after evaluation. Does not run/retry a probe."""
from collections import Counter
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
RAW = ROOT / "test-results/runtime-probe-v1"
def read(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))
def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

freeze = read(HERE / "frozen-v1.json")
for item in freeze["files"]:
    assert digest(ROOT / item["path"]) == item["sha256"], item["path"]
fixture = read(RAW / "mock/cases.json")
assert fixture["host_exit_code"] == 0 and len(fixture["cases"]) == 6
assert all((c["exit_code"] == 0) == c["expected_success"] for c in fixture["cases"])
identity = read(RAW / "game-identity.json")
after = read(RAW / "game-after.json")
assert identity["pid"] == after["pid"] and identity["created"] == after["created"]
assert after["responding"] and after["window"] and not after["helper_present"]
summary = read(RAW / "blue-prince/summary.json")
audits = []
for index in range(2):
    path = RAW / f"blue-prince/probe-{index}.json"
    report = read(path)
    assert digest(path) == summary["reports"][index]["sha256"]
    assert report["pid"] == identity["pid"] and report["created"] == identity["created"]
    assert report["session"] == 2 and report["omissions"] == []
    assert len(report["classes"]) == 16955 and len(report["fields"]) == 87225
    name_errors = sum("text" not in item["name"] for group in ["images","classes","fields"] for item in report[group])
    assert name_errors == 0
    live = [f for f in report["fields"] if f["status"] == "live_static_read_attempt"]
    assert len(live) == 989
    for field in live:
        assert len(field["samples"]) == 2 and int(field["address"],16) > 0
        for sample in field["samples"]:
            assert "value" in sample and sample["before_us"] <= sample["after_us"]
            assert len(sample["bytes"]) in (1,2,4,8)
    receipt = [json.loads(line) for line in (RAW / f"blue-prince/probe-{index}.stdout.jsonl").read_text().splitlines()]
    assert receipt[0]["helper_sha256"] == summary["helper_sha256"]
    assert receipt[-1]["helper_absent"] and receipt[-1]["inspection_exit"] == 0
    game_image = next(i for i in summary["reports"][index]["images"] if i["image"]["text"] == "Assembly-CSharp.dll")
    assert game_image["successful_static_reads"] == 38
    audits.append({"index":index,"raw_sha256":digest(path),"images":len(report["images"]),"classes":len(report["classes"]),"fields":len(report["fields"]),"successful_static_fields":len(live),"intra_pair_changes":sum(f["samples"][0]["bytes"]!=f["samples"][1]["bytes"] for f in live),"game_main_image":game_image,"name_errors":name_errors,"reasons":dict(Counter(f["reason"] for f in report["fields"])),"enumeration_ms":report["enumeration_elapsed_ms"],"total_loader_ms":receipt[-1]["elapsed_ms"],"helper_unloaded":True})
desktop = read(RAW / "desktop.json")
cleanup = read(RAW / "cleanup.json")
assert desktop["ok"] and not desktop["changed_this_turn"]
assert cleanup["remaining_owned_processes"] == []
result = {"audit_passed":True,"frozen_files_verified":len(freeze["files"]),"helper_sha256":summary["helper_sha256"],"runtime_sha256":identity["runtime_sha256"],"mock_cases":fixture["cases"],"game_identity":identity,"reports":audits,"two_pass":summary["two_pass"],"screenshots":0,"gameplay_inputs":0,"game_field_write_calls":0,"claim":"runtime metadata and static scalar feasibility only; no gameplay authority","progress_percent":31,"desktop":desktop,"cleanup":cleanup,"raw_artifacts":[{"path":str(p.relative_to(ROOT)).replace("\\","/"),"sha256":digest(p),"bytes":p.stat().st_size} for p in sorted(RAW.rglob("*")) if p.is_file()]}
(HERE / "results.json").write_text(json.dumps(result,indent=2))
print(json.dumps({"audit_passed":True,"frozen_files_verified":len(freeze["files"]),"raw_artifacts":len(result["raw_artifacts"]),"two_pass":summary["two_pass"]}))
