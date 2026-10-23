"""Freeze and run one metadata-only development replay with current code."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys

ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'test-results/sable-naming-replay-v1'
NATIVE=ROOT/'test-results/independent-sable-v1'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
old=read(next((NATIVE/'data/runtime/naming').glob('*/receipt.json')))
report=next(p for p in (NATIVE/'data/runtime').glob('*-*/**/report.json') if sha(p)==old['report_sha256'])
OUT.mkdir()
shutil.copy2(report,OUT/'report.json')
shutil.copy2(ROOT/'target/release/reversebolt-runtime-hypotheses.exe',OUT/'evaluator.exe')
files=list((ROOT/'crates').rglob('*.rs'))+list((ROOT/'crates').rglob('Cargo.toml'))
files += [ROOT/p for p in ('Cargo.toml','Cargo.lock','research/independent-sable/REPLAY-PROTOCOL.md','research/independent-sable/replay.py')]
sources={}
for p in files:
    name=p.relative_to(ROOT).as_posix();dest=OUT/'source'/name
    dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(p,dest);sources[name]=sha(dest)
frozen=dict(source_commit=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
    working_changes_included=True,sources=sources,report_sha256=sha(OUT/'report.json'),
    native_frozen_sha256=sha(NATIVE/'frozen.json'),evaluator_sha256=sha(OUT/'evaluator.exe'),
    model=old['model'],model_digest=old['model_digest'],new_images=0,field_writes=0)
(OUT/'frozen.json').write_text(json.dumps(frozen,indent=2)+'\n')
done=subprocess.run([str(OUT/'evaluator.exe'),str(OUT/'report.json'),str(OUT/'output'),old['model']],
    cwd=ROOT,text=True,capture_output=True)
(OUT/'stdout.txt').write_text(done.stdout)
(OUT/'stderr.txt').write_text(done.stderr)
(OUT/'exit.json').write_text(json.dumps(dict(exit_code=done.returncode))+'\n')
print(done.stdout or done.stderr,flush=True)
sys.exit(done.returncode)
