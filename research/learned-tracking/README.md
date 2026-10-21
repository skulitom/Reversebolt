# Learned tracking research

See [method, results and decision](../../docs/research/LEARNED_TRACKING.md).
The product remains Rust. Python and the Torch checkpoint are isolated offline
research dependencies, with no desktop launch or model-download hook.

`vendor/` holds three unmodified official TAPIR Torch source files, the upstream
Apache-2.0 license and pinned model card. `upstream.json` records exact revision,
source hashes, checkpoint hash and download locations. Original Reversebolt
aggregation, harness and documentation retain the repository's MIT license.

Run from the repository root, using Python 3.11. The tested environment was a
dedicated venv with existing NumPy 1.26.4 and Pillow 10.3.0, plus Torch
2.4.0+cu124, dm-tree 0.1.9 and einshape 1.0. The CUDA wheel is about 2.5GB and the
checkpoint about 219MB. The runner requires CUDA and does not silently fall back
to a different inference configuration.

```powershell
python -m venv --system-site-packages test-results/point-tracking-v1/runtime
test-results/point-tracking-v1/runtime/Scripts/python.exe -m pip install --no-deps torch==2.4.0+cu124 --index-url https://download.pytorch.org/whl/cu124
test-results/point-tracking-v1/runtime/Scripts/python.exe -m pip install dm-tree==0.1.9 einshape==1.0 numpy==1.26.4 Pillow==10.3.0
python scripts/fetch-point-tracker.py
cargo build -p reversebolt-observer --bin reversebolt-point-track --release --locked
test-results/point-tracking-v1/runtime/Scripts/python.exe scripts/point-tracking-probe.py --out test-results/point-tracking-v1/NEW-PROBE
test-results/point-tracking-v1/runtime/Scripts/python.exe scripts/point-tracking-evaluate.py --out test-results/point-tracking-v1/NEW-REPLAY
```

The procedural check generates its own images. The 48-window replay requires
the prior local research clips, annotations and frozen patch-tracking receipts;
these game assets are not distributed. Existing output directories cannot be
overwritten. Retain failed outputs, and use a new name for each attempt.

The checked-in image-free evidence describes the recorded local run. Verify
its full local chain with `python scripts/point-tracking-evidence.py` when those
local artifacts and the exact recorded executable are available. Binary hashes
identify this build; they are not a claim of reproducible compilation on every
machine. The model, raw frames, venv and executables stay in ignored directories.
