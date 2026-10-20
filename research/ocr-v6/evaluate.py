"""Image-only PP-OCRv6 comparison. Labels are read by a separate scorer.

Usage: <evaluation-python> evaluate.py <model-dir> <image-dir> <new-output.json>
Uses official pinned ONNX exports with their declared pre/postprocessing.
The Python harness is research tooling, not a new application runtime.
"""
import hashlib
import importlib.metadata
import json
import math
import os
from pathlib import Path
import re
import sys
import time

# This host's base Python bundles VC 14.27. Load the already-installed current
# system runtime for this evaluation process only; do not change Python/system files.
if sys.platform == "win32":
    import ctypes
    runtime_handles = [ctypes.WinDLL(str(Path(os.environ["SystemRoot"]) / "System32" / name))
        for name in ("vcruntime140.dll", "vcruntime140_1.dll", "msvcp140.dll")]

import yaml
import cv2
from rapidocr import RapidOCR, ModelType, OCRVersion
from rapidocr.ch_ppocr_rec import TextRecInput

models, images, output = map(Path, sys.argv[1:4])
assert not output.exists(), "Keep earlier evaluation evidence; choose a new output file."
det = models / (sys.argv[4] if len(sys.argv) > 4 else "PP-OCRv6_tiny_det_onnx")
rec = models / (sys.argv[5] if len(sys.argv) > 5 else "PP-OCRv6_small_rec_onnx")
det_yaml = yaml.safe_load((det / "inference.yml").read_text(encoding="utf-8"))
rec_yaml = yaml.safe_load((rec / "inference.yml").read_text(encoding="utf-8"))
dictionary = rec / "characters.txt"
dictionary.write_text("\n".join(rec_yaml["PostProcess"]["character_dict"]) + "\n", encoding="utf-8")
normalize = next(t["NormalizeImage"] for t in det_yaml["PreProcess"]["transform_ops"] if "NormalizeImage" in t)
post = det_yaml["PostProcess"]
params = {
    "Global.use_cls": False, "Global.text_score": 0.5,
    "Global.log_level": "warning", "Global.use_vertical_padding": False,
    "EngineConfig.onnxruntime.intra_op_num_threads": 4,
    "EngineConfig.onnxruntime.inter_op_num_threads": 1,
    "Det.model_path": str(det / "inference.onnx"), "Det.model_type": "tiny",
    "Det.ocr_version": "PP-OCRv6", "Det.mean": normalize["mean"], "Det.std": normalize["std"],
    "Det.limit_side_len": 736, "Det.limit_type": "min", "Det.use_dilation": False,
    "Det.thresh": post["thresh"], "Det.box_thresh": post["box_thresh"],
    "Det.max_candidates": post["max_candidates"], "Det.unclip_ratio": post["unclip_ratio"],
    "Rec.model_path": str(rec / "inference.onnx"), "Rec.model_type": "small",
    "Rec.ocr_version": "PP-OCRv6", "Rec.rec_keys_path": str(dictionary),
    "Rec.rec_img_shape": [3, 48, 320], "Rec.rec_batch_num": 6,
}
report = {"params": params, "packages": {p: importlib.metadata.version(p)
    for p in ("rapidocr", "onnxruntime", "numpy", "opencv-python-headless", "pyyaml")},
    "model_sha256": {str(p): hashlib.sha256(p.read_bytes()).hexdigest()
    for folder in (det, rec) for p in (folder / "inference.onnx", folder / "inference.yml")}, "frames": []}
params["Det.model_type"] = det.name.split("_")[1]
params["Rec.model_type"] = rec.name.split("_")[1]
runtime_params = {**params, "Det.model_type": ModelType(params["Det.model_type"]), "Rec.model_type": ModelType(params["Rec.model_type"]),
    "Det.ocr_version": OCRVersion.PPOCRV6, "Rec.ocr_version": OCRVersion.PPOCRV6}
engine = RapidOCR(params=runtime_params)
temporal = "--temporal" in sys.argv[6:]
report["temporal"] = {"enabled": temporal, "max_misses": 3, "max_regions": 64,
    "padding": 0, "revisited_output_refreshes_region": False}
regions, previous_size = [], None

def numeric(text):
    text = text.strip().rstrip("%")
    if not re.fullmatch(r"[0-9.+-]+", text):
        return False
    try:
        return math.isfinite(float(text))
    except ValueError:
        return False

def bounds(word):
    p = word["polygon"]
    return [min(q[0] for q in p), min(q[1] for q in p), max(q[0] for q in p), max(q[1] for q in p)]

def overlaps(a, b):
    intersection = max(0, min(a[2], b[2]) - max(a[0], b[0])) * max(0, min(a[3], b[3]) - max(a[1], b[1]))
    return intersection > 0.5 * min((a[2]-a[0])*(a[3]-a[1]), (b[2]-b[0])*(b[3]-b[1]))

for path in sorted(images.glob("*.png")):
    begin = time.perf_counter()
    prediction = engine(str(path))
    words = [] if prediction.boxes is None else [
        {"text": text, "score": float(score), "polygon": box.tolist(), "source": "detected"}
        for text, score, box in zip(prediction.txts, prediction.scores, prediction.boxes)]
    if temporal:
        pixels = cv2.imread(str(path))
        height, width = pixels.shape[:2]
        if previous_size != (width, height):
            regions = []
        previous_size = (width, height)
        detected = [w for w in words if numeric(w["text"])]
        for region in regions:
            box = region["bounds"]
            if any(overlaps(box, bounds(w)) for w in detected):
                continue
            left, top, right, bottom = map(round, box)
            crop = pixels[max(0,top):min(height,bottom), max(0,left):min(width,right)]
            if not crop.size:
                continue
            result = engine.text_rec(TextRecInput(img=crop))
            text, score = result.txts[0], float(result.scores[0])
            if score >= 0.5 and numeric(text):
                words.append({"text": text, "score": score, "source": "revisited",
                    "polygon": [[left,top],[right,top],[right,bottom],[left,bottom]]})
        for region in regions:
            region["misses"] += 1
        for word in detected:
            box = bounds(word)
            old = next((r for r in regions if overlaps(r["bounds"], box)), None)
            if old is not None:
                old.update(bounds=box, misses=0)
            elif len(regions) < 64:
                regions.append({"bounds": box, "misses": 0})
        regions = [r for r in regions if r["misses"] <= 3]
    elapsed = (time.perf_counter() - begin) * 1000
    report["frames"].append({"path": str(path), "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
        "elapsed_ms": elapsed, "words": words})
    output.write_text(json.dumps(report, indent=2, ensure_ascii=False), encoding="utf-8")
    print(json.dumps({"frame": path.name, "ms": round(elapsed),
        "numbers": [w["text"] for w in words if numeric(w["text"])]}, ensure_ascii=True), flush=True)
