"""Unchanged product reader on all retained independent browser-game runs."""
import collections
import hashlib
import json
import pathlib
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[2]
OUT = ROOT / "test-results/browser-activity-replay-v1"
EXE = ROOT / "target/debug/examples/runtime-appearance-replay.exe"


def read(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write(path, value):
    with path.open("x", encoding="utf-8") as stream:
        json.dump(value, stream, indent=2)


OUT.mkdir()
prior_frozen = read(ROOT / "test-results/regional-activity-entry-v1/frozen.json")
source_paths = subprocess.check_output(
    ["git", "ls-files", "crates", "Cargo.toml", "Cargo.lock"], cwd=ROOT, text=True
).splitlines()
paths = [ROOT / p for p in source_paths if p.endswith((".rs", ".toml")) or p == "Cargo.lock"]
paths.append(EXE)
for path in paths:
    if prior_frozen[str(path)] != sha(path):
        raise SystemExit(f"Product differs from the tested prior build: {path}")
paths += [pathlib.Path(__file__), pathlib.Path(__file__).with_name("PROTOCOL.md")]
cases = []
for name in ["2048-1", "tetris-1", "2048-2", "tetris-2"]:
    folder = ROOT / f"test-results/independent-games-v1/{name}-attempt-01"
    status_path = folder / "final-status.json"
    live = read(status_path)["state"]["live"]
    frames = live["frames"]
    if len(frames) != 16:
        raise SystemExit(f"Unexpected retained frame count: {name}")
    manifest = {"name": name, "investigation_id": live["investigation_id"],
                "batch": live["coverage"]["batch"], "frames": []}
    for frame in frames:
        matches = list((folder / "engine/captures").rglob(f"frame-{frame['number']:03}.png"))
        if len(matches) != 1 or sha(matches[0]) != frame["sha256"]:
            raise SystemExit(f"Missing, ambiguous or changed capture: {name}/{frame['number']}")
        paths.append(matches[0])
        manifest["frames"].append({"file": str(matches[0]), "reference": frame})
    manifest_path, observations_path = OUT / f"{name}.json", OUT / f"{name}-observations.json"
    write(manifest_path, manifest)
    write(observations_path, {"visual_tracks": live["visual_tracks"]})
    paths += [status_path, manifest_path, observations_path]
    cases.append((name, manifest_path, observations_path))
frozen = {str(path): sha(path) for path in sorted(set(paths))}
write(OUT / "frozen.json", frozen)
results = []
for name, manifest, observations in cases:
    output = OUT / name
    process = subprocess.run(
        [str(EXE), str(manifest), "cached-only", str(output), "--observations", str(observations)],
        cwd=ROOT, text=True, capture_output=True, timeout=180,
    )
    write(OUT / f"{name}-process.json", {"exit_code": process.returncode,
                                      "stdout": process.stdout, "stderr": process.stderr})
    if process.returncode:
        results.append({"name": name, "error": process.stderr})
        print(json.dumps(results[-1]), flush=True)
        continue
    result = read(output / "result.json")
    measured = result["evidence"]
    evidence = measured["evidence"]
    intervals = evidence["intervals"]
    exposures = collections.Counter(i["exposure"]["kind"] for i in intervals)
    reasons = collections.Counter(i["exposure"].get("value") for i in intervals if i["exposure"]["kind"] == "unknown")
    durations = [i["elapsed_us"] for i in intervals]
    frames = read(manifest)["frames"]
    spans = [b["reference"]["capture_interval"]["end_us"] - a["reference"]["capture_interval"]["begin_us"]
             for a, b in zip(frames, frames[1:])]
    search = measured["automatic_region_motion"].get("regional_activity_search")
    record = {
        "name": name, "frames": len(frames), "exposures": dict(exposures), "unknown_reasons": dict(reasons),
        "interval_spans_us": spans, "elapsed_bounds_us": durations,
        "image_measurement_errors": [i["measurement"]["error"] for i in intervals if i["measurement"]["error"]],
        "difference_range": [min(i["measurement"]["difference"] for i in intervals if i["measurement"]["difference"] is not None),
                             max(i["measurement"]["difference"] for i in intervals if i["measurement"]["difference"] is not None)],
        "bounded_key_episodes": sum(e["bounded_by_quiet"] for e in evidence["episodes"]),
        "activity_source": measured["activity_source"],
        "regional_attempts": len(search["attempts"]) if search else 0,
        "base_evidence_unchanged": evidence == result["without_observations"]["evidence"],
        "new_model_calls": result["publication"]["new_model_calls"],
        "visual_track_count": len(result["visual_tracks"]),
        "result_sha256": sha(output / "result.json"),
    }
    results.append(record)
    print(json.dumps(record), flush=True)
changed = [path for path, digest in frozen.items() if sha(pathlib.Path(path)) != digest]
report = {"product_commit": "24f5e89", "cases": results, "frozen_file_count": len(frozen), "changed": changed}
write(OUT / "results.json", report)
if changed or any("error" in case or not case["base_evidence_unchanged"] or case["new_model_calls"] != 0 for case in results):
    raise SystemExit("Replay or integrity failure; inspect retained evidence.")
