# Reversebolt

An open-source Rust workbench for discovering what controls an unfamiliar game.
Part of the Bolt family. Humans and agents use the same engine and session.

**Research preview:** connect a running application to start discovery automatically.
Supported IL2CPP applications provide internal-value candidates through runtime
inspection without screenshots. Other applications use the existing window and
bounded-memory observations to compare screen numbers with memory trajectories.
Values appear as simple cards; their gameplay meaning remains unverified.
Unnamed readings stay off the board. Named cards have short descriptions;
runtime descriptions are provisional metadata labels, not verified game meanings.
An installed local model can retain evidence-linked questions and competing
fields after runtime discovery. These remain shared engine/MCP investigation
hints and do not rename cards. See [automatic metadata hypotheses](research/runtime-hypotheses/README.md).
Editable numbers use a large direct-entry field with −/+ buttons; booleans use an
On/Off checkbox. Sliders require known small domain bounds, rather than an invented
range around the current number. Press Enter or leave the number field to apply.
**Freeze** periodically holds a supported internal value using fresh guarded
checks. Uncheck it to stop and leave the last value in place. It stops on reference
changes and never resumes automatically after restart; it is not a per-frame lock.
Other observation types show an unavailable Freeze control until supported.
**Undo** restores a confirmed edit when its original guards still hold. Each edit
freshly checks its target and saves its intent/result behind the single gesture.
Numeric runtime trials remain limited to adjustments of 20; gameplay effects
remain unverified. Screen-number controls need an eligible mapping first.
**Add controls** searches the retained runtime catalogue. **Show card** brings a
result onto the board. The small **i** button opens reading history, refresh/watch
actions and detailed evidence. Object labels are clues, not verified descriptions
or persistent identity. See [current controls](research/friendly-controls/PROTOCOL.md)
and [durable runtime trials](research/runtime-trials/README.md).
When a numeric finding has matches early enough in the observation pass,
Reversebolt automatically freezes its predictions and checks later readings.
It chooses by investigation effort and reading coverage, not an invented gameplay
meaning. Discovery never initiates edits; an explicitly enabled Freeze can reapply
its chosen value. Findings can remain uncertain or fail.
The main view keeps names, short descriptions and relevant actions together.
Drag a card's arrange handle to reorder it, or click the handle for accessible
move actions. Editable trial values can be dragged or typed. Detailed evidence
opens through the **i** button; application diagnostics and activity are separate
views. Card arrangement belongs to the current observation batch.
An image-reading agent can contribute provisional observations through MCP;
the desktop shows their sources and uncertainty alongside OCR.
Agents can also read ordered image sequences and contribute appearance or
behavior interpretations spanning several moments. Timing bounds and gaps stay
with the evidence; these interpretations do not establish editable controls.
For shorter sequences, agents can use visual capture mode to skip text
recognition while retaining images and memory. Actual cadence is measured;
skipped screen numbers stay missing.
An optional installed local vision model can now add provisional character,
held-item and object cards automatically, including discrete pieces and structures.
These describe visible appearance; general game understanding is not implemented.
See the [selected-object evaluation](research/visual-objects/README.md).
Some screen-number findings can now support explicit experimental memory edits;
these do not establish what the value controls in gameplay.
The disposable practice environment supports tested edits. The destination is
documented in [the full vision](docs/VISION.md), with an explicit
[progress rubric](docs/PROGRESS.md).

## Try it on Windows

Requires stable Rust and the Microsoft C++ build tools.

```powershell
cargo run -p reversebolt-app --bin Reversebolt
```

Choose **Connect to application**, search by name, title or PID, select a window,
then **Connect**. Discovery starts immediately. Supported runtimes first produce
internal-value cards. The visual/memory fallback uses a bounded 16-moment pass
while you play; comparisons begin after 12 moments. Each card
has a name, short description and current value. **Learning** means its effect
and controlling state are still unverified; the **i** button opens its details.
Use **Pause** or **Stop** at any time. **Keep looking** starts another pass over
different memory. Screenshots, activity and manual tools are collapsed behind
details. Results are unverified correlations. This preview
samples up to 16 MiB per moment, not the whole process. Capture support varies
by renderer; image/OCR failures are shown explicitly. **Disconnect** leaves the
application running. The connection invalidates when the original process exits.

For applications that keep state in another process, **Inspect application →
Memory source** lists accessible child processes. Choosing a source keeps watching
the same window and starts automatic observation in a fresh investigation.
Agents can use the same selection through MCP. The source's role in gameplay
is still unknown; automatic source selection is not implemented. See the
[process-scope checks](research/memory-sources/README.md).

Observation jobs also sample pressed keys and mouse buttons while the selected
window is active. **Inspect → Capture input context** shows a short summary;
agents receive the same timed samples with recorded frames. Pausing, stopping,
completion and disconnect stop sampling. These samples do not tell us which
inputs the game accepted or what they mean. See [input scope and evidence](docs/research/INPUT_CONTEXT.md).

Repeated input/value contrasts can add a short tentative card description.
**Inspect → Observed control patterns** shows later agreements, counterexamples,
and changes without detected keys. Agents receive the same evidence and suggested
follow-up through MCP. These are associations, not verified button meanings;
see [the method and limits](docs/research/ACTION_PATTERNS.md).

When outcomes differ, Reversebolt can also freeze competing starting-state and
action-history explanations, predict before a later sampled press, and rule out
contradicted predictions. **Inspect** keeps those details; surviving explanations
remain provisional. See [conditional predictions](docs/research/CONDITIONAL_PREDICTIONS.md).

Open **Local vision**, enter the name of a vision model already installed in
local Ollama, and choose **Use model**. Future connections and **Keep looking**
review image pairs after moments 2 and 16. The first review runs while capture
continues; each pass uses full images followed by crop checks and can still
misidentify things. Short named cards appear automatically and show when they
were seen. **Earlier** distinguishes an old image finding from
a current reading; **Inspect** retains its full explanation. No models are
downloaded and no images are uploaded. **Disable** turns this off for future
jobs. Stop or pause waits for the active request and prevents subsequent model
calls. An unavailable or rejected model pass contributes no findings and leaves
a warning under **Inspect application**; other capture continues. Evidence-save
or scope failures still stop the job. See
[integration evidence and limits](docs/research/BACKGROUND_OBSERVER.md).

For a series with matches, **Challenge these matches** freezes its predictions
and checks them against fresh observations. Agreement is limited evidence;
contradictions rule out that mapping. Neither outcome verifies a gameplay control.
Small reads bracket each fresh image. If the candidate bytes change across the
capture, that comparison stays uncertain; it counts neither for nor against the
match. Later stable captures can supply the missing evidence. Equal endpoints
still cannot rule out changes between reads or a delayed display copy.
Initial matching requires repeated value differences in both halves of the
observations, including the readings each mapping explains. A single unusual
reading cannot certify a match; rare changes may need more observation. See
[automated game checks and remaining limits](research/process-scope-study/README.md).

When a screen number has sufficient fresh evidence, its card offers **Try value**.
Drag or type a nearby value, then **Review edit**. Preparation does not write.
If every remaining interpretation proposes the same byte change, **Try edit**
explicitly attempts that experimental change and checks the screen number.
**Restore bytes** checks the complete interpretations before restoring the changed
bytes; it cannot undo all downstream gameplay effects. **Check effect** records
another reading. Details stay behind **Inspect**. Trial changes are limited to
20 from the latest observation and expire after 60 seconds. Ambiguity, changed
memory or an uncertain earlier write can prevent a trial. Connecting never
starts writes automatically. See [trial behavior and limits](docs/research/LIVE_TRIAL_EDITS.md).

When several separate locations remain plausible, **Inspect → Test a possibility**
can prepare a test of one location. Overlapping interpretations still have to
agree. The card offers **Try test** only after review; competing predictions and
their observed outcomes stay in Inspect and the shared agent API. A response
does not automatically turn the finding into a verified gameplay control.

Reviewing an edit now saves a fresh before-image and checks the selected number.
Subsequent checks also compare up to 31 other observed numbers. A card flags
other values that changed, and their cards show the fresh readings. **Inspect**
shows the comparison; agents can retrieve the original before/after images.
Changes can come from ordinary gameplay or recognition errors as well as the
edit. Unseen effects remain unknown, even when all observed numbers match.
See [effect-observation evidence](docs/research/EFFECT_OBSERVATIONS.md).

**Search more memory** archives the current batch's evidence and moves to other
eligible addresses. Collect fresh changing observations for each batch. The
coverage display distinguishes addresses read from those with complete memory
histories; neither means a gameplay control was found. Earlier findings remain
historical and unverified.

**Observe over time** collects up to 12 moments at one-second intervals while
you interact with the application. **Pause**, **Resume** and **Stop** remain
available during capture. An agent can start the same job through MCP; **Take
control** pauses its work and holds further agent changes. A capture already in
progress may finish. Jobs interrupted by restart remain in history and never
resume automatically.

For the editable demo, choose **Practice → Open practice → Observe changes →
Find matches → Test effects**.
Each practice process has new random offsets and a numeric encoding. Discovery
receives synchronized bytes and observations, not the true field locations.
Testing separates controlling fields from misleading display/telemetry copies.
When a unique candidate passes, drag the numeric field or click it to type a
value, then choose **Apply**.
**Restore last edit** checks the bytes have not changed before restoring them.

The two labels and observations come from the practice adapter. Twenty random
state changes, a supplied arena boundary, and paused whole-arena rollback make
this an intentionally easier problem than a real game.

Session history is saved under `%LOCALAPPDATA%\Reversebolt`. Previous process
addresses are invalidated on restart. `--data-dir <folder>` runs an isolated
test session; `--headless` runs the same engine without the window.

## Agent interface

`reversebolt-mcp` is a stdio MCP server using the official Rust SDK. It connects
to the running desktop's authenticated loopback API. Agent edits and desktop
edits appear in the same history. See [MCP setup](docs/MCP.md).

Agents can also run a bounded before/change/recovery comparison through the
shared engine. It retains images, sampled input and checked values while the
desktop offers **Stop test** and **Take control**. This compares an explicitly
chosen hypothesis; it does not yet plan experiments or confirm gameplay names
automatically. [Workflow and evidence](research/runtime-tests-live/README.md).

## Develop and verify

```powershell
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
python scripts/smoke.py
```

For the optional local neural text reader, run `scripts/get-models.ps1` before
publishing. The approximately 139 MB of weights are pinned by hash and included
in previews when available. See [model attribution](models/NOTICE.md).
Development builds look for `models/` in their data directory or beside the
executable. The desktop does not download models or upload screenshots.
An explicit MCP `read_capture` request returns the selected image to that
client; its subsequent processing is controlled by the client.

`scripts/publish-preview.ps1` builds versioned preview binaries, updates the
desktop launcher, and can launch the preview. Existing running versions are not
overwritten. No global MCP client settings are changed.

## Architecture

- `reversebolt-core`: general numeric matching, evidence, shared commands,
  serialized session model, verified practice edits and restoration.
- `reversebolt-app`: Windows native process transport, engine worker, local API,
  egui desktop and isolated practice fixture.
- `reversebolt-mcp`: official MCP protocol adapter to the same engine.
- `reversebolt-session-jobs`: durable observation jobs, receipts, interruption
  and human takeover; captures can include bounded local visual interpretation.
- `reversebolt-vision`: local PP-OCRv6 inference, word geometry and bounded
  temporal regions through the Rust RTen runtime.
- `reversebolt-observer`: experimental image-only local model adapter and
  crop reviewer, shared by automatic desktop/MCP jobs and offline replay. See its
  [measured results and limits](docs/research/LOCAL_OBSERVER.md).

The current practice adapter is deliberately separate from the discovery
algorithm. Native capture uses Windows Graphics Capture with a PrintWindow
compatibility path. Windows OCR and the optional PP-OCRv6/RTen reader produce
source-tagged readings. Conflicting numbers remain gaps in the matching data;
the desktop exposes their alternatives. Screen tracking is spatial, without
semantic gameplay labels.
An unsupported text region is skipped with a warning while other recognized
regions are retained. See [saved-image regression and failure isolation](research/observation-resilience/README.md).
General motion, appearance, object and rule inference,
live-game experiments, broader investigation jobs, and restart revalidation are
future milestones, not advertised capabilities.

## Research and license

Original source: MIT. Current optional PP-OCRv6 weights and dictionary are
Apache-2.0. Legacy Ocrs model installations remain separately CC-BY-SA-4.0.
Ocrs/RTen are MIT OR Apache-2.0; windows-capture is MIT. Attribution is retained
under `models/` and `licenses/`. Independent code originated in the MIT
GameDiscoveryLab research prototype.
Its measured failure cases and Squalr study are preserved under `docs/research/`.
An unchanged-product [independent browser-game baseline](research/independent-games/README.md)
records automatic 2048 and Tetris runs, including missing controls and retained
failures. These first-exposure runs do not satisfy the independent-game release gate.
Squalr was evaluated as a reference/backend candidate. It is GPL-3.0; no Squalr
source has been copied into this MIT implementation. Any future integration must
make its licensing implications explicit.
