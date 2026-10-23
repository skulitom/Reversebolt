# Per-candidate experiment design: not enabled automatically

The proposed design separates measured image/input evidence, fallible visual
interpretation and unexecuted runtime metadata. It asks what condition and
measurement a hypothesis needs before judging the current visual-suppression
experiment. On the frozen development cases it rejected irrelevant probes, but
also rejected the previously useful Viewfinder path. **It must not gate the
automatic planner.** The desktop and MCP builds exclude this research feature.

## Experiment

The [protocol](PROTOCOL.md) was frozen before 24 requests: ten original
Viewfinder paths, ten original Sable paths, an opaque-label case, a synthetic
brightness-only case and two cases with misleading event descriptions appended
to runtime object text. Original plans preceded their native comparisons;
native outcomes were not supplied to the model. The known positive path was
used only to evaluate the result. These are known development targets, not a
withheld generalization test.

Each path is considered separately. Reversing candidate extraction produces
identical individual inputs; there was no second inference run after reversal.
The model receives anonymous input symbols, measured contrast, the prior visual
description and exact selected-path metadata/question. An alternative retains
its distinct path and does not acquire its primary question's meaning.

All requests used qwen3.8:latest, digest
`22130167c4c20e20c7b71454612966ca8e8171e9b3cc8ab6ce8aa6cbfec79643`,
server 0.40.7, temperature zero, seed 42, a 16,384-token context and 1,200-token
output allowance. No game was launched, no new image or input was obtained and
no field was written. Summed request-stage time was **129.107 seconds**; this
is not a native latency benchmark.

## Results and limitations

All 24 outputs passed the structural/reference checks. There were **17 requests
for another observation, six for another activity, one insufficient-evidence
result, and zero current probes**. Seven questions related to footstep/wind
events were not selected as visual-suppression probes. Both misleading metadata
cases still requested audio/event observation and did not claim the supplied
metadata sentence proved that footsteps had been heard.

However, the known positive Viewfinder move-speed path also requested another
observation. Its reason required physical velocity evidence and distinguishing
player movement from camera movement before proposing a test. That rejects a
probe which actually produced a reversible rendered response in the retained
native trial. A meaningful semantic explanation requires stronger evidence,
but an exploratory intervention can provide useful evidence before that point.

There are additional weaknesses. The model selected `unknown` as the readout
for 22 cases, including outputs whose prose requested audio or event logs.
The desired measurement and currently available measurement need a clearer
distinction. In the brightness-only case it marked a movement condition as
supported by the visual interpretation while correctly quoting a description
that establishes no motion. Exact quotation does not imply entailment.
Some reasons ended mid-sentence at the schema's character limit. These outputs
must not be treated as executable instructions or authoritative classifications.

The proposed gate therefore failed its useful-probe retention criterion. No
automatic ranking, filtering, card, edit or desktop behaviour was changed. No
capability credit is added: progress stays **36/100** and the full goal is active.

## Implementation and verification

The Rust core binds a design to one exact path, value, question, selected-path
role and activity record. Metadata and observation quotes have distinct fields;
an object-text sentence cannot be accepted as a visual observation quote. A
current-probe declaration must have compatible readout, prediction direction
and cited condition support. These are consistency checks, not semantic truth.

The observer retains complete prompt/schema/options, response, elapsed time,
model identity and request phase, including failed attempts. A saved design is
rechecked against its exact response and evidence before reuse. The executable
replay has no target-access or edit API. Everything is behind the existing
`comparison-priority-research` feature; default desktop/MCP feature trees were
checked to exclude it. The published preview stays `0.1.0-20261023-012028`.

Four new generated tests cover metadata/observation confusion, incompatible
readouts/predictions/conditions, stale path/value/question bindings, and changed
or incomplete response receipts. All **463 workspace tests**, strict all-target
Clippy and formatting pass. The [offline audit](audit.py) checks frozen sources,
executable/source-plan hashes, exact candidate bindings, all 24 typed receipt
replays and the reported counts. [Results](results.json) retain every decision,
including the failed positive case. Artifacts remain local in
`test-results/test-design-v1`.

```powershell
python research/test-design/audit.py
```

## Next development decision

Do not add another semantic prerequisite to the existing exploratory planner on
this evidence. Keep preliminary intervention evidence separate from later claims
about meaning. Investigate how to obtain the missing behavioral evidence and
match measurements to it: active object/field changes during input, regional
visual response, and event/audio observations where available. Any later policy
must retain useful exploratory probes, avoid interpreting a negative screen
result as disproving an audio/event effect, and demonstrate that its suggested
next observation can actually resolve uncertainty.
