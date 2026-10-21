"""Fetch pinned official TAPIR research sources and a verified checkpoint.

No package installation, model execution or desktop integration occurs here.
The upstream Apache-2.0 notices are retained alongside verbatim source files.
"""
import hashlib
import json
from pathlib import Path
import urllib.request

ROOT=Path(__file__).resolve().parents[1]
REVISION="730cda1c730877cfedbe01bf87fb1cadb78a565d"
MODEL_REVISION="5d3fb48e76c5422841e38501514121e251beabb7"
MODEL_SHA="87c1e752cf5ce56e3e2f7da460aeb4d40fc826d04ef2939bade86a5c7495377f"
MODEL_SIZE=218887028
VENDOR=ROOT/"research/learned-tracking/vendor"
ASSETS=ROOT/"test-results/point-tracking-v1/assets"

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def download(url,path,limit,expected=None):
    path.parent.mkdir(parents=True,exist_ok=True)
    if path.exists():
        if expected and sha(path)!=expected: raise ValueError("Existing checkpoint differs from pinned source.")
        return
    temporary=path.with_suffix(path.suffix+".download")
    with urllib.request.urlopen(url,timeout=60) as source,temporary.open("xb") as target:
        size=0
        while block:=source.read(1024*1024):
            size+=len(block)
            if size>limit: raise ValueError("Download exceeds declared budget.")
            target.write(block)
    if expected and sha(temporary)!=expected: raise ValueError("Checkpoint checksum mismatch.")
    temporary.rename(path)

sources=[]
for name in ["LICENSE","tapnet/torch/nets.py","tapnet/torch/utils.py","tapnet/torch/tapir_model.py"]:
    url=f"https://raw.githubusercontent.com/google-deepmind/tapnet/{REVISION}/{name}"
    path=VENDOR/name
    download(url,path,1024*1024)
    sources.append({"url":url,"path":path.relative_to(ROOT).as_posix(),"sha256":sha(path)})
model="causal_bootstapir_checkpoint.pt"
model_url=f"https://huggingface.co/google/tapnet/resolve/{MODEL_REVISION}/{model}"
download(model_url,ASSETS/model,MODEL_SIZE,MODEL_SHA)
assert (ASSETS/model).stat().st_size==MODEL_SIZE
card_url=f"https://huggingface.co/google/tapnet/raw/{MODEL_REVISION}/README.md"
download(card_url,VENDOR/"MODEL_CARD.md",1024*1024)
record={"repository":"https://github.com/google-deepmind/tapnet","revision":REVISION,
        "sources":sources,"model":{"url":model_url,"revision":MODEL_REVISION,"path":(ASSETS/model).relative_to(ROOT).as_posix(),"sha256":MODEL_SHA,"bytes":MODEL_SIZE},
        "model_card":{"url":card_url,"sha256":sha(VENDOR/"MODEL_CARD.md")},
        "license":"Apache-2.0; see vendor/LICENSE and pinned official model card"}
destination=ROOT/"research/learned-tracking/upstream.json"
if destination.exists():
    assert json.loads(destination.read_text(encoding="utf-8"))==record,"Preserve prior provenance."
else: destination.write_text(json.dumps(record,indent=2)+"\n",encoding="utf-8")
print(json.dumps({"sources":len(sources),"checkpoint_bytes":MODEL_SIZE,"checkpoint_sha256":MODEL_SHA}))
