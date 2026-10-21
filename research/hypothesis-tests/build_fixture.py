"""Owned integration fixture: expose one explicit metadata label for model selection.

This is evaluator-only coupling, not a real game or a semantic generalization test.
The original fixture remains unchanged; generated sources stay with the evidence.
"""
import hashlib,json,shutil,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
OUT=Path(sys.argv[1]).resolve();source=ROOT/'research/runtime-paths/mock'
generated=OUT/'fixture-source';(generated/'src').mkdir(parents=True,exist_ok=False)
original=(source/'src/lib.rs').read_text()
assert original.count('c"Coefficient"')==1
modified=original.replace('c"Coefficient"','c"WalkSpeed"')
(generated/'src/lib.rs').write_text(modified)
shutil.copy2(source/'Cargo.toml',generated/'Cargo.toml')
subprocess.run(['cargo','build','--release','--manifest-path',str(generated/'Cargo.toml')],check=True)
(OUT/'bin').mkdir(exist_ok=True)
shutil.copy2(generated/'target/release/reversebolt_path_fixture.dll',OUT/'bin')
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
(OUT/'fixture-build.json').write_text(json.dumps({'source':str(source.relative_to(ROOT)),
    'original_sha256':sha(source/'src/lib.rs'),'generated_sha256':sha(generated/'src/lib.rs'),
    'change':'One metadata label: Coefficient -> WalkSpeed; layout and behavior unchanged.',
    'binary_sha256':sha(OUT/'bin/reversebolt_path_fixture.dll'),
    'limitation':'Owned fixture with an explicit label; no independent semantic/generalization evidence.'},indent=2))
