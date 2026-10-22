# Planning comparisons from observed play

The shared `plan_runtime_comparisons` command reads the current retained review,
image evidence and key samples. It proposes a four-step sequence only when it
finds four separate, visible responses with a repeated two-action pattern,
consistent timing, complete focus coverage and released input around each hold.
Durations come from sampled transition bounds. The command sends no input,
captures no new images, loads no target helper and performs no model inference.

Proposals retain exact hypothesis references. Eligible positive scalar values up
to 20 can be compared with zero; canonical true booleans with false. This is a
bounded experimental policy, not a learned range or proof that zero is useful.
Current source considers primary candidates and retained alternatives, deduplicates
only exact paths, and interleaves selected types. Separate paths to the same field
remain eligible, with unresolved aliasing made explicit. The frozen exploratory
run below used the earlier field-group deduplication and primary-only policy.
Existing
outcomes in the same evidence context are retained, with no automatic retry or
promotion to a gameplay control. Active edits, tests and freezes suppress `next`.
Preparing, observing, editing and restoring remain explicit engine operations.

The metadata model now receives a separate question budget for each selected
type, up to twelve questions in total. Each proposal may cite only that group's
rows. Existing lexical/reference checks still apply; names and questions remain
unverified. This can take up to nine metadata inference calls and does not prove
that the proposals are more useful. It does not change names on the main board.

## Frozen exploratory run

The [protocol](PROTOCOL.md), evaluator, source hashes and four binaries were
frozen before `test-results/comparison-planner-v1`. Manifold Garden build 9427366
used the existing evaluator save 3. This game and scene were explored previously;
this is not independent-game acceptance. Four evaluator D/A/D/A holds preceded
the planner; no game-specific field names or layouts were supplied to it.

The run retained fifteen automatic images and four input windows. The planner
derived holds of **199, 197, 198 and 198 ms**, within the sampled bounds. The
image model described a subtle shift but was uncertain about its mechanism.
All eight metadata question groups returned an empty list. The planner returned
no candidate and no next action. No comparison, product-executed input or
gameplay edit occurred. This is an abstention, not a successful discovery.

The subsequent owned-engine restart failed with Windows error 225. Defender
recorded `Behavior:Win32/DefenseEvasion.A!ml` against the test copy of
`Reversebolt.exe`, which is no longer present. Its pre-launch hash is retained,
but the executable cannot be rehashed after the run. No protection settings
were changed, no exclusion was added, and the blocked binary was not restored
or relaunched elsewhere. The restart/persistence check remains unverified.

The evaluator's final cleanup query failed after engine exit and left a recovery
flag. The retained pre-restart status has zero trials, tests, changes or freezes;
both owned processes and the game are confirmed exited. No restoration is needed.
One runtime helper unloaded and reported 110 pauses, maximum 5.169 ms, with no
helper heap operations while paused. User saves 1/2 and their thumbnails are
unchanged. Evaluator slot 3 and normal game logs changed. Four setup screenshots
were reviewed; no measurement images were manually reviewed or substituted.

The [artifact audit](results.json) distinguishes a passed evidence audit from
the incomplete native run. Source copies were retained after execution only
after matching every pre-launch hash. The frozen build passed 274 workspace
tests, strict Clippy, formatting, release compilation and the 53-tool smoke
before the native run. Subsequent source review corrected integer sample
handling, the conservative lower timing bound (exclude time inside the first
and last key query), and the planner/play-evidence read-only annotations. The
frozen lower bound included those query intervals and could slightly overstate
the minimum hold. These changes have separate core/MCP checks; they are not part
of the frozen game evaluation. All 158 core/MCP tests and strict workspace
Clippy pass after these corrections.

The existing desktop preview `0.1.0-20261021-225832` remains running with 44
history entries. The new build was not published. Progress stays **35%**; no
semantic, causal or generalization credit is added. Next work must establish
useful competing hypotheses and resolve the security detection without bypassing
protection before another native run or desktop update.
