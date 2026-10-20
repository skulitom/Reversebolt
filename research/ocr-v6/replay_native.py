"""Wait for the native GUI-subsystem executable to complete an offline replay.

Usage: python replay_native.py <exe> <models> <frames> <new-output.json>
PowerShell can return before a GUI executable exits; this helper owns and waits
for its exact child process instead of inferring completion from a launch.
"""
import json
from pathlib import Path
import subprocess
import sys

exe, models, frames, output = map(Path, sys.argv[1:])
assert not output.exists(), "Preserve earlier evidence."
subprocess.run([str(exe.resolve()), "--ocr-sequence", str(models.resolve()),
    str(frames.resolve()), str(output.resolve())], check=True,
    creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0), timeout=600)
report = json.loads(output.read_text(encoding="utf-8"))
assert report and all(frame["screen"]["warning"] is None for frame in report)
print(json.dumps({"frames": len(report), "output": str(output)}))
