# Desktop preview guide

This is a detailed guide to the development preview. Start with the current
[status and known gaps](../README.md) before interpreting a named card as a
verified control. Requires Windows x64, stable Rust and the Microsoft C++ build tools.

```powershell
cargo build --workspace --locked
.\target\debug\Reversebolt.exe
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
[process-scope checks](../research/memory-sources/README.md).

Observation jobs also sample pressed keys and mouse buttons while the selected
window is active. **Inspect → Capture input context** shows a short summary;
agents receive the same timed samples with recorded frames. Pausing, stopping,
completion and disconnect stop sampling. These samples do not tell us which
inputs the game accepted or what they mean. See [input scope and evidence](research/INPUT_CONTEXT.md).

The shared evidence reader also chooses up to four image regions from retained
visual observations and measures them against their next consecutive capture.
It needs no manually supplied coordinates and skips missing or uncertain sources.
Automatic discovery archives these measurements; region labels remain unverified
and motion results do not enable edits. See [the saved-observation regression](../research/observed-regions/README.md).
Selected regions also receive a bounded appearance search in the next image,
with duplicate-match rejection and a reverse check. This can follow a translated
appearance beyond its original rectangle; identity and gameplay meaning remain
unverified. See [controlled and saved-game checks](../research/region-appearance/README.md).
Bounded appearance sequences keep the original template fixed, retain signed
movement with capture/input context, and stop on uncertainty or missing images.
They do not reacquire an appearance after a failure or create verified controls.
See [sequence evidence and limits](../research/appearance-sequence/README.md).
With a configured local model, runtime discovery now starts one background
appearance pass after its second play capture, then settles that pass before
assembling runtime evidence. Missing images, rejected model responses and obsolete
geometry are retained as unavailable visual evidence. Metadata discovery remains
independent of whether any appearance is found. See [integration and saved-game
replays](../research/runtime-appearances/README.md).
Regional measurements now also accompany the activity interpretation for its
exact selected image pairs. Missing intervals stay explicit; the interpreter's
uncertainty is retained with the measurements. A four-call saved-image comparison
verified this connection, but did not establish better accuracy. See [regional
interpretation context](../research/appearance-context/README.md).
Traces now aim for the last selected activity image within the same twelve-match
budget. Both exposed saved-game cases consequently supply a regional measurement
for each interval; augmented interpretations flag a repeated visible change.
Object meaning and causal control remain unverified. See [activity horizon
results](../research/activity-horizon/README.md).
Agents can also compare one selected baseline appearance across a retained edit
trial's original, changed and recovered conditions. The bounded comparison keeps
the original template, checks each action separately and stops on unresolved
matches. It remains a retrospective image observation, not a verified gameplay
effect. See [regional trial comparisons](../research/trial-appearance/README.md).
An agent or desktop controller can lock that region after baseline collection
and before the linked edit. The choice remains bound to the original baseline,
target, value and action sequence, survives restart, and is used automatically
by the shared history reader. See [saved appearance choices](../research/appearance-plan/README.md).
The completed baseline also proposes a region automatically, using source
texture and quiet stability before bounded matching. A weak whole-screen
baseline can now proceed when this regional check succeeds; the engine saves
the region before any edit. Read-time proposals remain optional choices. See
[baseline region selection and its limits](../research/stable-baseline-regions/README.md).
The [regional baseline flow](../research/regional-baseline-flow/README.md) covers
this integration and its automated regression checks.
The completion response and shared history now also join the regional pattern
to recorded edit/restoration receipts and fresh value checks. Missing evidence
stays inconclusive; a supported image pattern still does not establish gameplay
meaning. See [regional intervention assessment](../research/regional-intervention/README.md).

Repeated input/value contrasts can add a short tentative card description.
**Inspect → Observed control patterns** shows later agreements, counterexamples,
and changes without detected keys. Agents receive the same evidence and suggested
follow-up through MCP. These are associations, not verified button meanings;
see [the method and limits](research/ACTION_PATTERNS.md).

When outcomes differ, Reversebolt can also freeze competing starting-state and
action-history explanations, predict before a later sampled press, and rule out
contradicted predictions. **Inspect** keeps those details; surviving explanations
remain provisional. See [conditional predictions](research/CONDITIONAL_PREDICTIONS.md).

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
[integration evidence and limits](research/BACKGROUND_OBSERVER.md).

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
[automated game checks and remaining limits](../research/process-scope-study/README.md).

When a screen number has sufficient fresh evidence, its card offers **Try value**.
Drag or type a nearby value, then **Review edit**. Preparation does not write.
If every remaining interpretation proposes the same byte change, **Try edit**
explicitly attempts that experimental change and checks the screen number.
**Restore bytes** checks the complete interpretations before restoring the changed
bytes; it cannot undo all downstream gameplay effects. **Check effect** records
another reading. Details stay behind **Inspect**. Trial changes are limited to
20 from the latest observation and expire after 60 seconds. Ambiguity, changed
memory or an uncertain earlier write can prevent a trial. Connecting never
starts writes automatically. See [trial behavior and limits](research/LIVE_TRIAL_EDITS.md).

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
See [effect-observation evidence](research/EFFECT_OBSERVATIONS.md).

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
