# Automatic regions reach the interpreter and shared context

Automatic discovery now supplies the region measurements to its existing
four-image activity interpreter, even when no observer-labelled regions exist.
The same optional `regions` summary survives successful interpretation through
the receipt and shared `play_context` used by humans and agents. Failed matches
remain explicit missing evidence, and a model's uncertainty is retained.

The full summary binds the proposal report, activity source and frame metadata,
origin/quiet references and quiet input hash. It includes root self-check status,
quiet evidence and the two exact action outcomes, with matched rectangles and
displacements in original image pixels. The five-image, four-candidate/twenty-
check proposal computation is unchanged. It creates no named card, trial
baseline, verified control or edit authority. Older records omit the new field.

The [first frozen study](../passive-region-context/README.md) exposed context
exhaustion when the entire provenance-rich summary was placed in the prompt.
The revised prompt uses only dimensions, requested origin rectangles, root/quiet
status and pair geometry/deltas. Full hashes, frame identities and clocks remain
in durable evidence. This keeps all numeric outcomes and failures while reducing
the model's context use; model, image bytes, schema and generation options remain
unchanged. [Protocol](PROTOCOL.md).

| Saved case, augmented arm | First study prompt bytes | Compact prompt bytes | Compact input / output tokens | Result |
| --- | ---: | ---: | ---: | --- |
| Hyperbolica | 11,925 | 9,660 | 6,916 / 96 | Completed; exact full summary retained |
| Manifold Garden | 10,564 | 8,523 | 6,863 / 167 | Completed; exact full summary retained |

Hyperbolica previously consumed 8,079 input tokens and was truncated after 113
output tokens at the 8,192-token context limit. The compact revision finished
with `done_reason=stop`; the original failure remains archived. Garden completed
in both versions. There was no retry inside either frozen study. The second study
made exactly four local model calls: baseline and augmented for each case.

Both baseline prompts match the first study byte-for-byte. All new numeric
proposal reports and complete summaries match the first study exactly, and two
summary runs per case are identical. Both arms share exactly the same source,
global motion and older appearance evidence. Every successful augmented receipt
retains its supplied full summary in `Receipt.context()`. The summaries are
3,738 and 2,698 bytes respectively, within their 6 KiB limit. Prompt limits remain
24 KiB; the observed model token counts are evidence for these two requests only.

Both cases have positive consistency flags in both arms. This is **not an
interpretation accuracy improvement**. For example, Hyperbolica's augmented prose
calls the desk/background static, while its baseline prose describes desk motion.
There is no semantic ground-truth oracle to resolve those claims here. Garden's
augmented prose combines the earlier region's approximately 40/42.5-pixel shifts
with the new candidate's 35/37.5-pixel shifts, but assigns apparent shapes to them
without verified identity. Every description and uncertainty is retained in
[results.json](results.json). These known development recordings do not establish
generalization, camera/actor separation or causal influence of an internal field.

All 369 workspace test executions passed before the prompt projection revision;
the affected observer test was rerun on the final version, and strict
workspace/all-target Clippy, formatting and diff checks pass. Tests cover exact
original-pixel scaling, source rebinding, malformed geometry, root/quiet stops
without resumption, missing displacement, legacy serialization, full durable
provenance despite prompt projection, and false/uncertain model results. A
no-target-access application test verifies that automatic activity preparation
receives the same summary returned by the shared evidence reader without observer
labels and leaves session state unchanged.

Frozen files are in `test-results/passive-region-context-v2`. All 163 source files,
24 original inputs, 21 generated inputs and both executable hashes pass audit.
The selected local model digest was checked before each interpretation and after
the run. No new screenshot, game input, target memory operation, object/metadata
model call or desktop launch occurred in either study.

- Source/input manifest SHA-256: `a61cecad0559145437ca58e8156d0380cca81ff167a4e52aeb53f701f634e55b`
- Interpretation-input manifest SHA-256: `cd250887613cc3a5a2eaa043fdb54748f5311f40c5e733ab02044af198153be2`
- Proposal executable SHA-256: `9de3b16e9e48abb966fa70c6984f954e726b47a9885948fa8bb1b8fd95e099e7`
- Interpreter executable SHA-256: `26087ad3bd2b1c4cad0fbe3d2ed1355c7a2b9a298f328d68b5ac6c5b44eea68e`
- Model digest: `22130167c4c20e20c7b71454612966ca8e8171e9b3cc8ab6ce8aa6cbfec79643`

Overall weighted progress remains **35%**. The desktop preview is still
unavailable after its earlier Windows quarantine; no replacement was published.
