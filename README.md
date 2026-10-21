# Reversebolt

An open-source Rust workbench for discovering what controls an unfamiliar game.
Part of the Bolt family. Humans and agents use the same engine and session.

**Research preview:** connect a running application, capture its window and
bounded memory, and compare changing screen numbers with memory trajectories.
Connecting starts observation automatically; values appear as simple cards.
The main view keeps names, short descriptions and relevant actions together.
Drag a card's arrange handle to reorder it, or click the handle for accessible
move actions. Editable trial values can be dragged or typed. Detailed evidence
opens through **Inspect**; application diagnostics and activity are separate
views. Card arrangement belongs to the current observation batch.
An image-reading agent can contribute provisional observations through MCP;
the desktop shows their sources and uncertainty alongside OCR.
Agents can also read ordered image sequences and contribute appearance or
behavior interpretations spanning several moments. Timing bounds and gaps stay
with the evidence; these interpretations do not establish editable controls.
For shorter sequences, agents can use visual capture mode to skip text
recognition while retaining images and memory. Actual cadence is measured;
skipped screen numbers stay missing.
An optional installed local vision model can now add provisional character and
held-object cards automatically. General game understanding is not implemented.
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
then **Connect**. Observation starts immediately. Keep playing while a bounded
16-moment pass discovers values; comparisons begin after 12 moments. Each card
has a name, short description and current value. **Learning** means its effect
and controlling state are still unverified; **Inspect** opens its details.
Use **Pause** or **Stop** at any time. **Keep looking** starts another pass over
different memory. Screenshots, activity and manual tools are collapsed behind
details. Results are unverified correlations. This preview
samples up to 16 MiB per moment, not the whole process. Capture support varies
by renderer; image/OCR failures are shown explicitly. **Disconnect** leaves the
application running. The connection invalidates when the original process exits.

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
calls. A failed pass stops its job, retaining evidence. See
[integration evidence and limits](docs/research/BACKGROUND_OBSERVER.md).

For a series with matches, **Challenge these matches** freezes its predictions
and checks them against fresh observations. Agreement is limited evidence;
contradictions rule out that mapping. Neither outcome verifies a gameplay control.

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
