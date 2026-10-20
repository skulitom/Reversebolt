"""Download and verify the exact official models used by this experiment.

Usage: python get_models.py <destination> [--metadata-only]
Existing files must match. Partial downloads remain with a .download suffix;
they are never loaded as models. No model data is sent to a remote service.
"""
import hashlib
import json
from pathlib import Path
import sys
import urllib.request


def main():
    manifest = json.loads(Path(__file__).with_name("models.json").read_text(encoding="utf-8"))
    destination = Path(sys.argv[1])
    metadata_only = "--metadata-only" in sys.argv[2:]
    for model in manifest["models"]:
        for file in model["files"]:
            if metadata_only and file["name"].endswith(".onnx"):
                continue
            path = destination / model["name"] / file["name"]
            path.parent.mkdir(parents=True, exist_ok=True)
            if path.exists():
                assert path.stat().st_size == file["bytes"], f"Size mismatch: {path}"
                assert hashlib.sha256(path.read_bytes()).hexdigest() == file["sha256"], f"Hash mismatch: {path}"
                print(f"Verified {path}")
                continue
            temporary = path.with_suffix(path.suffix + ".download")
            with urllib.request.urlopen(file["url"], timeout=60) as response, temporary.open("xb") as out:
                total = 0
                while chunk := response.read(1024 * 1024):
                    total += len(chunk)
                    assert total <= file["bytes"], f"Oversized response: {path}"
                    out.write(chunk)
            assert total == file["bytes"], f"Incomplete download: {path}"
            assert hashlib.sha256(temporary.read_bytes()).hexdigest() == file["sha256"], f"Hash mismatch: {path}"
            # Exclusive final creation prevents overwriting a concurrent writer.
            with path.open("xb") as out:
                out.write(temporary.read_bytes())
            temporary.unlink()
            print(f"Downloaded and verified {path}")


if __name__ == "__main__":
    main()
