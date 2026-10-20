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

## Available tools

| Tool | Behavior |
|---|---|
| `reversebolt_status` | Session, values, matches, evidence, edit history |
| `reversebolt_start_observation_job` | Durable timed captures with a receipt and shared progress |
| `reversebolt_job_control` | Pause, resume or cancel a current observation job |
| `reversebolt_job_events` | Read bounded, ordered job events after a cursor |
| `reversebolt_list_processes` | Application windows with title, PID, creation identity and window handle |
| `reversebolt_attach_process` | Connect an exact process/window for observation; default 16 MiB capture budget |
| `reversebolt_capture` | Window image, OCR and bounded memory snapshot from connected application |
| `reversebolt_read_capture` | A recorded PNG as MCP image content, with frame identity and checksum |
| `reversebolt_submit_observation` | Add an image interpretation with its source, uncertainty and explanation |
| `reversebolt_correlate` | Screen-number/memory hypotheses after at least 12 moments; no causal validation |
| `reversebolt_start_prediction_check` | Freeze one series' mappings and check fresh observations without refitting |
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
`window` returned by the list → watch the automatic report in status → inspect
evidence → detach. Connection starts one durable 16-capture pass with no initial
delay and one second between completed captures. Comparisons refresh from
capture 12. Attachment returns `data.automatic_observation`, its job receipt;
read the returned `jobs` for the advanced client request counter. An unsuccessful
automatic start is reported separately without undoing a successful connection.
On uncertain attachment delivery, inspect status before retrying, because a new
attachment replaces the investigation. No work resumes automatically on restart.

Use `auto_observe:false` on `attach_process` for controlled manual experiments.
`advance_memory` accepts `auto_observe:true` to start another bounded pass (default
false). A timed observation job can use `compare:true` to update its report after
each capture with enough evidence; this never refits an active prediction check.
The desktop uses the same controller and displays progress and pause/stop controls.

The `live` state is separate from the practice state; practice
write tools never edit the connected external application. A process restart
requires a new selection; PID alone is not accepted as an identity.

Only windows on the engine's Windows desktop are listed. For Anode testing,
launch the engine in Anode with an isolated `--data-dir`. A main-desktop engine
lists main-desktop applications.

## Image-grounded agent observations

There are now 21 tools. `reversebolt_read_capture` returns native MCP PNG image
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
semantics. No model is automatically invoked by Reversebolt in this preview.

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

Plans contain 1–16 read-only captures, 250–60,000 ms apart. Automatic reports and
jobs with `compare:true` start the first capture immediately; other jobs retain
the specified initial delay. Subsequent delays begin after completion. Connection
requests 16 moments at one-second intervals. Remaining retention is checked including queued
jobs. There is no gameplay input, semantic-model execution, automatic memory
batch advancement or live-game write in these plans.

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
the receipt/recovery contract above. Other commands remain synchronous and do
not acquire its idempotency guarantees. Inspect status before repeating them.

The practice transport permits writes only within the owned child's advertised
arena. Whole-arena rollback is an experiment convenience for that paused
fixture, not a rollback strategy for a real game. External process handles
currently request query/read access only. Each batch retains up to 20 moments and
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

After comparison, call `reversebolt_start_prediction_check` with the current
`investigation_id`, `batch`, `after_frame` equal to `sample_count`, `source`
(`ocr` or `external`) and the series' `track` ID. At least four capture slots
must remain. All existing mappings for that series are fixed before new captures.
Fresh observations join their original image identity; late agent submissions
can fill missing readings, but cannot change the stored predictions or bytes.

Inspect `live.prediction_check`: it contains the original mappings, image hashes,
fresh byte witnesses, read receipts, predictions, observations and outcomes.
`live.phase` gives the aggregate result. A contradiction rules out the mapping
against that observation. Four agreements including an adjacent hold and a
distinct change permit only **consistent in observed scope**. This is a
descriptive check, not statistical confidence, semantic meaning, causal authority,
or permission to edit. Missing observations do not count as agreement.

One check is allowed per batch. Correlation cannot refit its fresh interval.
Mapping changes, geometry changes, target exit and evidence persistence failures
invalidate the relevant scope. New batches archive the prior view and require
fresh discovery. `prediction-check.json` is historical evidence after restart.
The initial fitting interval's full bytes are transient; the check retains its
coefficients and fresh byte witnesses, not a complete replay of initial fitting.
