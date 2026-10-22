"""Read saved JSON only: do not run the original game/capture harnesses."""
import hashlib
import json
from pathlib import Path

root = Path(__file__).resolve().parents[2]
corpus = root / "test-results/independent-games-v1"
runs = []
for name in ["2048-1", "2048-2", "tetris-1", "tetris-2"]:
    directory = corpus / f"{name}-attempt-01"
    status_path = directory / "final-status.json"
    status_bytes = status_path.read_bytes()
    live = json.loads(status_bytes.decode("utf-8-sig"))["state"]["live"]
    captures = directory / "engine/captures"
    files = sorted(captures.rglob("frame-*.json"))
    manifests = []
    receipt_count = 0
    receipt_bytes = 0
    memory_clock_intervals = 0
    for path in files:
        raw = path.read_bytes()
        frame = json.loads(raw.decode("utf-8-sig"))
        reads = frame["memory_reads"]
        # This historical schema stores counts and timings, not the read payload.
        assert all(set(r) <= {"address", "requested", "retained", "begin_ms", "end_ms", "disposition", "capture_interval"} for r in reads)
        receipt_count += len(reads)
        receipt_bytes += sum(r["retained"] for r in reads)
        memory_clock_intervals += sum(r.get("capture_interval") is not None for r in reads)
        manifests.append({"path": path.relative_to(root).as_posix(), "sha256": hashlib.sha256(raw).hexdigest()})
    assert len(files) == live["sample_count"]
    runs.append({
        "run": name,
        "status_sha256": hashlib.sha256(status_bytes).hexdigest(),
        "captures": len(files),
        "read_receipts": receipt_count,
        "bytes_counted_as_retained_in_receipts": receipt_bytes,
        "bytes_counted_as_retained_in_status": live["retained_bytes"],
        "investigation_clock_read_intervals": memory_clock_intervals,
        "broad_memory_export_paths": [p.relative_to(root).as_posix() for p in captures.rglob("bytes.bin")],
        "candidate_witness_archive_paths": [p.relative_to(root).as_posix() for p in captures.rglob("memory-batch.json")],
        "frame_receipts": manifests,
    })
print(json.dumps({
    "method": "saved_capture_schema_audit_v1",
    "scope": "Original exposed recordings; JSON metadata only. Browser-profile files, game oracles and target source are not read.",
    "finding": "Known capture schemas retain read receipts, not broad memory bytes. A retained-byte count cannot reconstruct its payload. Candidate-only witness archives would also be selected evidence, not a blind memory corpus.",
    "native_process_access": False,
    "runs": runs,
}, indent=2))
