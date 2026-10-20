"""Compare RTen and ONNX Runtime on identical real-image model tensors.

This checks operators and CTC output, NOT a Rust image preprocessing port.
export <new-dir> <prediction.json>... saves first-frame model calls from each
specified image-only run. check <dir> <new-report.json> compares Rust outputs.
"""
import ctypes
import hashlib
import json
import os
from pathlib import Path
import sys

if sys.platform == "win32":
    runtime_handles = [ctypes.WinDLL(str(Path(os.environ["SystemRoot"]) / "System32" / name))
        for name in ("vcruntime140.dll", "vcruntime140_1.dll", "msvcp140.dll")]
import numpy as np


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def export(destination, reports):
    from rapidocr import RapidOCR, ModelType, OCRVersion
    destination.mkdir(parents=True, exist_ok=False)
    cases = []
    for source in reports:
        report = json.loads(source.read_text(encoding="utf-8"))
        params = report["params"]
        runtime = {**params, "Det.model_type": ModelType(params["Det.model_type"]),
                   "Rec.model_type": ModelType(params["Rec.model_type"]),
                   "Det.ocr_version": OCRVersion.PPOCRV6, "Rec.ocr_version": OCRVersion.PPOCRV6}
        engine = RapidOCR(params=runtime)
        first = report["frames"][0]
        frame = Path(first["path"])
        assert sha(frame) == first["sha256"]
        # RapidOCR constructs its modules lazily on first use.
        engine(str(frame))

        def recorder(session, kind, model):
            def call(inputs):
                expected = session(inputs)
                name = f"case-{len(cases):02}-{kind}"
                input_path = destination / (name + "-input.bin")
                expected_path = destination / (name + "-expected.npy")
                np.asarray(inputs, dtype="<f4").tofile(input_path)
                np.save(expected_path, expected)
                cases.append({"name": name, "kind": kind, "model": str(Path(model).resolve()),
                    "input": str(input_path.resolve()), "shape": list(inputs.shape),
                    "output": str((destination / (name + "-rten.bin")).resolve()),
                    "expected": str(expected_path.resolve()), "output_shape": list(expected.shape),
                    "image_sha256": first["sha256"], "input_sha256": sha(input_path),
                    "expected_sha256": sha(expected_path), "model_sha256": sha(Path(model))})
                return expected
            return call
        engine.text_det.session = recorder(engine.text_det.session, "det", params["Det.model_path"])
        engine.text_rec.session = recorder(engine.text_rec.session, "rec", params["Rec.model_path"])
        engine(str(frame))
    (destination / "cases.json").write_text(json.dumps(cases, indent=2), encoding="utf-8")
    print(f"Exported {len(cases)} model calls from {len(reports)} first-frame images.")


def collapsed(prediction):
    paths = prediction.argmax(axis=-1)
    return [[int(v) for i, v in enumerate(path) if v != 0 and (i == 0 or v != path[i-1])]
            for path in paths]


def check(directory, output):
    assert not output.exists()
    cases = json.loads((directory / "cases.json").read_text())
    rows = []
    for case in cases:
        for item in ("input", "expected", "model"):
            assert sha(Path(case[item])) == case[item + "_sha256"]
        expected = np.load(case["expected"])
        actual = np.fromfile(case["output"], dtype="<f4").reshape(case["output_shape"])
        assert np.isfinite(actual).all()
        diff = np.abs(expected-actual)
        row = {"name": case["name"], "kind": case["kind"], "shape": list(actual.shape),
               "mean_abs_error": float(diff.mean()), "max_abs_error": float(diff.max()),
               "output_sha256": sha(Path(case["output"]))}
        row["passed"] = row["max_abs_error"] < 0.001
        if case["kind"] == "rec":
            row["ctc_paths_match"] = collapsed(expected) == collapsed(actual)
            row["passed"] &= row["ctc_paths_match"]
        else:
            row["threshold_flip_fraction"] = float(np.mean((actual > 0.2) != (expected > 0.2)))
            row["passed"] &= row["threshold_flip_fraction"] < 0.0001
        rows.append(row)
        print(json.dumps(row))
    result = {"scope": "Identical Python-prepared tensors; Rust operator compatibility only",
              "cases_sha256": sha(directory / "cases.json"), "cases": rows,
              "passed": all(r["passed"] for r in rows)}
    output.write_text(json.dumps(result, indent=2), encoding="utf-8")
    assert result["passed"], "Runtime comparison failed."


if __name__ == "__main__":
    if sys.argv[1] == "export":
        export(Path(sys.argv[2]), list(map(Path, sys.argv[3:])))
    elif sys.argv[1] == "check":
        check(Path(sys.argv[2]), Path(sys.argv[3]))
    else:
        raise ValueError("Expected export or check")
