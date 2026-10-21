# Watch effects across state updates — 2026-10-21

Trial cards now offer **Watch effect** and **Watch restoration**, backed by the
same `start_change_watch` command for agents. A watch collects paced observations
while a human or external agent continues gameplay. It preserves immediate
nonresponses and later responses instead of relying on one image after a write.
The updated desktop preview is `0.1.0-20261021-122703`. Overall progress remains
**31%**: this adds a tested investigation workflow, not verified general game
controls or independent-game acceptance.

## Product behaviour

A watch is a durable, read-only job bound to one trial, its current phase and
observation count, and the original application/batch. Intervening observations,
restoration or target changes cannot silently redirect an old plan. Existing
Pause, Stop and Take control actions apply. Exact request replay returns the
existing job; restart does not resume it. Cancel waits for an active observation
to settle, then prevents subsequent observations. Stopping observation does not
restore the trial.

The desktop requests up to four moments one second apart. Agents can request
1–8 observations with 250–60000 ms intervals, within the existing eight-result
limit. While a trial is written, two slots remain reserved for restoration
checks. A watch never writes, injects game input, reapplies an edit, refits a
mapping or switches candidates. It cannot run alongside a discovery job.

Each result retains source images, observed numbers, complete interpretation
bytes around capture, and the existing persistence/timing outcome. Optional
bounded foreground key-state samples now accompany trial observations. Their
gaps, omitted polls and monitor generation remain explicit. They are not proof
of delivered input, continuous holds, gameplay updates or causation. The first
watch can lack context before its input monitor starts. Job receipts identify
the trial observation and image hash rather than borrowing a discovery frame.

## Three-case native matrix

The [protocol](PROTOCOL.md), [manifest](protocol-v1.json), unchanged product
binary and fixed inputs were frozen before these three launches. The owned
WinForms fixture randomizes a F32 field in the upper half of a 32 MiB allocation
and stores half the displayed number. The product receives no address or
representation hint. Follow changes selects the field, then twelve fresh
observations fit it and four later observations challenge it. Trial selection
uses the first qualifying candidate; the independent oracle verifies the
selection afterward. This is an exposed, purpose-built fixture.

The fixture renders cached text. A memory write alone does not refresh it; a
real foreground Space event does. Production has no knowledge of this rule or
key. After a single 50 → 60 trial, three paced observations include one before
the scripted event and two after it. Guarded byte restoration is followed by
two more observations, with another event between them.

All **three of three** fixed cases completed without runner errors:

| Recorded moment in each case | Visible value | Result |
|---|---:|---|
| Automatic check after the write | 50 | No displayed response yet |
| First effect-watch observation | 50 | No displayed response yet |
| Second effect-watch observation, after Space | 60 | Requested value matched |
| Third effect-watch observation | 60 | Requested value still matched |
| Automatic check after restoring the byte | 60 | Display had not refreshed |
| First restoration-watch observation | 60 | Display had not refreshed |
| Second restoration-watch observation, after Space | 50 | Original value matched |

Each case changed one byte and restored it once. Complete model reads bracketed
every result capture and agreed with the expected bytes. Each fixture confirmed
exactly two Space events, and the corresponding later observations retained
sampled Space context. The watch issued no writes. All 63 discovery readings in
this matrix were present and correct; although its rules allow explicitly
missing discovery readings, these three cases did not exercise that allowance.

The [post-launch audits](audit.py) verify all **87 image hashes**, disjoint
discovery stages, fresh-check provenance, candidate choice, job cursors and size
bounds, native write/restore receipts, and fixture/input observations. See the
[compact results](results.json), [execution receipts](execution.json), and
individual audits [0](audit-0.json), [1](audit-1.json), [2](audit-2.json).
Raw evidence is under `test-results/change-watch-v3/`.

## Earlier failures remain retained

This matrix does not replace the two earlier failed evaluations:

- [Revision 1](../change-watch/README.md) reached the trial, but a real watch
  step serialized to 261 bytes and exceeded the existing 256-byte journal limit.
  Cleanup restored the changed byte. Short unit-fixture identifiers had missed
  the issue. Compact journal-only names repair it without widening the bound;
  public command arguments are unchanged. A realistic-identifier regression
  failed before the repair and passes afterward.
- [Revision 2](../change-watch-v2/README.md), on the repaired build, had no
  readable text at fresh discovery frame 8. Its stricter harness stopped before
  a trial despite the product retaining that missing reading and obtaining four
  fresh agreements. No write occurred. Revision 3 explicitly permits null
  discovery readings while still requiring the product's qualification and the
  unchanged trial criteria. Wrong finite readings still fail. No case was replaced.

These cohorts have different methods and must not be pooled as a single success
rate. The earlier failures and their source images, metadata and cleanup remain
available; there is no blanket claim that observation always succeeds.

## Validation and limits

All **189 workspace tests**, strict Clippy, formatting, release build and the
**36-tool packaged smoke** pass. Regression coverage includes stale trial
counts/phases, reserved observation capacity, overlap with discovery, exact
replay, pause/restoration invalidation and cancellation while a read is active.
The native matrix exercises the shared commands through MCP; the new card
buttons were not separately clicked in a native UI test.

All 20 recorded owned processes across the five evaluations exited. Anode was
released after each cohort, with its already-visible viewer unchanged. No
screenshots were manually inspected. Desktop PID 67560 runs the verified new
binary, preserving 29 history entries and `qwen3.8:latest`; the inactive old
preview's state was saved before graceful replacement.

This fixture verifies that a delayed response and delayed display restoration
can be observed without erasing earlier nonresponses. It does not prove causal
authority in unfamiliar games, hidden-effect scope, game-state rollback or
restart recovery. The next independent-game investigation must observe relevant
updates and compare competing explanations while retaining natural-change
confounding and nonresponses. All weighted goal credits remain unchanged.
