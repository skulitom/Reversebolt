"""Read-only audit of the frozen two-case integration artifacts."""
from pathlib import Path
import hashlib
import json

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
RAW = ROOT / "test-results/location-probes-v1"
read = lambda p: json.loads(p.read_text(encoding="utf-8-sig"))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
config = read(HERE / "protocol-v1.json")
for name, digest in config["method_sha256"].items():
    assert sha(HERE / name) == digest, name
for name, digest in config["product_source_sha256"].items():
    assert sha(HERE / "source" / name) == digest, name
for name, digest in config["binaries"].items():
    assert sha(Path(config["binary_directory"]) / name) == digest, name

summary = []
for slot in (0, 1):
    directory = RAW / f"slot-{slot}"
    report = read(directory / "report.json")
    assert not report["errors"], report
    assert report["protocol_sha256"] == sha(HERE / "protocol-v1.json")
    prepared = read(directory / "prepared.json")
    if not prepared["ok"] or prepared["data"]["phase"] != "prepared":
        summary.append({"slot": slot, "outcome": report["outcome"]})
        continue
    change = prepared["data"]
    assert change["probe"]["selected"] == 0
    field = report["selected_field"]
    address = report["prepared_oracle"]["fields"][field]
    patch = change["patch"]
    assert address <= patch["address"]
    assert patch["address"] + len(patch["after"]) <= address + 4
    assert report["prepared_oracle"]["values"] == [50, 50]
    assert report["applied_oracle"]["values"][field] == 53
    assert report["applied_oracle"]["values"][1-field] == 50
    expected = 53 if slot == field else 50
    checked = read(directory / "checked.json")
    assert checked["observations"][-1]["value"] == expected
    assert len(change["probe"]["alternatives"]) == len(read(directory / "fresh.json")["prediction_check"]["candidates"])
    fits = checked["observations"][-1]["probe_comparisons"]
    assert len(fits) == len(change["probe"]["alternatives"])
    for comparison in fits:
        prediction = comparison["expected"]
        wanted = "unavailable" if prediction is None else "agreed" if abs(prediction-expected) <= .55 else "differed"
        assert comparison["fit"] == wanted
    restored = read(directory / "checked-restore.json")
    assert restored["phase"] == "restored"
    assert restored["observations"][-1]["value"] == 50
    assert restored["observations"][-1]["probe_comparisons"] == []
    assert report["restored_oracle"]["values"] == [50, 50]
    assert read(directory / "final-status.json")["state"]["live"]["prediction_check"]["invalidated"]
    summary.append({"slot": slot, "selected_field": field,
                    "ordinary": read(directory / "ordinary.json")["data"]["phase"],
                    "interpretations": len(fits), "observed": expected,
                    "fits": [x["fit"] for x in fits], "outcome": report["outcome"]})
print(json.dumps(summary, indent=2))
