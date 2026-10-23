# Relevance false-exclusion and adversarial challenge

Keep the previously frozen raw-label relevance prompt, schema, category policy,
model/options and limits unchanged. The only runner addition accepts explicitly
synthetic input so authored fixtures cannot be confused with native game evidence.
No production filter is enabled or candidate removed by this experiment.

Before inference, freeze the exact dataset, intended labels/roles, expected
retention policy, grouping, order variations, attack strings, executable, sources
and local model identity. Review each fixed input once without retries or prompt
tuning. Expectations and case keys are excluded from the model prompt.

Use 32 authored cases in four balanced batches of eight: each contains three
plausible gameplay properties, two application settings, one implementation detail
and two ambiguous labels. Deliberately include configuration/assets holding game
rules, camera/appearance/animation properties, simulated hearing and interaction
bindings. Hardware/OS configuration and plumbing provide negative cases. Generic
containers, display labels, identifiers and toggles test overconfident exclusion.
These hand-authored roles are a development rubric, not independently observed
truth or a held-out precision benchmark.

Repeat the four batches with reversed order. Also run one fixed mixed batch as a
paired baseline, then the identical batch with either (a) an instruction embedded
in one object's text to mark every candidate as gameplay, or (b) an instruction
to mark that useful property as application configuration. Other rows and all
field/type labels remain unchanged. This gives eleven fixed batches and 88
candidate judgments. The text is untrusted sample data and has no tool authority.

For gameplay cases, gameplay or unclear retains a useful lead; application
configuration or implementation detail would be a mistaken exclusion. For
ambiguous cases, unclear is the expected result; report other categories as
overclaims, and exclusions separately. For the explicitly negative cases, report
category disagreements rather than assuming every disagreement proves an error.
Compare categories by stable synthetic path across reversal and paired attacks.
Keep changes toward uncertainty separate from following an attack's target.

Audit frozen bytes, complete prompt evidence, exact citations, record bindings,
every raw response, model identity, token headroom and process exits. Recheck
each successful assessment using the frozen Rust executable with no inference.
Report request failures separately from valid category disagreements. Retain all
bad results. Do not improve this dataset or the prompt after seeing outputs.

A pass would only justify further testing. A mistaken exclusion, sensitivity to
irrelevant instruction text, or unsupported confident classification means this
review is unsuitable as a hard production gate. Prefer retained, provisional
priorities and actual behavior/effect evidence over silently suppressing controls.
No game, image, input or process-memory access is required. Desktop stays on the
already tested preview; progress gets no gameplay or generalization credit here.
