"""Freeze a published product and evaluator; do not inspect game internals."""
import hashlib
import json
import shutil
import subprocess
import argparse
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
parser = argparse.ArgumentParser()
parser.add_argument("--version", default="v1", choices=["v1", "v2"])
parser.add_argument("--storage", default=str(Path.home() / "AppData/LocalLow/HGS/Freight Hopper"))
parser.add_argument("--appid", default="1763190")
parser.add_argument("--data")
args = parser.parse_args()
assert args.appid.isdigit()
OUT = ROOT / ("test-results/independent-runtime-" + args.version)
BIN = ROOT / "dist/0.1.0-20261022-010125"
STORAGE = Path(args.storage).resolve()
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
OUT.mkdir()
shutil.copytree(STORAGE, OUT / "storage-before")
sources = [p for p in (ROOT / "crates").rglob("*.rs")]
sources += list((ROOT / "crates").rglob("Cargo.toml"))
sources += [ROOT / p for p in ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml"]]
sources += list(Path(__file__).parent.glob("*.py")) + list(Path(__file__).parent.glob("*.md"))
digests = {}
for path in sources:
    name = path.relative_to(ROOT).as_posix()
    target = OUT / "source" / name
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(path, target)
    digests[name] = sha(target)
manifest = Path("D:/STEAM/steamapps") / ("appmanifest_" + args.appid + ".acf")
shutil.copy2(manifest, OUT / "appmanifest-before.acf")
frozen = {"source_commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
          "bin": str(BIN), "data": str(Path(args.data).resolve() if args.data else OUT / "data"),
          "appid": args.appid, "binaries": {p.name: sha(p) for p in BIN.iterdir() if p.suffix in [".exe", ".dll"]},
          "sources": digests, "storage": str(STORAGE),
          "storage_before": {p.relative_to(STORAGE).as_posix(): sha(p) for p in STORAGE.rglob("*") if p.is_file()},
          "appmanifest_sha256": sha(manifest),
          "preserved_review_sample_sha256": sha(ROOT / "target/release/Reversebolt.exe")}
(OUT / "frozen.json").write_text(json.dumps(frozen, indent=2) + "\n", encoding="utf-8")
print(json.dumps({"frozen": str(OUT), "sources": len(digests), "storage_files": len(frozen["storage_before"])}))
