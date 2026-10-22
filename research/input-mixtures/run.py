"""Frozen offline mixed-input prediction experiment; Python standard library only."""
import hashlib
import json
import math
import pathlib
import shutil
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[2]
OUT = ROOT / "test-results/input-mixtures-v1"
NAMES = ["2048-1", "tetris-1", "2048-2", "tetris-2"]
VARIANTS = ["interior", "early_early", "late_late", "outer"]


def read(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write(path, value):
    with path.open("x", encoding="utf-8") as stream:
        json.dump(value, stream, indent=2, allow_nan=False)


def rank(rows):
    work = [r[:] for r in rows]
    pivot = 0
    for col in range(len(work[0])):
        index = max(range(pivot, len(work)), key=lambda i: abs(work[i][col]), default=None)
        if index is None or abs(work[index][col]) < 1e-10:
            continue
        work[index], work[pivot] = work[pivot], work[index]
        divisor = work[pivot][col]
        work[pivot] = [v / divisor for v in work[pivot]]
        for i in range(pivot + 1, len(work)):
            factor = work[i][col]
            work[i] = [a - factor * b for a, b in zip(work[i], work[pivot])]
        pivot += 1
    return pivot


def solve(matrix, rhs):
    work = [row[:] + [value] for row, value in zip(matrix, rhs)]
    for col in range(len(rhs)):
        index = max(range(col, len(rhs)), key=lambda i: abs(work[i][col]))
        assert abs(work[index][col]) > 1e-12, "Singular regularized design"
        work[index], work[col] = work[col], work[index]
        divisor = work[col][col]
        work[col] = [v / divisor for v in work[col]]
        for i in range(len(rhs)):
            if i != col:
                factor = work[i][col]
                work[i] = [a - factor * b for a, b in zip(work[i], work[col])]
    return [r[-1] for r in work]


def fit(rows, targets):
    n, columns = len(rows), len(rows[0])
    means = [sum(r[j] for r in rows) / n for j in range(columns)]
    scales = [math.sqrt(sum((r[j] - means[j]) ** 2 for r in rows) / n) for j in range(columns)]
    standardized = [[(r[j] - means[j]) / scales[j] if scales[j] > 1e-12 else 0.0
                     for j in range(columns)] for r in rows]
    intercept = sum(targets) / n
    matrix = [[sum(r[i] * r[j] for r in standardized) / n + (1.0 if i == j else 0.0)
               for j in range(columns)] for i in range(columns)]
    rhs = [sum(r[j] * (y - intercept) for r, y in zip(standardized, targets)) / n
           for j in range(columns)]
    return {"penalty": 1.0, "means": means, "scales": scales, "intercept": intercept,
            "coefficients": solve(matrix, rhs), "centered_design_rank": rank(standardized),
            "columns": columns}


def predict(model, rows):
    return [model["intercept"] + sum(c * (v - m) / s if s > 1e-12 else 0.0
            for c, v, m, s in zip(model["coefficients"], row, model["means"], model["scales"]))
            for row in rows]


def mse(predictions, targets):
    return sum((a - b) ** 2 for a, b in zip(predictions, targets)) / len(targets)


def synthetic_check(collinear):
    rows = [[1.0, a / 5, (a if collinear else b) / 5] for a in range(6) for b in range(6)]
    targets = [2 * r[1] - 3 * r[2] for r in rows]
    model = fit(rows, targets)
    baseline = fit([[r[0]] for r in rows], targets)
    error = mse(predict(model, rows[::-1]), targets[::-1])
    base_error = mse(predict(baseline, [[r[0]] for r in rows[::-1]]), targets[::-1])
    gain = 1 - error / base_error
    expected_rank = 1 if collinear else 2
    assert model["centered_design_rank"] == expected_rank
    assert gain > 0.7
    return {"collinear": collinear, "expected_centered_rank": expected_rank,
            "model": model, "relative_mse_reduction": gain,
            "limitation": "Synthetic arithmetic check, not independent game evidence."}


def dataset(case, variant, keys=None):
    if case.get("error"):
        raise ValueError(case["error"])
    selected = []
    for row in case["rows"]:
        if row["error"]:
            raise ValueError(row["error"])
        value = next(v for v in row["variants"] if v["variant"] == variant)
        if value["error"] or row["measurement"]["error"]:
            raise ValueError(value["error"] or row["measurement"]["error"])
        selected.append((row, value))
    if len(selected) != 15:
        raise ValueError("Incomplete evaluation run")
    seen = sorted({k for _, v in selected for k in v["key_samples"]}, key=int)
    keys = seen if keys is None else keys
    rows = [[v["duration_us"] / 1e6] + [v["key_samples"].get(k, 0) / v["polls"] for k in keys]
            for _, v in selected]
    targets = [row["measurement"]["difference"] for row, _ in selected]
    if not all(isinstance(y, (int, float)) and math.isfinite(y) and 0 <= y <= 255 for y in targets):
        raise ValueError("Invalid image measurement")
    return rows, targets, keys, sorted(set(seen) - set(keys), key=int)


def evaluate(training, testing, variant):
    try:
        train, train_y, keys, _ = dataset(training, variant)
        test, test_y, _, unseen = dataset(testing, variant, keys)
        mixture = fit(train, train_y)
        baseline = fit([[r[0]] for r in train], train_y)
        predicted = predict(mixture, test)
        base_predicted = predict(baseline, [[r[0]] for r in test])
        error, base_error = mse(predicted, test_y), mse(base_predicted, test_y)
        controls = []
        for shift in range(1, 15):
            shuffled = [[row[0]] + test[(i + shift) % 15][1:] for i, row in enumerate(test)]
            controls.append({"offset": shift, "mse": mse(predict(mixture, shuffled), test_y)})
        best_control = min(r["mse"] for r in controls)
        promising = not unseen and error < base_error and error < best_control
        return {"training": training["name"], "testing": testing["name"], "variant": variant,
                "keys": keys, "unseen_test_keys": unseen, "training_features": train,
                "test_features": test, "training_targets": train_y, "test_targets": test_y,
                "mixture": mixture, "baseline": baseline, "predictions": predicted,
                "baseline_predictions": base_predicted, "mse": error, "baseline_mse": base_error,
                "relative_mse_reduction": 1 - error / base_error if base_error > 0 else None,
                "alignment_controls": controls, "controls_at_least_as_good": sum(r["mse"] <= error for r in controls),
                "passes_declared_comparison": promising, "error": None}
    except (ValueError, AssertionError, ZeroDivisionError) as error:
        return {"training": training["name"], "testing": testing["name"], "variant": variant,
                "error": str(error), "passes_declared_comparison": False}


OUT.mkdir()
binary = OUT / "input-mixtures.exe"
shutil.copyfile(ROOT / "target/debug/examples/input-mixtures.exe", binary)
paths = [pathlib.Path(__file__), pathlib.Path(__file__).with_name("PROTOCOL.md"), binary]
tracked = subprocess.check_output(["git", "ls-files", "crates", "Cargo.toml", "Cargo.lock"], cwd=ROOT, text=True).splitlines()
paths += [ROOT / p for p in tracked if p.endswith((".rs", ".toml")) or p == "Cargo.lock"]
paths.append(ROOT / "crates/reversebolt-core/examples/input-mixtures.rs")
prior_path = ROOT / "research/browser-activity-replay/results.json"
prior = {c["name"]: c for c in read(prior_path)["cases"]}
paths.append(prior_path)
original_path = ROOT / "test-results/browser-activity-replay-v1/frozen.json"
original = read(original_path)
paths.append(original_path)
for name in NAMES:
    status_path = ROOT / f"test-results/independent-games-v1/{name}-attempt-01/final-status.json"
    assert sha(status_path) == original[str(status_path)]
    frames = read(status_path)["state"]["live"]["frames"]
    assert len(frames) == 16
    paths.append(status_path)
    for frame in frames:
        matches = list((status_path.parent / "engine/captures").rglob(f"frame-{frame['number']:03}.png"))
        assert len(matches) == 1 and sha(matches[0]) == frame["sha256"] == original[str(matches[0])]
        paths.append(matches[0])
    cached = ROOT / f"test-results/browser-activity-replay-v1/{name}/result.json"
    assert sha(cached) == prior[name]["result_sha256"]
    paths.append(cached)
frozen = {str(p): sha(p) for p in sorted(set(paths))}
write(OUT / "frozen.json", frozen)
synthetic = [synthetic_check(False), synthetic_check(True)]
write(OUT / "synthetic.json", synthetic)
process = subprocess.run([str(binary), str(ROOT), str(OUT / "features.json")], cwd=ROOT,
                         text=True, capture_output=True, timeout=60)
write(OUT / "process.json", {"exit_code": process.returncode, "stdout": process.stdout, "stderr": process.stderr})
assert process.returncode == 0, process.stderr
features = read(OUT / "features.json")
cases = {c["name"]: c for c in features["cases"]}
evaluations = [evaluate(cases[a], cases[b], variant)
               for a, b in [("2048-1", "2048-2"), ("2048-2", "2048-1"),
                            ("tetris-1", "tetris-2"), ("tetris-2", "tetris-1")]
               for variant in VARIANTS]
write(OUT / "evaluations.json", evaluations)
changed = [p for p, digest in frozen.items() if sha(pathlib.Path(p)) != digest]
summary = {"policy": features["policy"], "synthetic_checks": synthetic,
           "evaluations": [{k: e[k] for k in ["training", "testing", "variant", "error", "passes_declared_comparison"]}
                           | {k: e[k] for k in ["mse", "baseline_mse", "relative_mse_reduction", "controls_at_least_as_good"] if k in e}
                           for e in evaluations],
           "passes_all_directions_and_timing_choices": all(e["passes_declared_comparison"] for e in evaluations),
           "frozen_files": len(frozen), "changed": changed,
           "features_sha256": sha(OUT / "features.json"), "evaluations_sha256": sha(OUT / "evaluations.json"),
           "production_changed": False, "new_model_calls": 0, "new_captures": 0,
           "native_target_access": False, "inputs_sent": 0}
write(OUT / "summary.json", summary)
print(json.dumps(summary))
assert not changed
