# Shared engine, two interfaces

Value-guided search is a selection pass, separate from correspondence fitting.
It scans up to 512 MiB of previously unselected memory for literal encodings of
an OCR observation, retaining at most 4096 64-byte neighborhoods by stable
address hashing. Per-region metadata is checked around each read; partial,
failed or changed reads supply no candidates. The search discards its bytes and
archives its receipt before using the normal batch transition to collect fresh
trajectories. The seed never supplies a training or prospective sample. Search
reads share the 4 GiB ceiling, saved selection files share the archive budget,
and at most eight searches can occur in one investigation. Ordinary broad
cohorts remain available for values that do not share their visible encoding.

Windows Graphics Capture samples a short fixed window after the first completed
frame (80 ms, at most eight frames) and retains the last completed image. Pixel
content never selects a winner or causes a retry. The native request keeps only
the latest image plus small per-frame receipts: completion time, raw compositor
timestamp, dimensions, RGBA hash and a diagnostic detail fraction. A genuinely
blank scene remains blank. A dominant-color warning is informational and does
not turn missing OCR into a remembered value. Compatibility capture has no WGC
receipt. The existing one-worker and five-second caller bounds still apply;
if OS shutdown hangs, that worker remains occupied. Sequential memory/image
brackets remain non-atomic and do not rule out compositor lag or ABA changes.

Known-location rechecks use the same batch archive transition as broad memory
search. Before publishing the transition, the engine validates the original
process, batch and window epoch; it rejects
unrestored or unresolved trials. New checks retain all frozen interpretations,
link to the previous check/archive, and begin with empty samples. Candidate
reads are narrowed to their full numeric widths and shared by address; each
image is still bracketed by bounded memory reads. Earlier contrast and success
never enter the new assessment. Original fitting frames remain provenance.
The existing capture and archive budgets bound repeated rechecks; old raw
snapshots are released. Failed evidence persistence leaves the current batch
in memory. This is not a cross-file crash transaction or allocation/restart
identity guarantee. The controller normally queues four OCR captures without model calls;
additional observations remain possible within the ordinary 20-capture limit.

The reference learner scans only complete fitting snapshots for stable aligned
64-bit pointers with an inferred offset up to 4096 bytes. Four routes per
interpretation are retained; no game names, source layouts or pointer chains
are given. A recheck resolves these roots with mapping/read/mapping checks and
may add fresh hypotheses at new addresses. Old locations remain, including
unavailable rivals. The shared 200-candidate budget still applies. A suggested
destination that cannot be queried is a recorded unresolved route, not proof
that other routes are invalid. Root reads are charged conservatively against
the existing read budget. Root prefix reads precede all value prefixes; root
suffix reads follow value suffixes. A relocated candidate needs the full timing
bracket and a stable destination in addition to its ordinary fresh comparison.
Trial models carry their reference into the target preflight checks, so a later
move blocks application or restoration. These sequential guards are not atomic.

```mermaid
flowchart LR
    Person[Person] --> Desktop[Native Rust desktop]
    Agent[Agent] --> MCP[MCP stdio adapter]
    MCP --> API[Authenticated loopback API]
    Desktop --> Controller[Responsive job controller]
    API --> Controller
    Controller --> Journal[Durable receipts and job checkpoints]
    Controller --> Queue[One operation at a time]
    Queue --> Engine[Shared session engine worker]
    Engine --> Evidence[Observations and candidate evidence]
    Engine --> History[Persistent history and edit records]
    Engine --> Target[Practice target interface]
    Engine --> Live[Selected application session]
    Live --> Native[Read-only observation handle]
    Live --> Changes[Explicit trial and recovery journal]
    Changes --> Write[Temporary guarded write handle]
    Live --> Window[Bounded window capture and OCR]
    Engine --> Observer[Optional installed local vision model]
    Observer --> Evidence
    Target --> Memory[Windows process memory]
    Target --> Observations[Numeric observation adapter]
```

The engine worker exclusively owns the target. A separate controller thread
owns the job journal and dispatches one engine operation at a time. The UI
renders published snapshots and queues commands. HTTP joins the same controller;
status and job-control replies do not wait for a native capture. Other commands
receive their completed engine state. OS file locks protect both the session
directory and journal. No per-job engine workers are created. Only an explicit
trial opens a temporary native write handle; normal observation stays read-only.

The practice target is a separate owned process. It reports only its byte-arena boundary
and observations. Discovery enumerates naturally aligned numeric representations
and robust affine mappings, reserving the second half of the trajectory for
validation. Its result is evidence of correspondence, not proof of authority.

The current experiment applies +3 and -4 interventions, advances two idle ticks,
checks both observed channels, and restores the original arena between trials.
Only a single surviving hypothesis is editable. This is **practice-only**:
real-game rollback cannot overwrite an entire active region, and two scalar
observations cannot establish absence of hidden side effects.

Successful changes record before/after bytes, observation values, and a session
identity. Restore compares current bytes to the recorded result first. Metadata
is saved by replacing a synchronized temporary file. This is not yet a
crash-transactional write journal; interruption between a process write and
metadata save remains a limitation. Startup invalidates active addresses.

## Selected applications

The application picker and MCP `list_processes` use the same inventory of visible,
titled windows on the engine's Windows desktop. Selection carries a PID,
creation-time identity, and exact window handle. Attach checks these again
before taking a read-only process handle. Failed replacement leaves the current
connection intact. Idle polling invalidates the connection when the process
exits. Disconnect releases the connection without terminating the application.

`LiveSession` is independent of practice's fixed channels and oracle. It plans
bounded 64 KiB reads of committed writable regions, prioritizing writable image
regions and then smaller non-image regions. The desktop budgets 16 MiB per moment;
the API permits 1–32 MiB. Up to 20 moments and 512 MiB of raw snapshots are
retained per memory batch; this is not a whole-process memory ceiling.
Coverage is explicit. Pages unavailable in any sample are excluded from
correlation; missing OCR detections remain gaps, not zeros.

`advance_memory` archives observations and hypotheses, retains up to 256 sparse
byte witnesses, and releases the old raw snapshots before collecting a fresh
batch from previously unselected addresses. Interval subtraction preserves
unvisited suffixes of partially selected regions. Selection does not restart at
an earlier address when protection changes, so repeated churn cannot force the
planner to reselect that address indefinitely. It also means changed/reused
addresses are not revisited automatically.

Each chunk's allocation base, protection, type and containing range are checked
before and after a read. An observed incompatibility retires that chunk for the
rest of the batch; failed reads remain missing. Identical metadata cannot prove
allocation continuity. Native inventory errors are distinguished from the
documented upper-address boundary and query/region counts are bounded.

Coverage separates selected/read address unions, complete local trajectories,
requested I/O, retained raw bytes and bytes actually compared. A complete
trajectory needs at least 12 successful captures in one batch, with no observed
mapping discontinuity. It does not require a usable visual track. Memory read
receipts retain timing relative to each capture and a disposition.

Limits per investigation: 24 batches, 4 GiB requested reads, 512 MiB image
allowance and 32 MiB batch archives. A renderer finishing after timeout retains
its 10 MiB reservation; native PNG encoding is capped before writing a file.
Image allowance and batch archives are separate from the bounded per-frame
metadata and visual manifests, not a global disk quota. There is no automatic
memory sweep or restart continuation. Native calls remain cooperatively bounded and cannot
be preempted. See [coverage validation](research/MEMORY_BATCHES.md).

Window capture first uses Windows Graphics Capture for the selected HWND through
the MIT `windows-capture` library. PrintWindow's client area is a compatibility
fallback, with its reason reported. A single capture worker and a five-second
wait bound renderer blocking without accumulating capture threads. A stalled
worker retains its slot until it actually finishes. Neither path captures a
monitor or the whole desktop. The graphics frame is cropped to the selected
window's client area using checked window geometry; a geometry mismatch falls
back with a warning. Resize invalidates track association. Images and capture
metadata remain on disk.

Memory is sampled immediately after the image, before OCR. Recognition runs on
the shared worker afterward; screen capture, memory read and recognition durations
are reported separately. This reduces inference-induced sampling delay but does
not make screen and memory capture atomic.

Visual capture mode bypasses recognition while retaining the same image, memory,
timing and provenance paths. It clears adapter-supplied words and records explicit
gaps in numeric tracks. Full mode remains the default, including automatic
connection jobs. Visual jobs permit 50 ms post-completion delays and an immediate
first step; no target frame rate is guaranteed. The renderer still starts/stops
its bounded graphics capture for each image. Modes share all retention limits,
worker ownership and interruption semantics. See [cadence evidence](research/VISUAL_SAMPLING.md).

Windows OCR and optional pinned PP-OCRv6 medium models run locally. The native
`reversebolt-vision` crate uses RTen, with four inference threads, bounded
detection and recognition tensors, and checksums before model loading. CTC word
positions approximate word geometry within detected text lines. Each word carries
its source and bounds. Fresh numeric detections establish up to 64 regions that
can be re-read on later frames, with at most three missed detections. Recovery
never enlarges or refreshes its own seed geometry; resize and errors clear it.
Legacy Ocrs installations remain supported when PP-OCRv6 is absent. Only one
neural model version participates, alongside independent Windows OCR.
Neural recognition is currently limited to 2.1 million source pixels; unsupported
sizes, missing models and errors retain Windows observations with warnings.

Overlapping numeric readings are grouped; different values or a box bridging
distinct fields produce an explicit uncertain observation with all alternatives,
not a majority vote. These become `None` in matcher input. Agreement remains
provisional, not calibrated confidence. Direct spatial overlaps reserve their
tracks before any proximity fallback. Competing assignments become explicit gaps
independently of iteration order. Neighboring letters are not forcibly
converted into digits. Numeric tracks retain per-frame reading evidence as well
as samples; nearby text is a description, not a semantic gameplay label.

After at least 12 moments, the same general numeric matcher compares varied OCR
tracks against captured memory. Results retain absolute locations as hypotheses
only. Native causal intervention and arbitrary application writes are not
implemented; practice write commands never target this session.

## External visual observations

`visual.rs` defines typed, image-bound interpretations from agent or human
transports. Each investigation and image has a fresh identity; image bytes are
hashed and checked when read or used for a new submission. Scalar, boolean,
text and vector values keep provenance and explicit provisional/uncertain
status. Only provisional scalars enter memory correlation. The producer is
responsible for observing the image; the engine cannot certify a claimed model
name or the truth of its interpretation. OCR remains an independent channel.

MCP `read_capture` returns an image block, and `submit_observation` uses the
same serial engine owner as the desktop. External series are bounded to 32 and
aligned to the current batch's at most 20 captured moments. Duplicate submissions are no-ops;
conflicting replacements, geometry changes and stale identities are checked.
Accepted interpretations are saved in a local evidence manifest before they
are published, independently of active-session metadata. The manifest remains
historical on restart, with no restored process handle or write capability.

Single-frame evidence is insufficient for rates, animation or causal claims.
The temporal observation protocol now binds 2–8 ordered images and their capture
intervals to one declared appearance, screen-motion or change interpretation.
Image and memory-read intervals share an investigation-local monotonic clock.
Elapsed bounds, omitted moments and gaps remain explicit. Temporal series are
saved before publication, immutable on replay, invalidated by geometry/target
changes and archived with their memory batch. They are excluded from scalar
correlation; no continuous identity, physical motion or authority is established.
See [temporal evidence](research/TEMPORAL_EVIDENCE.md).

Temporal semantic inference and calibrated uncertainty remain required work.
Memory coverage and the original capture-only jobs use
reviewed integrations of their isolated prototypes; neither implies semantic
understanding or general causal experiments.

The experimental `reversebolt-observer` crate provides a bounded local Ollama
adapter. It accepts only ordered images,
uses an installed vision model, retains provenance and validates structured
proposals. It has no target handle, tools or write authority. Replay shows useful
actor/component proposals alongside frequent false categories and poor small
object localization. Optional automatic jobs now invoke it after selected
captures, with same-model crop filtering and durable operation ownership. See
[local-model evaluation](research/LOCAL_OBSERVER.md).

Its crop-review module takes source-bound proposal geometry without the proposed
name/category, presents source context and an enlarged crop, and retains exact
transforms and raw model output. Replay separates category filtering from box
refinement: filtering reduces unmatched proposals while refinement loses a
correct match. Same-model agreement is correlated evidence. An application
adapter publishes provisional text observations using the original coordinates;
it has no editing authority. See
[crop-review results](research/CROP_REVIEW.md).

## Observation job controller

The `reversebolt-session-jobs` crate supplies bounded durable state transitions,
request receipts, event cursors, pause/cancel and human takeover. Its application
executor accepts typed capture steps with optional local-model interpretation.
Every step pins the investigation,
batch, process creation identity and HWND, checked again by the engine before
capture. Delay is outside the durable operation intent so pausing between
captures does not manufacture a completed step.

The controller records intent before sending a capture to the sole worker.
It polls a completion channel without blocking command processing. A pause or
cancel can be acknowledged while that channel is pending. The worker's actual
outcome is recorded before releasing ownership. A timed-out renderer also
retains its slot until its thread finishes. Poll-budget exhaustion requests
drain of that owned operation, never replacement work. Storage failure fences
dispatch; a settled result may be exposed as volatile if it cannot be persisted.

On restart, the ledger is historical, the boot changes, and pending work is
interrupted without auto-resume. Exact retained command retries return the
original receipt; they do not replay captures. A durable intent without an
outcome is unknown even for read-only captures. Journal checkpoints and evidence
files are separate writes, not an atomic cross-file transaction. The library's
generic effect/consent tests do not expose production write operations.

The first local-model pass of a new multi-capture job uses a separate worker with
immutable verified PNG copies. It cannot access the target or publish findings.
The sole engine worker continues capture and serializes result publication,
checking original investigation, batch, process identity, image hash and geometry.
At most one model worker exists; the final capture drains it before starting the
final pass. That last ledger operation remains pending until analysis settles.

The controller retains a cancellation signal for the entire capture job, including
the gaps between steps. A paused/cancelled capture sequence can still own a draining
image request, explicitly exposed by `View.analysis`. Resume, scope replacement and
queued jobs wait for that request. Resume allocates a new signal without clearing
the old one. Failed background work stops later captures; an in-flight capture can
finish before the engine handles the failure. Source-bound raw receipts precede
publication; the complete observation group is validated/saved before exposure.

Status/control replies remain on the controller thread. Model steps use a 30-second
ledger poll cadence, separate from the 25 ms readiness/cancellation checks, to cover
two sequential passes at the final capture. HTTP timeout is not proof that the
separate Ollama server stopped computation. Restart resumes no inference. See
[background observation evidence](research/BACKGROUND_OBSERVER.md).

See [MCP contract](MCP.md#durable-observation-jobs) and
[integration evidence](research/OBSERVATION_JOBS.md) for limits and failure cases.

## Automatic reports and prospective checks

A successful application connection schedules one durable 16-moment report in
the same controller used by explicit observation jobs. The attachment response
returns its receipt before any capture completes. The first step has no delay;
later steps wait one second after completion. Each step pins the target and batch.
Comparison runs within the existing worker from capture 12, so partial reports
appear before the bounded pass ends. Insufficient variation remains an ordinary
report result. Stop, pause, takeover and restart interruption keep their existing
ownership rules. Manual agents can opt out with `auto_observe:false`.

`prediction` checks freeze one selected series' current coefficients and scope.
Fresh captures retain candidate bytes, frame identities and read receipts;
external interpretations can arrive later for those exact frames. Observation
agreement cannot establish authority or distinguish identical copies. A frozen
check blocks refitting until the batch changes. Geometry/process/mapping changes
and failed evidence persistence invalidate scope. The file is historical after
restart, never an active handle or write capability. Initial fitting bytes are
not retained by this check. See [evaluation](research/FROZEN_PREDICTIONS.md).

## Next architectural boundaries

The `changes` module now supplies a bounded operation journal for explicit
native trials. It retains every surviving interpretation, compares their exact
requested byte actions, persists intent before entering the write backend, and
records actual write/restoration receipts independently of the ordinary session
save. Restoring checks the complete interpretations, including unchanged bytes
around the normalized patch. Restart expires prepared plans and turns incomplete
intents into uncertain outcomes without replay. Uncertain writes hold subsequent
writes to the same process identity; there is no override or automatic recovery.

These are best-effort native guards, not target-thread exclusion or atomic CAS.
Allocation metadata cannot detect every reuse. Visible post-trial readings retain
their image hashes and optional OCR warnings, while successful byte restoration
does not establish recovery of gameplay state. See [the operation contract and
evidence](research/LIVE_TRIAL_EDITS.md). Transaction and journal failure semantics
remain distinct from the existing read-only observation-job ledger.

Trial preparation now captures a baseline and freezes up to 32 observed OCR
regions. The selected number must still agree. Post-edit/restoration observations
carry monotonic capture bounds and compare each other region with that baseline.
Ambiguous associations and missing values remain unreadable. Comparisons are
descriptive: ordinary gameplay and timing are alternative causes, and unobserved
state is not certified. Cards consume the fresh readings across all watched
values. The read-only evidence API serves hash-checked baseline/post image pairs
even after restart, retaining appearance for later interpretation without
claiming automatic visual effect detection. See
[effect-observation evidence](research/EFFECT_OBSERVATIONS.md).

1. Broader renderer support, efficient incremental snapshots and coverage planning.
2. A timestamped observation bus: screen tracks, motion, appearance, input, audio.
3. Hypotheses over objects and relationships, not two fixed scalar tracks.
4. Experiment proposals, confidence calibration, scoped write journals and
   verifiable recovery without assuming a paused whole arena.
5. Broader investigation jobs, multiple agent identities, journal lifecycle and performance budgets.
6. Learned re-identification and withheld independent-game evaluation.

These boundaries should replace practice assumptions rather than disguise them
as general capabilities. Reuse Squalr findings, but retain independent MIT code
unless a deliberate licensing decision is made.

Numeric prospective checks are now a bounded collection in `LiveView`, keyed by
source and track identity. Eight series share 200 candidates and deduplicated
capture-prefix reads, with one persisted snapshot of the collection. Automatic
selection freezes fitting results before future captures; failed extra saves do
not replace existing checks. Trial scopes bind the relevant check ID, while any
attempted write invalidates passive evidence across the collection and expires
other unapplied plans. Legacy single-check JSON is read as a historical list.
See [multiple-check verification](../research/multiple-checks/README.md).
