# Progress against the full vision

This is a weighted capability estimate, not an estimate of elapsed development
time or a measured probability of success. The denominator stays fixed. Credit
requires a concrete implementation and evidence. Partial credit is allowed.

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
