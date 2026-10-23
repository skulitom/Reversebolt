import hashlib,json,shutil
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];PRIOR=ROOT/'test-results/runtime-embedded-activity-v2';OUT=ROOT/'test-results/runtime-embedded-activity-v3'
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
assert read(PRIOR/'anode-job.json')['finished'] and read(PRIOR/'anode-job.json')['exitCode']==1
assert read(PRIOR/'failure.json')['holds']==[] and not (PRIOR/'sample-0-0-process.json').exists()
original=read(PRIOR/'target.json');target=read(PRIOR/'seat-target.json')
assert original['window']==0 and target['window']>0 and all(original[k]==target[k] for k in ['pid','created','session'])
OUT.mkdir()
for name in ['bin','source','selection','plan']:shutil.copytree(PRIOR/name,OUT/name)
for name in ['discovery.json','discovery.stdout','discovery.stderr','discovery-process.json','frozen-plan.json']:shutil.copy2(PRIOR/name,OUT/name)
shutil.copy2(PRIOR/'seat-target.json',OUT/'target.json')
frozen=read(PRIOR/'frozen.json');frozen['prior']=str(PRIOR);frozen['prior_frozen_sha256']=sha(PRIOR/'frozen.json')
source=PRIOR/'source/research/runtime-embedded-activity/live.py'
frozen['measurement_source']=str(source);frozen['measurement_source_sha256']=sha(source)
for name in ['resume.py','freeze-resume.py','RESUME-PROTOCOL.md']:
 p=ROOT/'research/runtime-embedded-activity'/name;relative=p.relative_to(ROOT).as_posix();shutil.copy2(p,OUT/'source'/relative);frozen['sources'][relative]=sha(p)
(OUT/'frozen.json').write_text(json.dumps(frozen,indent=2));print(OUT)
