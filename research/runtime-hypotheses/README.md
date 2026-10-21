# Automatic metadata hypotheses

An optional installed local model can now review a supported runtime discovery
inside its existing job. The engine publishes raw cards first, then retains a
bounded set of provisional questions with exact field references, quotations and
separate alternatives in `state.live.runtime.naming`. These proposals do not
rename cards, establish gameplay meaning or grant edit authority. The desktop
continues to show named direct controls with short explanations and Freeze.

## Shared engine contract

Automatic attachment uses the configured observer model. Explicit MCP discovery
can set `local_model`; omission keeps the discovery-only path. The model name is
bound into a content-addressed request document before job admission. This keeps
real process/window/investigation identities within the journal's 256-byte step
limit and detects changes to the saved request. A refresh cannot request naming.

At most two local calls select eight observed types and propose twelve questions
from 256 paths. All observed field labels are available to type selection; path
sampling shares its budget across fields and types before additional instances.
The model sees one candidate ID per row; the engine retains full report-bound
paths. Same-object text belongs only to its observed path, never a global label.
There are no game-specific offsets, title checks or source-derived layouts.

Validation rejects missing references, wrong-object or partial quotations,
unfinished text, repeated candidate paths, missing alternatives, generic-only
names and names whose significant words do not appear in their quotations.
Opaque identifiers provide no semantic anchor. Rejected proposals retain their
original output and reason. These are reference and lexical checks, not proof of
the proposed explanation; meaningful semantics still need independent evidence.

The engine saves intent, inventory and a full receipt before publishing a batch.
Receipts retain model identity, prompts and their hashes, schemas, options, raw
responses, selections, accepted/rejected proposals and errors. Failure to find
the model leaves usable discovery with an unavailable review. Cancellation or
target/catalogue replacement prevents publication. A failed receipt write fails
the job and clears pending hypotheses. Restart retains history without resuming
the review or live target. An in-flight model request settles before cancellation
takes effect; subsequent requests and publication stop. Status and job controls
remain responsive, while other engine operations wait for the owning job.

Current bounds include 110,000 prompt bytes, 16 KiB model response content,
32,768 context tokens, 600/4,096 output tokens and 12 MiB per archive document.
The shared observer uses an already installed local vision-capable model; no
model is downloaded. Archives do not yet have automatic retention pruning.

## Retained replay results

Twelve exploratory runs used already inspected Manifold Garden, menu and context
fixture reports plus a research-only opaque-name variant. There was no new game
launch, gameplay input, screenshot or memory edit. This was iterative development
on retained data, not a held-out evaluation. The local model was `qwen3.8:latest`
with digest `22130167c4c20e20c7b71454612966ca8e8171e9b3cc8ab6ce8aa6cbfec79643`
on Ollama 0.40.7. Full evidence remains under `test-results/runtime-hypotheses-v1`
through `v4`; [results.json](results.json) records identities and counts.

| Version | Manifold accepted/rejected | Menu | Context fixture | Opaque |
|---|---:|---:|---:|---:|
| v1: truncated field index, full row identifiers | 12/0 | 0/0 | 0/0 | 0/0 |
| v2: complete field index, shared sample budget | 0/12 | 4/8 | 0/0 | 0/0 |
| v3: one candidate ID in model input | 12/0 | 1/11 | 0/0 | 0/0 |

In v1 the apparent success was mostly internal flags and omitted movement fields.
In v2 the model confused several different identifier columns and produced bad
quotations; validation rejected them. In v3 it retained `moveSpeed` with
`walkSpeed` as a distinct alternative, as well as carrying/gravity-related fields.
It also proposed UI/miscellaneous settings, and the menu pass retained a raycast
setting without establishing any gameplay relevance. Acceptance is therefore
not a precision or correctness score. The context fixture abstained at type
selection, before its useful same-object text reached stage two. This remains a
known selection limitation. Opaque-name abstention on these cases is not a
general guarantee against hallucination.

v4 changes only engine/job integration after v3; its replay CLI hash is identical
to v3, so the twelve model calls were not repeated. No verified gameplay controls
are created by this pass. Automatic experiment planning, evidence-based semantic
promotion, useful control selection and recovery remain unfinished.

## Native verification and failures

`run_live.py` uses the actual MCP and engine with an owned runtime ABI fixture in
Anode. It passed 14 checks: automatic review, early cards, unchanged card contents,
durable receipt/report binding, missing model, duplicate job, pending cancellation,
forced receipt-storage failure, absence of edits/captures, restart and preserved
model/history. Its successful review produced zero hypotheses; positive candidate
bindings were checked in the retained replays and core tests, not established by
this native fixture. Cancellation was tested while the review was pending and
does not claim to interrupt an HTTP request already running.

The first native attempt exposed a real bug: its inline model name made the job
payload 263 bytes, exceeding the 256-byte limit. v4 moves that setting into the
immutable request document. A regression uses realistic UUID/process identities,
the maximum window ID and a 128-character model name, then tampers with the saved
document to verify rejection. The original failure and child exits remain under
`v3/native`. The evidence auditor initially assumed discovery reports had an
`operation` property; it was corrected to support their existing schema, without
rerunning the workflow.

Final verification: 264 workspace tests, strict Clippy, formatting, release build,
49-tool smoke and the native workflow passed. Four helpers unloaded successfully;
20 pauses balanced with zero helper heap operations while paused. Maximum measured
fixture stop-to-resume was 82 microseconds. All owned children exited, no images or
input actions were recorded, and no gameplay fields were written. Engine/MCP
exit code 1 reflects deliberate test-process termination after assertions; the
fixture exited normally. `audit.py` checks source/binary identities, all twelve
model receipts, actual inventory references, native receipts, pause balance and
child outcomes without further inference. Progress remains **34%**: this is a
shared investigation foundation, not verified automatic gameplay understanding.
