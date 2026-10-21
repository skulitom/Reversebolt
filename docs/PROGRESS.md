# Progress against the full vision

This is a weighted capability estimate, not an estimate of elapsed development
time or a measured probability of success. The denominator stays fixed. Credit
requires a concrete implementation and evidence. Partial credit is allowed.

Latest automatic prospective checks (2026-10-21): **27% unchanged**. Connection
can now choose a numeric finding, freeze predictions and check later observations
without manual capture/check commands. A controlled fixture completed the full
sixteen-capture flow; original Tetris automatically froze nine score candidates,
contradicted four and kept five unresolved because the fresh sequence lacked a
hold. Original 2048 still had no usable memory matches. No gameplay control was
verified. All 149 tests and packaged checks pass; desktop preview
`0.1.0-20261021-081604` is running with history preserved. All weighted credits
remain unchanged. [Evidence and limits](../research/automatic-checks/README.md).

Latest explicit location tests (2026-10-21): **27%**. A reviewed test can now
investigate one candidate location while retaining competing predictions and
requiring agreement across overlapping interpretations. Two fixed native cases
tested a source and an unused copy, observed the different responses, and restored
the original bytes; one case used desktop apply/restore controls. All 146 tests
and packaged checks pass. The main UI remains short draggable cards; additional
test choices stay behind Inspect. Desktop preview `0.1.0-20261021-080250` is
running with history preserved. General discovery moves from 4/20 to **5/20**
for explicit experimental planning and retained alternatives. Validation remains
2/15 and independent-game generalization remains 0/10: these fixture tests do
not establish gameplay authority. Other credits remain foundation 4/10, native
access 6/10, observation 5/15, intent 0/10 and agent collaboration 5/10.
[Evidence and limits](../research/location-probes/README.md).

Latest capture timing (2026-10-21): **26% unchanged**. Fresh checks now bracket
images with bounded candidate reads. Changed or unbracketed values count neither
for nor against a match, and late observations cannot bypass that rule. Controlled
tests cover asynchronous changes and later recovery; all 144 workspace tests
pass. Fresh original-game runs retained no verified controls: 2048 had no matches,
and Tetris's fresh score stayed constant before its mappings became unavailable.
The native run verifies endpoint collection and accounting, not reduced timing
errors. Desktop preview `0.1.0-20261021-073028` is running with history preserved.
[Evidence and limits](../research/capture-alignment/README.md). All weighted
credits remain foundation 4/10, native access 6/10, observation 5/15, discovery
4/20, validation 2/15, intent 0/10, agent collaboration 5/10, generalization 0/10.

Latest repeated-evidence check (2026-10-21): **26% unchanged**. Four automated
original-game sessions exposed an isolated-reading false-match pattern. The
generic matcher now requires repeated contrast in both halves and their inliers;
140 tests and packaged MCP/practice checks pass. Fresh 2048 hypotheses were all
contradicted; three Tetris hypotheses agreed with four later OCR readings, but
the last image was independently unscorable. No gameplay control was verified.
[Method, failures and evidence](../research/process-scope-study/README.md).

Latest process-scope support (2026-10-21): **26%**. Window observation and
memory-source identity are now separate, with shared desktop/MCP child selection,
fresh investigations on switching, and guarded writes bound to the actual
memory process. Native fixture capture, scope rejection, child-exit and UI
automation checks pass; 136 workspace tests pass. Native access moves from 5/10
to 6/10. Other credits remain unchanged, including zero withheld-generalization
credit. [Evidence and limitations](../research/memory-sources/README.md).

Latest visual-object coverage (2026-10-21): **25% unchanged**. Automatic
observation can publish provisional named object cards alongside characters
and held items. One fresh automated Tetris run produced seven object findings;
three of four predeclared piece locations matched, with one missed preview.
Three saved character pairs retained all four annotated entities after crop
review. This small exposed evaluation verifies neither descriptions nor gameplay
controls and earns no new generalization credit. See
[evidence and limits](../research/visual-objects/README.md).

Latest observation resilience (2026-10-21): **25% unchanged**. Unsupported text
regions no longer discard unrelated readable values; archived optional-model
failures leave warnings while other captures continue. Paired replay of 100
saved images improves selected 2048 readings and preserves earlier regressions.
Native MCP/UI checks confirm continued capture with an unavailable model. These
changes do not establish controlling state or independent-game acceptance. See
[evidence and limits](../research/observation-resilience/README.md).

Independent-game baseline (2026-10-21): **25% unchanged**. Four automatic
runs of original 2048 and Javascript Tetris retained 64 captures, including an
optional-model failure. No verified editable control emerged. The evidence
exposes limitations in text-region handling, useful-object coverage, process
scope and input cadence; see [the baseline](../research/independent-games/README.md).
All weighted credits remain unchanged and the full goal remains active.

Interface refinement (2026-10-21): **25% unchanged**. The main surface is
now focused on short named cards, with drag arrangement and accessible move
actions. Activity and application diagnostics open separately. Numeric trial
controls still use the existing engine's eligibility and explicit review gates;
unknown findings do not gain cosmetic editors or invented gameplay meanings.
Automated native UI checks exercise arrangement, editing/restoration, and
stable action positions as observations age. This is usability work, with no
additional inference, causal-validation or generalization credit.

| Capability | Weight | Evidence needed for full credit |
|---|---:|---|
| Foundation and delivery | 10 | Open repository, shared persistent engine, accessible desktop, versioned runnable delivery |
| Native target access | 10 | Target selection, reads and captures, typed writes and history, efficient snapshots |
| Multimodal observation | 15 | Screen values, motion, animation, model geometry, synchronized input and audio |
| General discovery | 20 | Numeric matching, experimental planning, objects and relationships, hidden and derived rules |
| Validation | 15 | Authority vs copies, correct representation and scope, actual gameplay consequences |
| Intent and editing | 10 | High-level requests, composed changes, reusable knowledge, recovery and revalidation |
| Agent and human collaboration | 10 | MCP, durable jobs, observable shared sessions, takeover and interruption |
| Generalization and release | 10 | Withheld independent games, regression/performance evidence, usable open-source release |
| **Total** | **100** | **Full VISION.md acceptance criteria** |

## 2026-10-20 — starting point: 2%

Inherited independent MIT research demonstrates affine numeric matching on
randomized byte layouts and live owned-child reads/writes. The Squalr study and
benchmark failures are preserved in `research/`. No credit for a general screen
observer, real-game inference, or autonomous semantic understanding.

Subsequent entries must identify what changed and what remains unproven.

## 2026-10-20 — first desktop preview: 10%

| Capability | Credit / weight | Evidence and limits |
|---|---:|---|
| Foundation and delivery | 4 / 10 | Rust workspace, native desktop, persistent shared engine, versioned preview and desktop launcher; early UX, local distribution |
| Native target access | 1 / 10 | Real Windows reads/writes to an owned process; advertised arena only, no arbitrary target inventory or scan |
| Multimodal observation | 0 / 15 | Numeric fixture oracle only |
| General discovery | 2 / 20 | Affine matching and held-out evidence on randomized layouts; no semantic, object, motion or rule inference |
| Validation | 1 / 15 | Rejects display copies with two practice interventions, verifies both tracks; hidden side effects and representation aliases unresolved |
| Intent and editing | 0 / 10 | Low-level numeric controls only; no high-level intent or recovery across loads |
| Agent and human collaboration | 2 / 10 | Nine working MCP tools, same desktop session, visible activity and history; no durable cancellable jobs or takeover protocol |
| Generalization and release | 0 / 10 | No independent real-game acceptance gate passed |
| **Total** | **10 / 100** | **The full goal remains active** |

Release binaries passed the real-process/MCP smoke suite, and the desktop passed
Anode UI checks at 1280×720. See [validation](VALIDATION.md). The two inherited
discovery regression tests and strict Clippy checks pass.

Next: target inventory and bounded capture, a real screen/behavior observation
path, then an unchanged discovery pipeline evaluated against an independently
implemented offline target. Keep the supplied-label practice as a regression
fixture; do not let it become the product's general inference architecture.

## 2026-10-20 — application connection and observation: 13%

| Capability | Credit / weight | New evidence |
|---|---:|---|
| Foundation and delivery | 4 / 10 | Dark Bolt-family desktop; existing delivery remains |
| Native target access | 3 / 10 | Searchable window picker, exact process identity, read-only attachment, bounded snapshots, disconnect and exit invalidation |
| Multimodal observation | 1 / 15 | Window image + native OCR + numeric screen tracks; 12/12 varied values in an owned WinForms fixture |
| General discovery | 2 / 20 | Existing matcher accepts screen tracks; no new real-game discovery credit |
| Validation | 1 / 15 | Existing practice evidence only |
| Intent and editing | 0 / 10 | No external application writes or high-level intent |
| Agent and human collaboration | 2 / 10 | Five new MCP tools share the application connection and history; 14 total |
| Generalization and release | 0 / 10 | No independent real-game acceptance gate passed |
| **Total** | **13 / 100** | **The full goal remains active** |

The native fixture test found no correspondence in its 4 MiB sampled memory;
this is recorded as zero hypotheses, not successful discovery. Unit evidence
covers excluding incomplete pages and revoking hypotheses on process exit.
Anode checks cover search, selection, connection, capture, empty search,
cancellation, and the corrected sidebar layout. See `VALIDATION.md`.

Parallel research owns isolated `experiments/` directories for visual behavior,
causal validation, and hidden-rule inference. Their results receive no
integration or generalization credit until reproduced and integrated.

## 2026-10-20 — graphics capture and explicit OCR uncertainty: 14%

The multimodal observation credit increases from **1 / 15 to 2 / 15**.
Other category credits remain unchanged: foundation 4, native access 3,
discovery 2, validation 1, intent 0, agent collaboration 2, generalization 0.
Total: **14 / 100**. The full goal remains active.

Windows Graphics Capture now handles Blue Prince where PrintWindow returned
black, including a window obscured by Reversebolt. Both rendering paths use
the selected client area. Local Windows and neural OCR retain their sources;
overlapping conflicting readings become uncertain samples with visible
alternatives. Regions cannot expand themselves through repeated recognition,
resizing retires tracks, and inference now follows memory acquisition.

The Half-Life 2 development clip improves from 19/36 to 33/36 correct numeric
samples, with three explicit abstentions. The final ordinary-window test passes
24/24 values across its original and fresh sequences. It still finds zero
memory hypotheses. Blue Prince's fresh gameplay reveals a false 1850 reading
for an icon beside 50, followed by an abstention on 50 versus 950. This is
failure evidence, not successful gameplay discovery.

Nine workspace tests, strict Clippy, the native checks and the published
practice/MCP smoke suite passed. Desktop preview
`0.1.0-20261020-201128` is running with the previous history retained.
See [observation evaluation](research/OBSERVATION_EVALUATION.md).

Round 2 research has isolated assignments for memory coverage, shared durable
jobs and real-game visual tracking. Next main-session work: distinguish icons
and scenery from text, prevent spurious associations across scene changes, and
collect changing gameplay evidence in Blue Prince's separate test profile.

## 2026-10-20 — Blue Prince changing-counter test: still 14%

The full goal and all weighted credits remain unchanged. The test found a real
idle-capture crash: the cached Windows OCR factory referenced an unloaded DLL.
A process-lifetime runtime hold fixes that lifetime mismatch. The same process
then completed twelve gameplay captures and another capture after a 200-second
idle gap. Ten tests, strict Clippy, formatting and release practice/MCP checks pass.
Desktop preview `0.1.0-20261020-203437` carries the fix.

The visible counter changed from 49 down to 38 during normal movement between
two rooms. The unchanged observer admitted zero correct values, three incorrect
values, eight conflicts and one missing sample. It often included the adjacent
footprint icon as digits. Correlation correctly refused to run without enough
changing observations. This does not earn discovery or generalization credit.
Raw images, separately recorded evaluation labels and failure evidence are
described in [observation evaluation](research/OBSERVATION_EVALUATION.md).

Next: generic icon/text separation and stylized-number observation, with fresh
gameplay and prior-game regressions; review the completed shared-jobs prototype
before integrating automatic investigations. Memory coverage remains another
limit: this run sampled 24 MiB, about 0.58% of eligible memory at attachment.

## 2026-10-20 — image-grounded agent collaboration: 15%

Agent and human collaboration increases from **2 / 10 to 3 / 10**. Other credits
stay unchanged: foundation 4, native access 3, multimodal observation 2,
discovery 2, validation 1, intent 0, generalization 0. Total: **15 / 100**.
The full goal remains active.

Two new MCP tools return actual capture images and accept typed, source-tagged
interpretations. Numbers, text, booleans, vectors and explicit uncertainty share
the desktop investigation. Images and observations are bound to capture and
connection identities; replay is idempotent and incompatible replacements are
rejected. Historical evidence survives restart while live authority is revoked.

In Blue Prince, the agent inspected three actual MCP images and submitted the
visible counter readings 38, 37 and 36, plus descriptive and uncertain evidence.
The desktop displayed the interpretations and their sources. This is an
agent-assisted integration test, not a blinded accuracy benchmark, automatic
model invocation, OCR improvement or discovered gameplay control. No discovery
or generalization credit is awarded.

Thirteen workspace tests, strict Clippy, formatting, the 16-tool release smoke
suite, nine rejection checks and archive/restart verification passed. Desktop
preview `0.1.0-20261020-210429` includes the change. See
[agent observation evidence](research/AGENT_OBSERVATIONS.md).

Next: integrate bounded shared investigation jobs and broader memory coverage,
then evaluate a repeatable agent observation loop against fresh gameplay.
Completed research prototypes still require independent reproduction and engine
integration; their results do not count as delivered product capabilities.

## 2026-10-20 — bounded coverage across memory batches: 16%

Native target access increases from **3 / 10 to 4 / 10**. Other credits remain:
foundation 4, multimodal observation 2, discovery 2, validation 1, intent 0,
agent collaboration 3, generalization 0. Total: **16 / 100**. The full goal
remains active.

The desktop and MCP can now continue beyond the initial memory window. Each
batch keeps its observations separate, archives hypotheses and sparse byte
witnesses, then releases its raw snapshots. Mapping checks exclude observed
discontinuities, and coverage distinguishes addresses read from complete local
histories. Read, retention, image and archive limits are explicit.

In paused Blue Prince, two 16 MiB batches each completed 12 captures. A third
batch brought unique reads to 48 MiB, with complete histories for 32 MiB. The
static scene supplied no changing numeric observations, so matching refused to
run. No gameplay control or editing ability is claimed. The prototype's broader
synthetic recall is not transferred to the application as generalization credit.

Twenty-one tests, strict Clippy, formatting and the 17-tool release smoke suite
pass. Anode checks cover the desktop workflow; MCP advancement and archive
preservation after restart also pass. Preview `0.1.0-20261020-212137` carries the
change. See [memory coverage evidence](research/MEMORY_BATCHES.md).

Next: durable cancellable investigation jobs that can coordinate observations
and memory batches, followed by fresh changing-gameplay evaluations. The full
vision still requires semantic inference, causal validation and scoped edits.

## 2026-10-20 — shared observation jobs and human takeover: 18%

Agent and human collaboration increases from **3 / 10 to 5 / 10**. Other credits
remain: foundation 4, native access 4, multimodal observation 2, discovery 2,
validation 1, intent 0, generalization 0. Total: **18 / 100**. The full goal
remains active.

Desktop and MCP share durable timed observation jobs, request receipts, progress
and event cursors. Pause/cancel remain responsive during a capture, retain its
actual outcome and prevent later steps. Human takeover holds agent mutations.
Target/batch changes invalidate old work. Restart leaves interrupted jobs in
history without replay or automatic continuation.

The integrated workspace passes 53 tests. Blue Prince checks through real MCP
and Anode cover completion, retry, cancellation during capture, desktop takeover,
human pause/resume/stop, rebind and restart. The initial native cancellation
acknowledgement took 53 ms while status took 27 ms. A human-started desktop job
also captured six moments before pausing and stopping. These are read-only
coordination results, not gameplay understanding or discovered controls.
Desktop preview `0.1.0-20261020-214537` carries the change with prior history
retained; the release regression suite passes with 20 MCP tools.

The production plan supports captures only. Broader investigation, multiple
agent identities, journal lifecycle, semantic interpretation, meaningful game
experiments and scoped edits remain required. See
[observation job evidence](research/OBSERVATION_JOBS.md).

## 2026-10-20 — fresh Blue Prince OCR experiment: still 18%

All capability credits remain unchanged. A general PP-OCRv6 medium reader with
regions learned from prior images reads 12/12 counters in a fresh Blue Prince
sequence after its configuration was frozen. It also reads 36/36 selected fields
in the Half-Life 2 development clip. A footprint creates one separate false `1`;
ordinary-window text is correct but its numeric field is not isolated.

The official models run in Rust's existing RTen runtime. Six real-input model
calls match the Python reference's detection masks and all twelve decoded text
crops. This is runtime feasibility, not a native observer implementation.
The full evaluation and pinned reproduction tools are in
[the OCR experiment](../research/ocr-v6/README.md).

The experiment is not integrated, has not searched or validated game memory,
and does not earn generalization credit. The desktop remains on the tested
`0.1.0-20261020-214537` preview. Next is native extraction, word geometry and
uncertainty integration, followed by fresh complete-pipeline evaluation.

## 2026-10-20 — native styled text and fresh live evaluation: 19%

Multimodal observation increases from **2 / 15 to 3 / 15**. Other credits remain
foundation 4, native access 4, discovery 2, validation 1, intent 0, agent
collaboration 5 and generalization 0. Total: **19 / 100**. The full goal remains
active, with the same denominator and acceptance criteria.

The Rust observer now runs pinned PP-OCRv6 models, extracts numeric words from
text lines and re-reads bounded regions learned only from prior images. Windows
OCR remains independent; disagreements are gaps. Spatial matching no longer
lets a nearby icon displace a direct counter overlap. Sources and alternatives
remain visible to desktop users and agents.

After freezing native code and binaries, all 12 fresh Blue Prince counter values
were tracked correctly, from 25 through 14. A fresh ordinary-window sequence
also passed 12/12 with both readers active. Development replay results were
12/12 Blue Prince, 36/36 selected Half-Life 2 values and 12/12 ordinary-window
values. These are selected-field results: false footprint readings and duplicate
Rank readings remain. There is no whole-image precision or unseen-game claim.

Real MCP comparison returned 18 non-exact hypotheses from 16 MiB of Blue Prince
memory. None is a verified control; monotonic activity can generate spurious
correlations. No game writes occurred. Median live capture time was 1.89 seconds,
mostly recognition. The cost and the remaining false readings are explicit.

All 60 workspace tests, strict Clippy, formatting and the 20-tool release smoke
suite pass. Preview `0.1.0-20261020-224603` is running on the normal desktop with
the identical tested binary, pinned models and all five prior activity entries
retained. See [native OCR evidence](research/NATIVE_OCR.md).

Next: investigation sequences that distinguish changing gameplay state from
time and activity, broader memory coverage, and visual behavior evidence beyond
text. Causal validation, scoped live-game edits and restart revalidation remain
required before the full goal can be completed.

## 2026-10-20 — automatic discovery, control cards and frozen predictions: 20%

Validation increases from **1 / 15 to 2 / 15** for implemented prospective
falsification, supported by controlled failure tests and a real-game rejection
result. Other credits remain foundation 4, native access 4, observation 3,
discovery 2, intent 0, agent collaboration 5 and generalization 0. Total:
**20 / 100**. The full goal and denominator remain unchanged and active.

Four initial Blue Prince correlations failed all four later observations with
their coefficients frozen. The engine retains predictions, fresh bytes, images,
timing and contradictions; no control is promoted. Missing evidence, geometry,
mapping, process exit and persistence faults are tested. Identical copies remain
indistinguishable. This is progress in rejecting bad hypotheses, not causal
control, representation recovery or unseen-game generalization.

Automatic OCR in this run was 12 correct, 2 missing and 2 wrong on the selected
counter. Agent interpretations stayed separate. The failure is preserved and
the observer was not retuned. No game writes occurred.

Connecting now starts a bounded 16-moment observation pass automatically, with the first
capture immediate and comparison from moment 12. The main surface is now control
cards with names, short descriptions and values; investigation information and
images are secondary. Live cards remain in Learning/Uncertain states, while
verified practice values can be dragged or typed. Pause/stop remain visible.
Humans and MCP share the same controller. A scripted native window
passed 16/16 readings, automatic comparison, pause/resume/stop and manual opt-out;
it found zero memory hypotheses. Sixty-seven workspace tests pass.

Follow the user's testing preference: automate regression and recorded-input
evaluation, using owned windows for native plumbing. Live-game control and
manual screenshots should answer a specific remaining uncertainty. See
[prediction and automatic-discovery evidence](research/FROZEN_PREDICTIONS.md).

Preview `0.1.0-20261020-233851` is running on the normal desktop with all seven
prior history entries retained. Its published binaries and model hashes were
verified; the 21-tool release smoke passed. Native UI Automation verified a
practice numeric edit and restoration. The updated interface receives no extra
general-discovery credit. Delivery is recorded in
`test-results/automatic-report-v1/desktop-delivery.json`.

## 2026-10-20 — temporal evidence and shared timing: still 20%

The engine now retains image and memory-read operation bounds on one monotonic
clock. Agents can read ordered image sequences and submit multi-image
appearance, motion or change interpretations. The desktop shows them as
provisional cards, with stale observations distinguished from current values.
Persistence, scope, ordering, gaps and immutable retries are enforced.

Seventy-two workspace tests passed, and an automated native shape fixture
verified the real MCP image/observation round trip. UI Automation verified the
card and Inspect view without screenshots. This receives no additional motion
understanding or generalization credit: no semantic observer, reliable entity
tracking, high-cadence acquisition or temporal memory matching is integrated.
The full goal remains active at **20 / 100**, with the same denominator.
See [temporal evidence and limitations](research/TEMPORAL_EVIDENCE.md).

Desktop preview `0.1.0-20261020-235611` was delivered with preserved history,
matching executable/model hashes and a passing 23-tool published-build smoke.

## 2026-10-21 — faster visual sampling: still 20%

Agents can now request short image/memory sequences without paying the text
recognition cost between samples. Full automatic discovery remains the default.
In one owned-window run, median operation time fell from 1,793.5 to 193.5 ms;
actual visual samples began 310.938 ms apart. Pause/resume/cancel and skipped
reading gaps were verified through actual MCP. No game input was used.

The 75-test workspace suite and an added visual-mode integration test passed,
76 tests total. This is acquisition progress, not motion understanding or
generalization; the unchanged weighted estimate remains **20 / 100**. Faster
retained-stream capture, semantic observation and temporal matching remain open.
See [visual sampling evidence](research/VISUAL_SAMPLING.md).

Desktop preview `0.1.0-20261021-000945` was delivered with all seven history
entries preserved exactly. Executable/model hashes and the launcher were
verified; its 23-tool published-build smoke passed. Delivery proof is
`test-results/visual-sampling-v1/desktop-delivery.json`.

## 2026-10-21 — local semantic observer replay: still 20%

A new Rust observer can run already installed local vision models over ordered
images, retaining model/image/prompt provenance and rejecting invalid outputs.
It does not receive game identities, labels, memory or game-specific layouts.
The categorized full replay covers 53 previously annotated pairs from five
Quake/Celeste clips. At strict IoU 0.5 it matches 45/58 visible targets with 44
unmatched proposals; 3/53 outputs fail the structural contract. Conditional
motion error on the 11 moving matched pairs is 4.14 annotation pixels.

This is meaningful semantic-proposal research but insufficient for automatic
control publication. The observer remains experimental, with no causal/editing
or unseen-game claim. All 79 tests pass (78 workspace tests plus the final CLI
guard test); strict Clippy passes. The desktop preview remains available.
The stable weighted goal estimate stays **20 / 100**. Next: independent proposal
verification and tighter grounding before automatic shared-session integration.
See [local observer evidence and scoring corrections](research/LOCAL_OBSERVER.md).

## 2026-10-21 — crop-focused review: still 20%

The Rust observer can now review an unlabelled crop with its source context,
retaining exact image provenance and coordinate transforms. A frozen automated
replay reviewed 90 proposals across the same 53 archived pairs. Category
filtering retained all 45 strict matches and reduced unmatched proposals from
44 to 16; 27 removals were semantic judgments and one was a contract failure.
Using the new coordinates instead lost a correct match. The experiment therefore
supports filtering with original coordinates, not automatic box refinement.

This remains correlated same-model evidence on already seen clips, with false
proposals and missed actors. Median additional cost is 3.817 seconds per review.
It does not establish persistent identity, physical movement, memory relations
or editing authority. No desktop source or game control was enabled. All 81
workspace tests, strict Clippy, formatting, image-provenance negative checks and
exact receipt-only rescoring pass. The desktop remains available.

All category credits and the denominator stay unchanged at **20 / 100**; the full
goal remains active. Next: independent grounding and bounded shared-session
model execution, preserving provisional observations until effects are verified.
See [crop review evidence](research/CROP_REVIEW.md).

## 2026-10-21 — automatic local appearance findings: 21%

Multimodal observation increases from **3 / 15 to 4 / 15** for an implemented
image-derived source in the shared desktop/MCP workflow. Other credits stay
foundation 4, native access 4, discovery 2, validation 2, intent 0, agent
collaboration 5 and generalization 0. Total: **21 / 100**. The denominator and
full goal remain unchanged and active.

With a local vision model selected, connection automatically reviews captured
image pairs after moments 2 and 16. Source-bound proposals and crop reviews can
produce provisional named appearance cards. Descriptions are short; Inspect
retains the explanation. Models have no process handles or write authority.
The same durable job owns capture, inference and publication; cancellation,
failure and restart behavior are explicit.

An automated native replay produced a character card without manual capture or
model commands. A second pass acknowledged cancellation in 35.97 ms and
published nothing; failed model work also stopped subsequent capture steps.
Restart preserved settings/history and resumed no jobs. All 85 tests, strict
Clippy, native UI Automation and the published 24-tool smoke pass. This is
integration evidence using already seen imagery, not a new accuracy result or
independent-game acceptance gate. No gameplay input or writes were used.

Desktop preview `0.1.0-20261021-011044` is running with the tested local model
enabled and prior history preserved. Next: persistent, uncertainty-aware visual
measurements that can be compared with internal trajectories, followed by
independent validation of meaning and effects. See
[integration evidence and remaining limits](research/LOCAL_OBSERVER_INTEGRATION.md).

## 2026-10-21 — seeded tracking replay: remains 21%

An original Rust image-patch tracker and a source-bound offline runner now have
automated evidence over all 48 adjacent annotated one-second windows. Fixed
appearance, ambiguity and reverse-search gates retain only 1/15 moving endpoint
opportunities from initially correct seeds. Ungated search is worse than holding
boxes still. This does not justify publishing trajectories or increasing any
capability credit. The denominator and all category credits remain unchanged.

All 91 workspace tests, strict Clippy and formatting pass. Existing appearance
cards and desktop preview `0.1.0-20261021-011044` remain available. The new code
is research-only; no new capture, game control or model inference was needed.
Next: independent acquisition and observation freshness, then a tracking method
that passes moving/animated-object retention before internal-state comparison.
See [the failed approach and evidence](research/SEEDED_TRACKING.md).

## 2026-10-21 — capture alongside image review: remains 21%

The first image interpretation now runs on a separate worker while ordinary
capture continues. Its source is immutable and late results fill only their
original image slots. The native fixture captured ten additional moments while
the first review remained pending. Pause/resume, failed-model handling, restart
and visible card ages were checked automatically. All 94 tests and strict Clippy
pass. This refines the existing observation capability; category credits and the
fixed denominator remain unchanged. The full goal stays active.

Desktop preview `0.1.0-20261021-015824` is running, with the prior eight history
entries and model selection preserved. Its 24-tool smoke and final native model
pass/failure checks passed; test windows were closed afterward.

See [concurrency evidence and limits](research/BACKGROUND_OBSERVER.md). Next:
improve observation cadence and moving-object measurements before comparing
them with internal state; retain explicit source freshness and uncertainty.

## 2026-10-21 — exact-action validation research: remains 21%

The new research planner preserves ambiguous types while comparing their exact
requested byte actions. A frozen 1,080-search run found scoped tested actions
in 323 searches, versus 41 canonical type identifications. Across 2,160 requests,
620 passed the declared effect scope and longer audit; 120 remain unknown in
effect scope, and 80 of those have hidden effects in the private audit. Prior
failed recovery/disposal and 1,420 untested requests remain in the record.

Eight new tests, strict Clippy and source-bound automated verification pass.
This improves the research direction but has not been integrated with live
targets or evaluated on new independent games. Category credits and the fixed
denominator remain unchanged; the full goal stays active. The existing card-based
desktop preview is still running. See [evidence and limits](research/ACTION_EQUIVALENCE.md).

Next: operation-level evidence in the shared engine, with real target lifecycle,
effect observation and recovery capabilities. Keep representation uncertainty
explicit; a tested requested action does not establish a whole slider range.

## 2026-10-21 — explicit native trial cards: 22%

Native target access increases from **4 / 10 to 5 / 10** for an implemented
experimental write/restoration path through the shared desktop and MCP engine.
Other credits remain foundation 4, multimodal observation 4, discovery 2,
validation 2, intent 0, agent collaboration 5 and generalization 0.
Total: **22 / 100**. The denominator and full goal remain unchanged and active.

Cards offer a draggable or typed requested number, Review edit, explicit Try
edit, Restore bytes and Check effect. Every surviving interpretation must agree
on the requested byte action. The journal records intent before native entry,
retains actual outcomes and prevents replay of uncertain writes. Restoration
checks complete model bytes. Visual agreement is separately recorded and does
not establish gameplay authority or complete recovery.

An owned native fixture was discovered without a supplied field address. MCP
changed its visible number 50 → 53 → 50. The final UI Automation test edited the
requested number and performed 50 → 54 → 50 through the card. All 104 workspace
tests, strict Clippy, formatting, native backend guards and the published
28-tool smoke pass. Earlier OCR-fallback and fixture failures remain documented.
No real-game editing or independent-game acceptance credit is earned.

Desktop preview `0.1.0-20261021-030209` is running, retaining the prior eight
history entries and local model selection. Owned test windows were closed.
See [operation evidence and limits](research/LIVE_TRIAL_EDITS.md). Next: scoped
causal effect checks, recovery beyond restoring bytes, and unchanged evaluation
on independently implemented games; broader visual and hidden-rule discovery
remain necessary parts of the full goal.

## 2026-10-21 — broader trial observations: remains 22%

Explicit trial preparation now captures a fresh baseline and watches up to 32
screen-number regions. Post-edit/restoration checks compare other values as
well as the requested number. Cards show fresh contextual readings and a short
change notice; Inspect holds details. Agents can retrieve hash-checked
before/after images through the new read-only evidence tool, including after
restart. Missing/ambiguous readings remain unknown.

The automated native matrix found a residual resource change after restoration,
also detected independent time drift, and failed to reveal private hidden state
changes. Those distinctions are retained: observed differences are not causal
attribution or proof of complete rollback. No gameplay meaning or unseen-game
acceptance gate has been passed. The fixed category credits and total remain
**22 / 100**, with the full goal active.

All 105 workspace tests, strict Clippy, formatting, the three-mode native matrix
and a separate published-build UI/MCP run pass. Preview
`0.1.0-20261021-031846` is running with prior history and model settings retained.
See [effect observations and limits](research/EFFECT_OBSERVATIONS.md).
Next: grounded action/context measurements and controlled contrasts to separate
ordinary activity from intervention effects, including nonnumeric behavior.

## 2026-10-21 — learned point tracking: remains 22%

An isolated causal BootsTAPIR runner now supplies source-bound point trajectories
to original Rust box aggregation. It uses pinned Apache-licensed upstream code
and weights, with no game profiles or labels entering the tracker. Five new
Rust checks cover geometric fit, missing/scattered points, malformed input and
permanent loss. The controlled translation probe has 0.451px median point error.

Automated replay of the same 48 saved-game windows improves gated moving-endpoint
retention from 1/15 to 11/15, but loses all three cases with displacement at least
four annotation pixels. Four false initial seeds still survive. The result is
research-only; it does not establish meaningful motion, gameplay control or
unseen-game generalization. A full repeat after an input-validation correction
has identical outcome metrics. See [results and limitations](research/LEARNED_TRACKING.md).

Category credits and the fixed denominator stay unchanged at **22 / 100**.
The full goal remains active. The current card-based desktop preview remains
running; this experiment adds no desktop model dependency or report view.
The interface design explicitly retains named controls, short descriptions and
appropriate direct actions, with evidence under Inspect. All 110 workspace
tests, strict Clippy, formatting and the receipt-chain verification pass.
The desktop check confirms the previous preview is responsive with eight
history entries and the local model setting retained. Next: foreground point
selection and supported deformation, while retaining fresh detection and simple
hold baselines and preserving all failed cases.

## 2026-10-21 — foreground input context: 23%

Multimodal observation increases from **4 / 15 to 5 / 15** for integrated,
timed Windows key-state sampling in the selected foreground window. Other
credits remain foundation 4, native access 5, discovery 2, validation 2,
intent 0, agent collaboration 5 and generalization 0. Total: **23 / 100**.
The fixed denominator and full goal remain unchanged and active.

Automatic jobs collect bounded input polls alongside image/memory observations.
Pause, stop, failure, completion and disconnect end collection. Resuming starts
a fresh generation. Humans can inspect a short capture-context summary, while
MCP reads the same samples, timing bounds, focus states and explicit gaps.
Key states are not game-delivered events, continuous holds or learned meanings.

Automated native fixtures verify selected-window keys, exclusion of another
window's keys, pause/resume generations, cancellation and completion. The
published build passes the same checks, an accessibility-only Inspect test,
evidence persistence and restart invalidation. All 114 workspace tests,
strict Clippy, formatting and the published 29-tool smoke pass. No gameplay
semantics, causal effects or independent-game acceptance gate is claimed.

Desktop preview `0.1.0-20261021-040557` is running with eight retained history
entries and the existing local vision model selection. No owned Anode test
windows remain. See [input evidence and limits](research/INPUT_CONTEXT.md).
Next: controlled action/context contrasts that use this evidence to challenge
candidate relationships, including the same input producing different or no
effects; appearance/motion reliability and hidden rules remain unfinished.

## 2026-10-21 — automatic input/value contrasts: 24%

General discovery increases from **2 / 20 to 3 / 20** for integrated generic
input/value relationship hypotheses, counterexamples and suggested follow-up
observations. Other credits remain foundation 4, native access 5, multimodal
observation 5, validation 2, intent 0, agent collaboration 5 and generalization 0.
Total: **24 / 100**. The denominator and full goal remain unchanged and active.

The shared engine now compares bounded sampled-key episodes with OCR numbers
and provisional image values. Simple cards can show a short tentative pattern;
Inspect and MCP retain later disagreements, changes without detected keys and
excluded intervals. Sustained holds count as one episode. Missing/focus-lost
observations and mixed keys cannot become evidence of inactivity. Earlier
input generations and image geometry do not carry patterns into a fresh scope.

An unchanged native four-mode algorithm distinguishes a repeated decrease,
no response, a third-action counterexample, and independent drift. A separate
published coupled run, accessibility-only UI check, persistence/restart checks,
120 workspace tests, strict Clippy and the 29-tool smoke pass. The boolean path
uses declared fixture observations and earns no visual-understanding credit.

This is descriptive relationship discovery, not prospective experimental
validation, learned gameplay semantics or causal control authority. No independent
game acceptance credit is added. Preview `0.1.0-20261021-042809` is running with
prior history and model selection preserved. See [the evidence and limits](research/ACTION_PATTERNS.md).
Next: use candidate patterns to select state-conditioned, prospective contrasts,
including delayed or hidden effects, and connect them to nonnumeric behavior.

## 2026-10-21 — conditional predictions before sampled actions: 25%

General discovery increases from **3 / 20 to 4 / 20** for automatic conditional
state/history hypotheses with frozen parameters, retained boundary ambiguity,
and forecasts prepared before fresh sampled actions. Other credits remain
foundation 4, native access 5, multimodal observation 5, validation 2, intent 0,
agent collaboration 5 and generalization 0. Total: **25 / 100**. The fixed
denominator and full goal remain unchanged and active.

Two owned native fixtures look identical during discovery but use different
conditions for a change. Restoring the visible number separates their frozen
predictions: later outcomes reject the history explanation in one and the
starting-value explanation in the other. Threshold gaps and late/missing
forecasts stay unknown. New forecasts are saved before publication; save failures
stop updates without automatic resumption. Human cards and MCP share these checks.

The research rule library's stronger input/measurement assumptions are not
asserted for passive samples, and its verification gate was not imported. This
is original, limited conditional prediction over observed episodes. It does not
recover hidden state, prove a unique rule, establish causal edit authority or
pass independent-game acceptance. No validation/generalization credit is added.

All 128 workspace tests, strict Clippy, formatting, native state/history checks,
forced persistence-failure checks, a published UI/MCP run and restart checks
pass. Earlier failures and the late-declaration regression are preserved.
Preview `0.1.0-20261021-045650` is running with history and model settings retained.
See [conditional prediction evidence](research/CONDITIONAL_PREDICTIONS.md).
Next: evaluate these predictions against less controlled behavior and genuinely
independent software, retain failures, and improve motion/appearance observations
and active selection of reachable distinguishing contexts.

## 2026-10-21 — independent browser-game failure baseline: still 25%

The unchanged published product and installed local model were evaluated on
two fresh sessions each of independently authored 2048 and Javascript Tetris.
All 64 captures, 664 delivered input pulses, original source hashes, model
responses and final states were retained and audited. No oracle observations,
game profiles or memory addresses entered the product; no edits were attempted.

Tetris's changing score was correct in 30 captures, with two unscorable moments.
2048's score produced nine correct readings, one wrong reading, one conflict,
five abstentions and 16 unscorable moments. Constant-zero row readings are kept
separate. Temporary Tetris memory matches disappeared later. No visual/temporal
tracks, conditional checks or verified controls emerged. One completed capture
sequence ended in a failed job after its optional model exceeded a summary limit.

The full goal remains active with unchanged credits: foundation 4, native access
5, observation 5, discovery 4, validation 2, intent 0, agent collaboration 5 and
generalization 0. Total **25 / 100**. Collecting independent failures adds useful
evidence but does not pass independent-game acceptance. No new desktop binary
was needed; preview `0.1.0-20261021-053218` remains running with the card interface.

Next: preserve usable text when one region exceeds recognition limits; broaden
grounded visual objects beyond actors and held components; isolate optional
model failures. Use saved-image regression first, followed by fresh evaluation.
Process relationships and active capture/input planning also remain unresolved.
See [method, results and artifact audit](../research/independent-games/README.md).

## 2026-10-21 — retain partial observation and isolate model failures: still 25%

Valid PP-OCRv6 regions survive alongside explicitly skipped unsupported crops.
Archived optional inference/validation failures contribute no items, retain
source-bound warnings and allow other captures to continue. Evidence persistence,
scope, publication and worker failures still stop the pipeline. Human Inspect
details and MCP expose the same warnings; no invalid proposal gains edit authority.

Paired replay processes 100 saved images through both old and new executables.
Across the 2048 runs, selected score readings improve from nine to 13 correct;
the one wrong reading becomes conflicting. Best-counter readings improve from
12 to 15 correct. Thirty-four browser-counter opportunities remain unscorable;
these are never counted correct. Tetris and the previous Blue Prince, Half-Life
2 and ordinary-window selected fields retain their results. Full-scene precision
and fresh-game performance remain unmeasured. Extra retained text costs additional
recognition time in the 2048 sequences.

The exact rejected Tetris proposal is a regression fixture. A real four-step
MCP job also completes with two deliberately unavailable model reviews while
its native numeric card remains readable. Accessibility verifies the compact
notice and optional details; no game control or manual screenshot review was
needed. All 132 workspace tests, strict Clippy, formatting and release build pass.

All weighted credits remain unchanged: foundation 4, native access 5, observation
5, discovery 4, validation 2, intent 0, agent collaboration 5 and generalization 0.
Total **25 / 100**; the full goal remains active. Next: grounded visual items
beyond actors and held components, followed by fresh evaluation, while addressing
process scope and the gap between visible values and actual controlling state.

Preview `0.1.0-20261021-060956` is running with eight prior history entries and
the selected local model preserved. The packaged 29-tool MCP/practice smoke and
binary/model hash checks pass. No owned Anode fixture remains. Full results,
reproduction and compact evidence are in the observation-resilience research.

## 2026-10-21 — separate memory process: 26%

The window owner can remain the image/input source while an accessible child
supplies memory. Inventory, exact creation identities, writes and source changes
are shared between the UI and MCP. Twelve native fixture captures produce six
owned-arena hypotheses; source changes start fresh, stale/unrelated identities
are rejected, and child exit invalidates capture. The UI source action completes
its automatic 16-capture pass. Native write/restore guards also pass on the
selected child. These checks establish access and isolation, not gameplay roles.

Credits: foundation 4/10, native access **6/10**, observation 5/15, discovery
4/20, validation 2/15, intent 0/10, agent collaboration 5/10, generalization 0/10.
Total **26/100**. Automatic memory-source selection and verified real-game
controlling state remain unfinished; the full goal remains active.
