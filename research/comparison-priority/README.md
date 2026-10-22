# Comparison-priority research — not enabled

The local model did not reliably improve which existing comparison to try first.
The experiment is retained behind `comparison-priority-research`, an explicit
Cargo feature that desktop and MCP builds do not enable. No automatic planner,
discovery job, displayed control or published desktop binary was changed.

## Experiment and result

We replayed the saved pre-trial Viewfinder plan, with its ten existing choices,
four observed input holds, fallible scene interpretation and measured evidence.
The model could nominate at most eight indices; all remaining choices retained
their original relative order. It could not alter probe values, recorded input,
restoration rules or edit authority. Native trial outcomes were withheld from
the model. The positive path was used only to assess the returned order.

Each version had four declared cases: original input, reversed candidate order,
opaque labels, and a synthetic brightness-only description. The latter two are
counterfactual robustness checks. This known development game is not held out.
All eight requests used the same local model digest. No new screenshots, target
reads, game input or memory writes were performed.

| Case | V1 | V2 |
| --- | --- | --- |
| Original | Abstained; known positive remains 3rd | Known positive ranked 7th |
| Reversed | Abstained; known positive remains 8th | Known positive ranked 2nd |
| Opaque labels | Abstained | Abstained |
| Brightness-only description | Abstained | Nominated four sound-related candidates |

V1's explanations requested established links before proposing an experiment;
its prompt also omitted measured image contrast. V2 added those measurements and
clarified that an experiment investigates an unproven mechanism. The separately
frozen V2 did produce nominations, but worsened the original ordering and gave a
different answer after reversal. Its first reversed choice shares a field label
with the known positive but is a different, untested path; it is not counted as
the established effect. One run per case cannot separate order sensitivity from
server nondeterminism.

V2 also described metadata strings referring to sound events as observed events.
Those strings were read from runtime objects; there was no audio evidence. It
ranked them for the synthetic brightness change too. A syntactically valid answer
and exact evidence references did not make that reasoning reliable. Some reasons
also ended mid-sentence within the schema's length cap.

Summed model-stage time was 17.970 seconds for V1 and 17.048 seconds for V2. These
are development timings for four requests each, including cache effects, not
latency benchmarks. There is no new accuracy or generalization claim. Progress
remains **36/100**.

## Retained implementation and checks

The core experiment binds a preference to the exact report, activity, actions,
candidate order, labels and values. Applying it preserves every full choice and
its history. Invalid, duplicated or stale nominations fail; abstention keeps the
original ordering. The observer retains the full prompt, options, response and
model identity, and checks the saved response against the accepted preference.
Cancellation, missing stages, incomplete responses and prompt overflow cannot
produce an accepted preference. These checks establish record consistency and
ordering limits, not semantic truth or safety of a proposed edit.

Six generated unit tests cover those boundaries. The current offline `check`
command also revalidates the exact typed digest and saved V2 response, and
applies the proposed ordering without contacting the model. V1 has a different
frozen prompt and intentionally cannot be accepted as a current-policy receipt.
All **456 workspace tests** passed with the research feature enabled, as did
strict all-target Clippy, formatting, the four V2 typed receipt checks and the
artifact audit. The default desktop/MCP dependency feature trees were separately
checked to exclude this feature. Logs are retained locally as
`test-results/comparison-priority-{workspace,clippy,default-features}.log`.

The [artifact audit](audit.py) checks frozen source hashes, input transformations,
prompt evidence, model identity, output indices and result accounting. Its
[results](results.json) retain the exact paths and receipts used for assessment.
The existing native effect evidence is described in
[the Viewfinder record](../independent-viewfinder/README.md).

Local artifacts are in `test-results/comparison-priority-v1` and
`test-results/comparison-priority-v2`. Both retain sources, input plan, prompts,
responses and results. V2 also retains its executable. V1 recorded a hash of the
then-current executable without copying it; that binary has since been replaced.
The audit does not claim to verify V1's missing binary or its typed input digest.
The source snapshots remain available. Both original runs and their failures are
preserved; the current harness additionally propagates a failed process exit.

To check saved evidence without another model request:

```powershell
python research/comparison-priority/audit.py
cargo run -p reversebolt-observer --example comparison-priority-replay --features comparison-priority-research -- check test-results/comparison-priority-v2/original
```

Repeat `check` for `reversed`, `opaque` and `other_activity` to validate all four.
A new inference run needs a separately built release example, a fresh output
directory and the local model service. The frozen V1/V2 directories must not be
overwritten.

## What would justify integration

The next design should distinguish actual observations from unexecuted metadata
clues in its evidence format, and evaluate candidate relevance across several
activities and independently authored targets. Permutations, irrelevant object
text and obscured labels need explicit checks before native comparisons. A model
preference should earn a measurable reduction in comparisons or investigation
time while preserving abstention and alternate candidates. Adjusting a prompt
until this one known path wins would not establish that benefit.
