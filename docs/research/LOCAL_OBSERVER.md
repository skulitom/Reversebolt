# Local model observations — 2026-10-21

This is the original actor/held-component study. The observer is now integrated
into automatic desktop jobs and also proposes discrete non-character objects.
See [background integration](BACKGROUND_OBSERVER.md) and the later
[selected-object evaluation](../../research/visual-objects/README.md). Historical
methods and scores below retain their original scope.

## Purpose and implementation

The earlier image-difference observer confused camera movement, lighting and
background changes with entities on actual footage. The new Rust
`reversebolt-observer` library and executable test a complementary route:
image-only semantic proposals from an already installed local vision model.
It receives 2–4 ordered PNGs, never game names, executable metadata, source
layouts, memory addresses, labels or action logs. No training or game-specific
prompt is used. This is a research observer, not yet an automatic desktop source.

The optional runner is Ollama at the fixed loopback address `127.0.0.1:11434`.
The client bypasses proxies, refuses redirects and remote-model declarations,
requires installed vision capability, and checks the model tag digest before
and after inference. It never downloads a model. A local server is still a
trusted runtime: these checks do not attest what code or weights it executed.

Requests contain images, timestamps, a generic prompt and JSON schema. They
contain no tools. Responses retain the exact model text, image hashes, model
digest, server version, prompt/schema/options and timing. JSON shape, string
length, indices, ordering and geometry are checked without clamping or silently
repairing output. Truncated generations, malformed JSON and tool calls are
rejected. A valid structure is **not evidence that its contents are correct**.

The categorized protocol asks for actors, held components, other objects or
uncertainty. Stationary characters are included. Names and short descriptions
remain model claims, with no persistent identity, physical speed, memory relation
or editing authority. Even a plausible description can have incorrect boxes.
The initial full-scene protocol permits up to six proposals. Generation uses
temperature 0, seed 42, 8,192 context tokens and a 1,500-token output ceiling;
thinking is disabled when the installed model advertises that capability.
Each HTTP request has a 90-second timeout; that is not a worker-cancellation or
real-time latency guarantee.

The final runner additionally bounds the input manifest to 64 KiB and checks its
2–4-frame count before reading images. An existing receipt is rejected before
inference. Metadata HTTP requests use five-second timeouts; generation retains
90 seconds. These final transport/CLI guards did not change the measured prompt,
schema, model options or interpretation rules.

## Evaluation design and corrections

Use the pre-existing five-clip Quake/Celeste archive and all 53 annotated pairs.
These clips and labels were already inspected by earlier research. This is a
replay experiment, not a new holdout, unseen-game evaluation or completion gate.
The model never receives annotations. Each pair is an independent request,
usually 100 ms apart, with no earlier frames or learned persistent identity.

Initial feasibility compared the first two annotated pairs in each clip.
Gemma 3 12B produced valid structures in 9/10 cases but poor localization.
The installed `qwen3.8:latest` tag produced substantially better visible boxes
on some frames but also proposed scenery. Its digest, not the mutable tag name,
identifies the measured local artifact; this is not a claim about another
upstream model with a similar name.

The earlier labels call the idle clip negative for **dynamic activity**, while
explicitly retaining a stationary character under `static_reference`. The first
scorer incorrectly treated that as absence of an actor. One raw image was
inspected to resolve the mismatch. The corrected visible-entity evaluation adds
those supplied static-reference boxes; it does not create new annotations or
backfill them into earlier frames. The old feasibility output is retained.
Do not compare the corrected headline precision directly with the earlier
activity observer's headline precision.

Other corrections: ignore regions are raw boxes and apply only to unmatched
proposals; UTF-8 receipts must be decoded explicitly on Windows. A scorer decode
failure occurred after the tenth Qwen receipt had been saved. Rescoring reused
all ten receipts without another inference request.

The final categorized prompt was changed after feasibility and then held fixed
for the full replay. It was not trained on the labels, but development feedback
informed its wording. Count only model-declared actors and held components as
proposals; retain other/uncertain classifications separately. Match one-to-one
at IoU 0.3 and 0.5. Invalid outputs count as no detections. Report moving actors,
held components and supplied static references separately. These labels are not
exhaustive for every useful object, collectible, door or gameplay concept.

Paired displacement scoring requires both boxes to match the same annotated
entity at loose IoU 0.3. It reports coverage and conditional endpoint error in
320×180 annotation pixels, not world speed. Existing labels have roughly ±2
pixel uncertainty. Stable appearance descriptions, animation rules and hidden
properties have no semantic ground truth in this archive and remain unscored.

## Measured result and integration decision

The categorized Qwen replay completed all **53 pairs**; **50** returned a valid
structure. Three title-screen responses exceeded the 300-byte summary bound
(306, 343 and 306 ASCII characters). They are structural failures, not correct
semantic abstentions. The five annotated negative pairs include these three
rejections and two accepted responses with unmatched actor/component proposals.

| Clip | Pairs | TP / FP / FN at IoU 0.3 | TP / FP / FN at IoU 0.5 |
|---|---:|---:|---:|
| Quake development | 11 | 15 / 3 / 0 | 14 / 4 / 1 |
| Celeste idle, including supplied stationary-character references | 7 | 7 / 8 / 0 | 7 / 8 / 0 |
| Celeste movement | 11 | 8 / 14 / 2 | 7 / 15 / 3 |
| Quake archived test clip | 11 | 14 / 4 / 3 | 11 / 7 / 6 |
| Celeste archived transition clip | 13 | 7 / 9 / 2 | 6 / 10 / 3 |
| **Total** | **53** | **51 / 38 / 7** | **45 / 44 / 13** |

One unmatched proposal overlaps a supplied ignore region and is excluded at
both thresholds. Loose precision/recall is **57.3% / 87.9%**; strict is
**50.6% / 77.6%**. Strict matches include **19/29 dynamic actor extents**,
**19/22 held components** and **7/7 stationary references**. These are repeated
sampled observations, not 58 independent entities. A false proposal here can be
the wrong category, a duplicate, or a poorly located real entity; it does not
necessarily mean an entirely invented object.

The model repeatedly classifies a decorative flower as an actor in the idle
scene and sometimes describes a terrain shape as a face-like object. Better
generic prompt wording reduced some scenery proposals but did not solve that
problem. Tiny moving sprites and clipped weapon extents remain difficult.

Of 44 loose current-frame matches with a prior annotation, 43 also match that
same entity in the preceding frame. Their mean displacement endpoint error is
**1.30 annotation pixels**, but **32/43** are stationary. On the **11 moving
pairs**, mean error is **4.14 px** and maximum **7.87 px**. Stationary pairs have
0.33 px mean predicted displacement. Do not present the pooled error or nearly
zero median as evidence of accurate general motion tracking.

Median inference receipt time is **4.864 seconds per pair** on this shared RTX
4090 host, including the actual request and post-request model check. Initial
uncached loads can take much longer (the first Qwen probe took 38.2 seconds).
This is a sparse semantic proposal source, not a frame-rate tracker.

**Keep this observer experimental rather than automatically publishing its
outputs as desktop controls.** The next integration needs image-grounded
proposal verification, better small-object localization, explicit rejection
reasons and a durable/cancellable model executor. Persistent object association,
semantic change validation and memory correspondence remain separate work.
The current desktop preview `0.1.0-20261021-000945` remains running; this research
does not change its control cards or claim new game editing capability.

The full 78-test workspace suite and strict Clippy passed, followed by the added
CLI limit test and all observer tests/lints: **79 passing tests total**. A final
real-model smoke used the same prompt/digest and reproduced the measured first
pair's interpretation exactly. That one repeat is not a determinism guarantee.
The measured source snapshot and hashes remain under the local evidence root.

Compact checked evidence is
`research/local-observer/evidence/replay-v1.json`. Raw receipts, run manifests,
full per-pair scores, the initial feasibility failures, scorer corrections and
source snapshots are in `test-results/local-observer-v1/`. No new game capture
or input was needed; one old source image was viewed to resolve label scope.

## Reproduction and provenance

```powershell
cargo test -p reversebolt-observer --locked
cargo build -p reversebolt-observer --release --locked
python scripts/local-observer-evaluate.py --model LOCAL_INSTALLED_TAG --out test-results/NEW_RUN
python scripts/local-observer-paired-score.py test-results/NEW_RUN
```

The evaluator expects the existing local `experiments/visual-challenge` footage
and labels. A fresh checkout without that private archive can use the executable
with its own JSON manifest: `model` plus `frames`, each containing an absolute
PNG `path` and a strictly increasing `timestamp_ns`. Paths are read locally and
are not sent to the model. Supply a new receipt filename; evidence is not
overwritten. `--rescore` uses retained receipts and never starts inference.

Original Rust/evaluator code is MIT. Ollama and installed model weights are
separate dependencies and are not redistributed. Gemma has its own model terms;
the local Qwen alias/provenance is recorded as installed metadata rather than
asserting a license for an unverified upstream identity. No commercial game
footage is added to the repository or sent to a remote model.

Primary API references: [Ollama image input](https://docs.ollama.com/capabilities/vision),
[structured outputs](https://docs.ollama.com/capabilities/structured-outputs),
[chat options](https://docs.ollama.com/api/chat).
[Gemma's model card](https://ai.google.dev/gemma/docs/core/model_card_3) describes
its image/text input; that capability does not establish game-observation accuracy.
