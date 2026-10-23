# Independent Vampire Survivors baseline

The unchanged published preview `0.1.0-20261023-070135` failed runtime discovery
on the installed Vampire Survivors build. The native report contains only
`field metadata budget`; no runtime catalogue, named control, comparison or
gameplay edit was produced. Progress stays **38%**, with generalization **0/10**.

The game had not appeared in the project's earlier recorded evaluations. The
[protocol](PROTOCOL.md) and [freezer](freeze.py) retained 232 source files, all four
published binaries, the installed game/runtime hashes, model identity and local
storage before launch. The installed Steam manifest reports build `25016043`.
No game update, source inspection, decompilation, field selection, prompt change,
threshold change or evaluator modification was used for this baseline.

## Procedure and retained outcome

Hidden Anode session 3 launched the game and a fresh shared engine. Two Enter
presses passed the initial screens; the offered Quick Start option selected the
game's defaults. Six setup images were viewed. There were seven screenshot tool
requests in total: the first result was truncated before it could be viewed.
No measurement image was manually viewed.

The unchanged `research/independent-runtime/run.py` attached once, collected 15
automatic images and sent the planned four 200 ms D/A/D/A probes. Sampled input
records contain those four episodes, foreground ownership and no omitted polls.
They do not prove delivery to gameplay or movement effects.

The automatic image interpretation described a level-up menu in its first two
frames. One proposal and two focused review requests completed using the frozen
local model; this is same-model, unverified interpretation. No menu intervention
or scene rescue was made after the investigation started. This run consequently
also fails to establish the intended movement baseline.

The observation job ended **Failed**, even though the evaluator itself exited
normally after retaining the result. Native inspection completed in **8.367 s**
and confirmed helper unload. Metadata naming never started; there were no
runtime trials, comparisons, freezes or unresolved jobs to recover.

## Diagnosis and next implementation

`load_class` in `crates/reversebolt-runtime/src/lib.rs` rejects either a cumulative
200,000-field limit or a 20-second elapsed limit with the same message. The whole
helper finished in less than nine seconds, implicating the field-count cap. This
is an inference from source and timing: the error report retained neither the
actual field count nor pause telemetry.

The catalogue currently loads every declared class and field before bounded root
traversal. Root pagination therefore cannot help when this earlier stage runs out
of budget. This is a general scaling problem, not evidence for a game-specific
field list. The next change should provide bounded metadata acquisition with
explicit coverage, stable continuation and fresh typed-path checks. Preserve
failure counters and the exhausted budget, and test large synthetic catalogues
and boundary conditions before another separately frozen native evaluation.
Separately, an active comparison needs a baseline that distinguishes gameplay
from a menu transition; increasing a metadata budget cannot solve that issue.

The published build and original failure remain unchanged. No successful retest
or compatibility claim is implied by this diagnosis.

## Audit and storage

Run `python -X utf8 research/independent-vampire-survivors/audit.py` to verify the
retained evidence without starting a game, viewing images or calling a model.
The [audit](audit.py) checks frozen/current sources and binaries, game hashes,
every image, sampled input episodes, model prompt hashes and identity, the native
failure/unload, empty edit history and cleanup. [Results](results.json) contain
the derived summary. Full local artifacts remain in
`test-results/independent-vampire-survivors-v1` and are not committed.

The existing Roaming save and its backup are byte-identical. The Steam
`remote/SaveData` and `remotecache.vdf` changed during ordinary launch/play/exit;
their content was not interpreted and complete save rollback is not claimed.
Both before and after copies are retained locally. New LocalLow files comprise
the player log, preferences and shader analytics. Nothing was restored or
overwritten to improve the result.

Owned game, engine, MCP and evaluator processes exited, the Anode lease was
released, and the user's existing editor remained responsive. No product source
changed, so no new desktop build or duplicate Rust test run was needed. The
artifact audit passed; this records a failed independent trial, not progress in
verified game generalization.
