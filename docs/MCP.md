# MCP and the shared desktop engine

Start Reversebolt first. Build the workspace, then configure an MCP-capable
client to launch:

```json
{
  "mcpServers": {
    "reversebolt": {
      "command": "C:\\DEV\\Reversebolt\\target\\debug\\reversebolt-mcp.exe"
    }
  }
}
```

This is an example configuration, not an automatic modification of any client.
For a versioned preview use the executable inside the current `dist` version.
The MCP server accepts `--data-dir <path>` to join an isolated desktop/test
session. Both processes must use the same data directory.

The official Rust SDK handles initialization, negotiation, tool schemas, and
stdio framing. The desktop owns a single serialized engine worker, so desktop
actions and agent actions cannot interleave memory experiments. The MCP server
never creates a private simulator or duplicate session.

## Memory process selection

`reversebolt_list_memory_processes` refreshes the connected application's accessible
family and returns `data.processes`, `data.error` and a scope limitation. The same
list is in `state.live.memory_sources`, with refresh failures in
`memory_sources_error`. Inaccessible or exited ancestry branches can be omitted.

Keep the original `live.process` identity when calling `attach_process` again;
add both `memory_pid` and `memory_created` from the refreshed inventory. The
window, image and input source remain the original process, while every memory
region, address, candidate and trial belongs to `live.memory_process`. Omit both
optional fields to use the window process. Do not combine addresses from two
sources or reuse old jobs/matches: attachment starts a new investigation and
requires fresh evidence. Default automatic observation is unchanged. One process
is sampled per investigation; no ownership or semantic role is inferred from
ancestry. See [native checks and limits](../research/memory-sources/README.md).

## Available tools

Observation jobs now attach optional `input_context` to recorded frames. The
same object is available through `status.state.live.frames`, `read_capture` and
`read_sequence`; no new tool or separate agent session is needed. It records
bounded polls on the investigation's monotonic clock, with a fresh generation
after pause/resume. `down` contains sorted Windows virtual-key codes, and only
`selected_foreground` polls may contain keys. `outside_selected` and `unavailable`
are not idle observations. Zero key states, polling gaps, missing context,
omitted polls and retired history do not establish that no input occurred.
The sampler generates no game input and stops with its owning job. Agents must
not interpret physical key state as a delivered event, a continuous hold or a
gameplay meaning. See [the contract and native evidence](research/INPUT_CONTEXT.md).

| Tool | Behavior |
|---|---|
| `reversebolt_status` | Session, values, matches, evidence, edit history |
| `reversebolt_start_observation_job` | Durable timed captures with a receipt and shared progress |
| `reversebolt_job_control` | Pause, resume or cancel a current observation job |
| `reversebolt_job_events` | Read bounded, ordered job events after a cursor |
| `reversebolt_list_processes` | Application windows with title, PID, creation identity and window handle |
| `reversebolt_list_memory_processes` | Refresh the connected window process's accessible descendants and exact identities |
| `reversebolt_attach_process` | Connect an exact process/window for observation; default 16 MiB capture budget |
| `reversebolt_set_observer_model` | Set or disable the installed local vision model used by future automatic jobs |
| `reversebolt_capture` | Window image, OCR and bounded memory snapshot from connected application |
| `reversebolt_read_capture` | A recorded PNG as MCP image content, with frame identity and checksum |
| `reversebolt_submit_observation` | Add an image interpretation with its source, uncertainty and explanation |
| `reversebolt_read_sequence` | 2–8 ordered recorded PNGs with timing bounds, hashes and gaps |
| `reversebolt_submit_temporal_observation` | Provisional appearance, screen-motion or change interpretation across several images |
| `reversebolt_correlate` | Screen-number/memory hypotheses after at least 12 moments; no causal validation |
| `reversebolt_start_prediction_check` | Freeze one series' mappings and check fresh observations without refitting |
| `reversebolt_prepare_change` | Prepare a bounded OCR-number trial; no process write |
| `reversebolt_prepare_probe` | Review a test of one candidate location, retaining overlapping interpretations and rival predictions; no process write |
| `reversebolt_apply_change` | Explicitly attempt the prepared native write and record a screen-number check |
| `reversebolt_restore_change` | Restore original bytes only while the original scope and complete interpretations still match |
| `reversebolt_observe_change` | Add a source-hashed post-trial reading without writing process memory |
| `reversebolt_read_change_evidence` | Read the before-image and one recorded post-edit/restoration image, with hashes and comparisons |
| `reversebolt_advance_memory` | Archive a memory batch, release its raw snapshots and select new addresses |
| `reversebolt_detach_process` | Release the connection and snapshots; leave application running |
| `reversebolt_open_practice` | New randomized disposable target; optional reproducible seed |
| `reversebolt_observe` | Twenty observation/memory samples |
| `reversebolt_discover` | Numeric correspondence hypotheses with held-out validation |
| `reversebolt_test_candidates` | Two interventions per match, scope checks, practice rollback |
| `reversebolt_set_value` | Change a uniquely supported control and check the observation |
| `reversebolt_undo` | Restore last current-session edit if its bytes still match |
| `reversebolt_history` | Shared recorded activity and patches |
| `reversebolt_close_target` | Close owned target and invalidate controls |

Typical investigation: status → open practice → observe → discover → inspect
candidate evidence → test candidates → inspect remaining ambiguity → set value
→ undo. A protocol response may contain `isError: true`; do not treat a delivered
tool call as a successful edit.

Application investigation: list processes → attach using `pid`, `created` and
`window` returned by the list → watch automatic discovery in status → inspect
evidence → detach. Connection starts one durable 16-capture pass with no initial
delay and one second between completed captures. Comparisons begin at
capture 12. Attachment returns `data.automatic_observation`, its job receipt;
read the returned `jobs` for the advanced client request counter. An unsuccessful
automatic start is reported separately without undoing a successful connection.
On uncertain attachment delivery, inspect status before retrying, because a new
attachment replaces the investigation. No work resumes automatically on restart.

Use `auto_observe:false` on `attach_process` for controlled manual experiments.
`advance_memory` accepts `auto_observe:true` to start another bounded pass (default
false). A timed observation job can use `compare:true` to update its findings after
each capture with enough evidence; this never refits an active prediction check.
The desktop uses the same controller and displays progress and pause/stop controls.

With at least four future captures scheduled in that same job and available in
the batch budget, comparison automatically selects up to eight currently readable
OCR series with matches and starts separate prospective checks. Ranking prefers fewer
transitively overlapping location groups, then more readable moments, fewer
interpretations and finally track ID. This estimates investigation effort, not
gameplay relevance or confidence in a control. Labels never affect the choice.
Selection and ranked evidence are persisted in
each `state.live.prediction_checks[].automatic_selection` before later captures occur.
`planned_tracks` records the selection plan; the actual checks list records which
starts succeeded. All checks share 200 candidate interpretations in total. A
series that does not fit is deferred whole; its competing explanations are never
discarded merely to fit the budget. Identical address/width reads are shared.
Those captures challenge immutable mappings; they do not refit them. No trial
preparation or write is automatic. Pause/stop still bounds the job; a scheduled
capture is not a promise it will happen or provide useful contrast.

Each card uses the check matching its source and track ID. Existing explicit
checks are preserved. For agent-selected series, use manual attachment
and a job with `compare:false`, then `correlate` and `start_prediction_check`.
Old job plans without a remaining-capture count do not start automatic checks.

`capture` and `start_observation_job` accept `mode: "full"` (default) or
`mode: "visual"`. Full mode includes text recognition. Visual mode keeps the
window image and bounded memory but skips OCR, leaves number readings missing,
and performs no automatic later recognition. Existing image/temporal submissions
remain usable; the default connection flow continues to use full mode.

Visual jobs allow `interval_ms: 50` and start the first capture immediately.
Full jobs require at least 250 ms; both allow up to 60,000 ms and 1–16 steps.
Intervals are delays **after each completed capture**, not frame-rate promises.
Both modes share the batch's 20 moments, 512 MiB retained-memory ceiling and
existing image/read budgets. `compare:true` is rejected with visual mode;
submit any interpretations and compare separately when there is enough evidence.
The mode is part of the immutable job plan, so a retry cannot switch modes.
Old plans without a mode retain full recognition and their original retry identity.
Capture metadata records the chosen mode explicitly.

The `live` state is separate from the practice state; practice
write tools never edit the connected external application. A process restart
requires a new selection; PID alone is not accepted as an identity.

Only windows on the engine's Windows desktop are listed. For Anode testing,
launch the engine in Anode with an isolated `--data-dir`. A main-desktop engine
lists main-desktop applications.

## Explicit experimental changes

`prepare_change` takes `investigation_id`, `batch`, an OCR `track` ID and a
requested numeric `value`. It requires current frozen predictions with a fresh
hold and change, excludes contradicted interpretations, and retains all other
alternatives. Missing/invalid evidence blocks preparation; different proposed
byte actions produce an unavailable plan. No arbitrary address is accepted.
The requested difference must be greater than 1.1 and no more than 20. Preparation
does not write, and does not imply an instruction to call `apply_change`.

`prepare_probe` additionally takes the exact `prediction_id` and zero-based
`candidate` index from that check. This explicitly tests one location when
different locations remain plausible. Every transitively overlapping surviving
interpretation must have fresh evidence and propose the same patch. Unresolved
overlaps cannot be discarded. Nonoverlapping rivals remain in `change.probe`,
including unreadable alternatives with explicit errors and conditional
`predicted_if_written` values. The same preparation, expiry, journal, application
and restoration guards apply. No address argument or automatic write is added.

After a written probe, `observations[].probe_comparisons` compares the observed
number with each alternative's conditional prediction (`agreed`, `differed`, or
`unavailable`). These comparisons do not eliminate hypotheses or establish causal
authority: effects can propagate between locations, and reads are sequential.
Restored observations do not reuse predictions for the written patch. On desktop,
the selection is under the card's **Inspect → Test a possibility**; applying the
reviewed test remains an explicit card action.

`apply_change`, `restore_change` and `observe_change` take the returned
`change_id`. The first two are annotated destructive and open-world because they
change the selected application's process memory. They obey the same human hold
and observation-job ownership checks as other engine mutations. The current
backend only writes 1–8 bytes in unchanged private PAGE_READWRITE allocations;
it does not change page protections, suspend threads, inject code or bypass
access controls. The expected-byte checks are not an atomic transaction with
the game. A plan expires after 60 seconds or a scope/capture change.

Native receipts appear in `data` and durable `state.changes`. `written` means
the requested bytes were observed after the native write; `restored` means the
original bytes were observed again. Neither verifies gameplay meaning or all
effects. `uncertain` is a held outcome, never permission to retry. Repeating an
already completed change ID does not write again; it may add a visual check.
Inspect a failed response and the recorded state before deciding what happened.

Expected-byte conflicts retain a refusal receipt with `attempted: false`, zero
transferred bytes and the reason in `error`. A refused restoration is recorded
in `restoration` while preserving the original `write` receipt and `written`
phase. That phase describes the earlier write, not a successful restoration.
An absent receipt is not proof of success; older builds did not record receipts
for preflight conflicts. Inspect `reason` as well, and never infer restoration
from a successful tool response alone.

Preparation captures a fresh before-image and checks that the selected number
still agrees. Failure makes the plan unavailable. It freezes at most 32 active
OCR regions, including the selected number, and records any omitted track count.
The automatic post-write check and `observe_change` store a PNG hash, monotonic
capture-operation interval, stage, selected reading and the other region readings.
Each post-reading compares those same regions against the before-image:
changed, unchanged within 0.55, or unreadable. Missing/ambiguous readings never
become zero or proof of stability. Unexpected numeric regions are counted but
not assigned an invented identity. Normal gameplay and timing can produce changes;
this is descriptive evidence, not causal attribution or a complete effect scope.

Each post-edit/restoration observation now also contains `intervention_memory`:
complete expected interpretation bytes and separate before/after reads around
the capture operation, with monotonic intervals, allocation checks and errors.
It reads at most 200 interpretations of at most eight bytes twice (3,200 bytes
per observation), separately from discovery snapshots. Sequential reads do not
establish atomicity or allocation-generation identity. Equal endpoints cannot
exclude ABA changes or display lag.

`intervention_outcome` distinguishes `matched`, `did_not_follow`, `memory_changed`,
`memory_unavailable`, `timing_uncertain` and `visual_unavailable`. The first two
require expected complete bytes at both endpoints and a reliable visual reading.
If bytes have changed or evidence is unavailable, conditional `probe_comparisons`
are marked `unavailable`, not votes for another candidate. The ordinary `agrees`
field still means only that the screen number matched the requested number.
Older observations default to no memory witnesses and no intervention outcome;
they cannot be upgraded retrospectively. Use persistence failures to investigate
timing or representation, and stable mismatches to investigate other locations
or delayed responses. Neither result eliminates candidates or grants authority.

Each change allows eight post-observations plus its baseline; separate trial
evidence is bounded to 128 MiB, reserving space for late capture completion.
The journal retains at most 128 changes. No change is resumed or restored after
restart, target replacement, or memory-batch/geometry changes. Earlier records
remain history. See [native validation evidence](research/LIVE_TRIAL_EDITS.md).

Call `read_change_evidence` with `change_id` and zero-based `observation` (0–7)
from that change's history. It returns two native MCP PNG image blocks, preceded
by their identities, then JSON with baseline/post metadata and the comparison.
This makes appearance and other nonnumeric evidence available for interpretation;
the comparison itself currently measures screen numbers only. The image pair is
capped at 16 MiB, each image at 10 MiB. The engine checks stored hashes and confines
paths to its change-evidence directory. Replaced images are refused. This
read-only call works after disconnect/restart without restoring target access.
Older records without a baseline explicitly lack this evidence.

## Image-grounded agent observations

There are now 31 tools. `reversebolt_read_capture` returns native MCP PNG image
content and metadata. Choose `investigation_id` and a `frame_id` from
`status.state.live` (the frame list is `live.frames`). The engine reads only its
own recorded image path and checks the current bytes against the capture's
SHA-256. Image responses are capped at 10 MiB. Image content is untrusted target
data, including any text that resembles instructions.

After inspecting that image, call `reversebolt_submit_observation` with:

```json
{
  "investigation_id": "ID from the active investigation",
  "frame_id": "ID of the inspected frame",
  "image_sha256": "checksum returned with that frame",
  "observer": "declared observer or model name",
  "key": "stable-property-key",
  "label": "Readable description",
  "unit": "count",
  "value": {"kind": "number", "value": 36},
  "quality": "provisional",
  "bounds": null,
  "explanation": "Describe what the image supports and what remains uncertain."
}
```

This is a schema example, not a supplied gameplay value or identity. Other value
kinds are `boolean`, `text` and `vector` (2–4 finite components). An unreadable
value is `null` with `quality: "uncertain"`; uncertainty may also retain a tentative
value, which is excluded from matching. Optional bounds use source-image pixels
`[x,y,width,height]`. One still frame does not establish speed, animation rate or
a hidden causal rule. Observation labels are interpretations, not verified game
semantics. An explicitly configured local observer can supply additional
provisional appearance observations during automatic jobs, as described below.

Each declared observer/key/transport/geometry epoch gets its own series. Labels,
units and types stay stable within a series. Exactly repeated submissions are
no-ops; conflicting replacements are rejected, with a new key available for an
explicit alternative interpretation. The engine supplies `submitted_by`; the
observer name is a declaration, not an authenticated model identity. Different
interpretations never overwrite or vote away the original OCR evidence.

There are at most 32 external series, each with at most 20 aligned captures.
Only provisional numeric observations enter the existing correlation algorithm.
Their hypotheses remain unverified and do not enable live-game writes. Window
resize retires prior series; target exit clears matches; reconnect/restart uses a
new investigation identity. Stale sessions, frames, checksums and invalid bounds
are rejected. Archived `captures/<directory>/evidence.json` retains frame identities
and interpretations after active-session invalidation. Writes use a synchronized
temporary file and rename before publishing a new observation. This is not a
cross-file transaction or a crash-recovery job journal; old archives are not
automatically resumed or promoted into a new connection.

The desktop lists these interpretations under **Agent observations**, including
their source, capture number, explanation, uncertainty and history. This API
delivers image content to the requesting client; no background image uploads or
global MCP configuration changes are made.

## Observations across several images

Use `reversebolt_read_sequence` with `investigation_id` and `frame_ids` (2–8
distinct, chronological IDs from the current batch). The response alternates
frame labels and native PNG blocks, followed by the shared state JSON. Its
`data.sequence` contains the matching frames, monotonic clock identity,
`elapsed_us: [minimum, maximum]`, `largest_gap_us` and `omitted_moments`.
Images must share geometry and clock; combined image bytes are capped at 16 MiB.
The capture intervals bound engine operations, not exact game presentation.

After inspecting the sequence, `reversebolt_submit_temporal_observation` takes:

- `investigation_id`, current `batch`, declared `observer`, stable `key`, `label`
  and `unit`.
- `basis`: `appearance`, `screen_motion` or `observed_change`.
- `frames`: ordered objects containing `frame_id`, `image_sha256` and `bounds`
  (null or `[x,y,width,height]` in that image).
- `value`, `quality` and `explanation`, with the same typed value forms as
  single-image interpretations.

The engine derives timing from recorded frames; the observer cannot supply a
different clock or elapsed interval. It retains gaps and verifies every image
before accepting the declaration. An exact retry is a no-op. A different
declaration ending at an occupied frame needs a new key. Late observations are
sorted by their ending frame. There are 16 series and 20 observations per series
per batch. The current state exposes `live.temporal_tracks`; evidence persists
in `temporal-evidence.json` and the batch archive.

These series remain descriptive and do not enter numeric memory matching.
Screen motion is not world motion; several snapshots do not prove continuous
identity, a path, animation frequency, or causality. Use uncertainty for gaps
or ambiguity. The desktop shows cards and explanations, with older observations
distinguished from current readings. Submission respects human takeover and
can occur between automatic captures, never concurrently with a capture.
See [implementation and verification](research/TEMPORAL_EVIDENCE.md).

## Durable observation jobs

Read `status.jobs.history` after connecting an application. It contains `boot`,
`current` (the exact binding), `clients` and `event_head`. Start a job with:

```json
{
  "request_id": 1,
  "boot": 1,
  "binding": {"session": "from status", "target": "from status", "generation": 1},
  "count": 12,
  "interval_ms": 1000
}
```

These are schema examples; use the actual current values. The next request ID
is `clients["mcp-agent"].high_water + 1`, or 1 for a new client. All MCP callers
currently share this client identity and must serialize their job requests.
The adapter assigns agent provenance; supplying `source: "desktop"` in HTTP
does not grant human control. Declared observer names are unrelated to authority.

A successful reply's `data.ack.job` identifies the job. The acknowledgement is
a durable receipt, **not completion**. Inspect `jobs.history.jobs[id].phase`,
`next_step`, `checkpoints` and `jobs.in_flight` for current progress. On a lost
reply, retry the exact original arguments and ID. A retained exact retry returns
the historical receipt even after rebind/restart; it does not perform the action
again. Changed contents with the same ID are rejected. Evicted IDs cannot be
used to create replacement work. Business rejections saved in `data.ack` consume
the ID; validation before journaling may not. Refresh status rather than guessing.

`job_control` takes the next request ID, current boot and binding, plus `job`
and `operation: "pause" | "resume" | "cancel"`. Pause/cancel prevents the next
capture; an owned capture settles before `Pausing`/`Cancelling` becomes terminal
or paused. Cancelling does not remove earlier evidence. Calls that alter the
investigation are held during active jobs. Image interpretations can still be
submitted between captures. Status and job controls remain responsive.

Desktop **Take control** transfers active agent jobs to the human, pauses them
and holds further agent mutations. **Release control** allows new agent work;
it does not resume paused jobs or transfer their ownership back. The human can
resume or stop those jobs. The desktop also has a reserved control admission
lane, preserving order within each source.

The binding pins the investigation, memory batch, process creation identity and
window. Replacement, exit and restart invalidate work. Restart increments boot,
unbinds the target and leaves unfinished jobs `Interrupted`; a pending durable
intent has an unknown outcome. There is no automatic continuation, rediscovery
or replay. Old jobs remain historical. Create a new job after reconnecting.

Plans contain 1–16 read-only captures with post-completion delays of 250–60,000 ms
in full mode or 50–60,000 ms in visual mode. Automatic discovery passes and
jobs with `compare:true` start the first capture immediately; other jobs retain
the specified initial delay. Subsequent delays begin after completion. Connection
requests 16 moments at one-second intervals. Remaining retention is checked including queued
jobs. There is no gameplay input, automatic memory
batch advancement or live-game write in these plans.

`reversebolt_set_observer_model` accepts `model: "installed-tag"` or `null` to
disable local inference. It checks the installed local Ollama vision model and
persists the choice for future automatic connections and batch advances. It
never downloads weights, changes an existing job, or starts inference itself.
The desktop **Local vision** setting uses the same command.

Explicit `start_observation_job` requests can include `local_model`; omission
keeps capture-only behavior. A model requires at least two captures. Its name is
pinned in the durable step payload after capture 2 and the final capture; actual
digests, prompt/schema/options, image hashes and crop transforms are retained in
each pass receipt. Automatic 16-moment jobs use the configured choice at moments
2 and 16. No target identity, memory or annotations enter the model prompt.

The first pass of a new job with more than two captures runs on one background
worker using immutable image copies; later captures continue on the engine
worker. The final capture drains any first pass, then runs its own pass before
the job completes. Two-capture jobs wait for their single pass. Legacy serialized
plans without `background_model` retain their blocking behavior.

`state.analysis` identifies the outstanding worker's job, evidence id, model,
investigation, batch, source frame ids and observation/start times. It is null
once the worker settles. Capture progress and analysis progress are distinct:
a job may be `Paused` or `Cancelled` while `state.analysis.stopping` is true.
Resume, target replacement and other mutations wait for that worker to settle;
read-only requests and declared image-observation submissions remain available
between engine operations. No queued job takes over a draining worker's scope.

Pause, cancel and desktop takeover signal the active pass even between capture
steps. The current HTTP request may finish, but later calls and publication are
skipped when the flag is observed. Already completed publication is not undone. A pass
checks a 90-second budget between requests (an active request can add up to
90 seconds plus bounded metadata checks). An inference/validation failure whose
receipt was saved contributes zero findings and returns a `local_observation.warning`;
other capture steps continue, with no retry of that pass. The next separately
scheduled review can still run. `state.live.local_observation_warnings` retains
up to 32 notices for the current memory batch, including evidence id, owning job,
source frame ids and message. These notices are not observations or permission
to edit. A successful job does not imply every optional review succeeded.
Evidence persistence, scope, publication or worker failures still stop the job;
background failure is checked before the next capture starts. An already-running
capture can finish before the failure is handled.
HTTP timeout means the client stopped waiting, not proof of
server quiescence. Raw receipts are saved under `captures/model-observations/`.

Accepted items are image-bound `provisional` text observations from engine source
`local-model`, with generic names and original proposal coordinates. They receive
text categories `Character`, `Held object` or `Object`. Discrete non-character
objects and structures now pass through the same proposal/crop-review path;
HUD text and diffuse backgrounds are not object findings. Categories and
descriptions remain model claims. See the [selected-object checks](../research/visual-objects/README.md).
They receive new keys per pass, making no persistent identity claim. Late findings retain
their original frame slot; later slots remain missing. The desktop shows their
capture age. Restart discards the transient worker state and resumes no inference.
They do not generate
numeric memory matches or enable writes. See
[background integration evidence](research/BACKGROUND_OBSERVER.md).

The job ledger retains at most 24 jobs, 8 queued jobs, 8 receipts per client and
128 events. `job_events` returns up to 64 events after its `cursor`; use `next`
until reaching `head`. An expired cursor requires a status refresh and restart
from `event_head`; this explicitly skips the missing historical interval. A
future cursor is rejected. Cursors belong to one ledger session.

`jobs/jobs.wal` is append-only, checksummed and synchronized, under an exclusive
lock. The ceiling is 64 MiB, with 2 MiB records and reserved outcome/recovery
space. Incomplete final records can be repaired; complete corruption stops
loading. Storage uncertainty fences new job dispatch. Preserve that ledger;
rotation/migration and user-facing recovery are unfinished. This is not an
exactly-once guarantee or a transaction with the separate capture archives.
After the bounded poll budget, a slow capture retains ownership until it
actually settles; no replacement capture is launched behind it.

## Local transport and limits

To search beyond the initial memory window, call `reversebolt_advance_memory`
with the current `investigation_id` and `batch` from `live.coverage.batch`.
This archives the current observations, hypotheses and up to 256 candidate byte
witnesses, releases its raw memory and resets capture/track lists. Inspect or
compare the current batch before advancing if its findings are still needed.
The new batch needs fresh changing observations. Replaying an old batch number
is rejected; inspect status after an interrupted call before retrying.

`live.coverage` separates unique selected/read addresses, complete trajectories,
actual requested reads and current analyzed bytes. `coverage.batches` contains
historical summaries and local archive paths. These counts span different times
and do not prove object identity or current editing authority. Old frame IDs
cannot be submitted into the new batch. The earlier PNGs and manifests remain
on disk, but the MCP image tool reads the active batch only.

Live `status` includes source-tagged screen words and per-track `readings` with
raw alternatives. A `null` sample can mean missing or conflicting recognition;
check the corresponding reading's reason. Agreement remains provisional, and
all memory matches remain unverified hypotheses. Retired tracks have
`active: false` after a window resize. Capture metadata reports the rendering
method and separates screen, memory and recognition timings. The desktop and
MCP use these same observations.

The app binds an ephemeral port on `127.0.0.1`; endpoint and random bearer token
are stored in its user-local data directory. The adapter checks the loopback
host, disables redirects and proxy routing, and reloads endpoint information for
each call. HTTP requests require the token and reject browser Origin headers.
The token protects against accidental unauthenticated clients, not other
software with access to the same user's files.

A file lock prevents two engines from owning one data directory. History
persists; active addresses do not survive engine restart. Observation jobs use
the receipt/recovery contract above. Native trial operations have their separate
change-ID journal and never replay uncertain writes. Other commands remain
synchronous and do not acquire those idempotency guarantees. Inspect status
before repeating them.

The practice transport permits writes only within the owned child's advertised
arena. Whole-arena rollback is an experiment convenience for that paused
fixture, not a rollback strategy for a real game. Observation process handles
request query/read access only; explicit trial operations temporarily request
write access and use the guards described above. Each batch retains up to 20 moments and
512 MiB of raw memory; images remain in the data directory. The optional attach
budget is 1–32 MiB per moment. The desktop uses 16 MiB.

An investigation allows at most 24 batches, 4 GiB of requested reads, 512 MiB of
image storage allowance (including late-renderer reservations) and 32 MiB of
batch archives. Per-frame metadata is separate. Batches are explicit actions;
there is no automatic batch advancement, restart resume, or guarantee of whole-process
coverage within these limits. Observed mapping changes exclude affected chunks
for the rest of a batch; same-address reuse between checks remains unverified.

Capture reads screen and memory sequentially without pausing the target.
Unavailable pages are reported and excluded from correlation; coverage and
timing are visible. A window-rendering timeout preserves memory evidence.

## Frozen prediction checks

`live.rule_checks` separately exposes automatic conditional outcome checks.
Each has a discovery scope/boundary, immutable candidate feature/threshold ranges,
training episodes/values and per-image `forecasts`. A forecast identifies its
baseline image, prior bounded-episode count and one prediction slot per candidate.
`issued_us` is computation completion in the investigation's monotonic clock.
Current-image declarations may create a new forecast for an updated episode
count; old forecasts remain unchanged. Unknown contexts are not backdated.

`challenges` retain the observed episode, `first_pressed_us`, and candidate
comparisons (`agreed`, `contradicted`, `unknown`). An agreement requires a known
prediction prepared strictly before the first sampled press. Candidates inside
an unresolved threshold gap abstain. A survivor is not a verified mechanism.
Use a reachable context that separates alternatives; the engine never injects
input or authorizes writes on this basis. `live.rule_error` stops this batch's
updates on evidence-save failure. See [the contract](research/CONDITIONAL_PREDICTIONS.md).

`live.action_patterns` separately contains automatic descriptive input/value
contrasts for active `ocr` and `external` series. Intervals reference exact frame
IDs and retain endpoint values, elapsed-time ranges and exclusion reasons.
Episodes merge contiguous sampled exposures; a bounded key episode needs
no-key-detected observations on both sides. Pattern indices refer to the containing
evidence's episodes, and episode interval indices refer to its intervals.
`first_episodes`, `later_agree`, `later_disagree`, `no_keys_unchanged`,
`no_keys_changed` and `unbounded_key_episodes` preserve the comparison counts.
`next_observation` suggests a follow-up; it is not an automatic action or permission
to inject input. These comparisons can be revised by late image declarations;
they are not the frozen prospective memory checks below. New input generations,
geometry and memory batches require fresh patterns. No state grants edit authority.
See [input/value contrasts](research/ACTION_PATTERNS.md).

Initial numeric comparison requires repeated contrast in both temporal halves:
the second-highest minus second-lowest observed value must exceed 2.2, and the
inliers explained by a mapping must independently meet that requirement in
each half. The existing 0.55 tolerance and 80% inlier minimum still apply.
An isolated event, or contrast supplied only by rejected outliers, yields no
match. This is a minimum evidence rule, not a confidence or authority score.

After comparison, call `reversebolt_start_prediction_check` with the current
`investigation_id`, `batch`, `after_frame` equal to `sample_count`, `source`
(`ocr` or `external`) and the series' `track` ID. At least four capture slots
must remain. All existing mappings for that series are fixed before new captures.
Fresh observations join their original image identity; late agent submissions
can fill missing readings, but cannot change the stored predictions or bytes.

Inspect `live.prediction_checks` by source and track, not list position. Each
check contains the original mappings, image hashes,
fresh byte witnesses, read receipts, predictions, observations and outcomes.
Each fresh sample includes `before_image`, a small candidate-byte read and its
receipt before requesting the image. The ordinary post-image memory scan is
the other endpoint. Equal raw bytes, complete reads and correctly ordered bounds
on the image's clock are required for an agreement or contradiction. Otherwise
the outcome is `timing_uncertain` (or `missing` if the later memory value is
unavailable). Late external observations cannot override that timing decision.
Historic samples without a prefix witness do not count toward a new trial.

`latest.prediction_reads` records these extra reads, charged against the same
4 GiB investigation budget before capture starts. At most 200 unique candidates
of up to eight bytes each are read per fresh image; repeated address/width pairs
are deduplicated. `latest.screen_started_unix_ms` and the frame's wall-clock
start identify the image request after those reads; `latest.started_unix_ms`
still marks the overall capture. Read receipts share the monotonic image clock.
Equal endpoints cannot exclude intervening changes, stale rendered frames or
display copies. Continuously changing values can remain uncertain; these are
not synchronized game-state snapshots.

`live.phase` gives the aggregate result. A contradiction rules out the mapping
against that observation. Four agreements including an adjacent hold and a
distinct change permit only **consistent in observed scope**. This is a
descriptive check, not statistical confidence, semantic meaning, causal authority,
or permission to edit. Missing observations do not count as agreement.

Up to eight checks share a 200-candidate budget per batch. Freeze additional
series at the same capture count before gathering fresh evidence. A retry cannot
replace a check for the same source/track; a new series cannot bypass an
invalidated check. Correlation cannot refit any frozen interval. An attempted
trial write invalidates every passive check, since effects may cross values.
Previously prepared, unapplied plans backed by those checks expire before another
write can enter the backend. Completed apply calls remain idempotent and guarded
restoration remains available; invalidation does not authorize forcing an undo.
Mapping changes, geometry changes, target exit and evidence persistence failures
invalidate the relevant scope. New batches archive the prior view and require
fresh discovery. `prediction-checks.json` (schema version 2, `checks` list) is
historical evidence after restart. Current status publishes only the plural
`prediction_checks` field; saved legacy views with a single `prediction_check`
object or null still load without granting a new process capability.
The initial fitting interval's full bytes are transient; the check retains its
coefficients and fresh byte witnesses, not a complete replay of initial fitting.
