# Shared-engine execution of planned inputs

`observe_runtime_test` now accepts `perform_action: true`. The existing default
only observes. The opt-in path executes the immutable action for the next frame
of the prepared comparison, then captures through the existing engine pipeline.
The first two frames wait with no keys pressed. Each subsequent frame brackets
its planned hold with 300 ms of released input. Edits and recovery remain explicit
shared-engine commands, with the existing baseline, reference and value guards.

The engine saves its pending capture and an exclusive input intent before
launching a hidden, short-lived worker. The worker checks process creation
identity, Windows session, exact foreground window, other pressed keys/buttons,
editor lifetime and cancellation. It never brings an application to the front.
Only letters, digits, arrows and Space are supported, in sorted chords of up to
three keys and holds of 100–1000 ms. The independent worker releases its keys
when the editor exits. A failed or unsettled step stops the test and cannot be
replayed with its old revision.

`read_runtime_test` returns the saved execution intents and outcomes, including
steps that failed before capturing an image. Outcomes bind the intent hash and
record accepted keydown/keyup counts, timing, worker identity and errors. Those
counts are Windows API results, not proof of game delivery. Existing sampled
input witnesses and image/value checks still decide the comparison assessment.
Focus checks are sampled, so they cannot provide atomic input isolation.

## Frozen native evaluation

The [protocol](PROTOCOL.md) and five binary hashes were frozen before
`test-results/planned-input-v1/case`. This reuses an owned, explicitly labelled
ABI fixture and renderer. The evaluator chooses its field and D/A/D/A plan;
the product executes the inputs. No game discovery or learned key meaning is
claimed. No external gameplay input or manual image review occurred.

The actual MCP workflow passed **57 assertions**:

- A complete baseline/edit/recovery comparison retained 18 images and 12
  product-executed holds. Rendered-response medians were 136.845, 0 and 131.806.
  The fixture independently confirmed the original bytes after restoration.
- Eighteen stale observation requests produced neither captures nor new input
  intents. Hypothesis references and alternatives remained intact.
- Moving focus to a separate owned window caused refusal before any keydown.
- Terminating the editor during another hold caused the worker to release the
  key and save an error. Restart retained a stopped pending test without replay.

There are 22 execution intent/outcome pairs: the successful comparison, one
focus refusal, and two idle frames plus one interrupted hold. All 22 workers and
the owned test processes exited. Nine runtime helpers unloaded; the fixture's
independent counters matched 21 balanced pauses. Maximum reported pause was
58 microseconds, with no helper heap operations while paused. Two confirmed
writes comprise the intended change and its restoration.

The [post-run audit](results.json) checks source/binary hashes, receipt binding,
images/input windows, worker cleanup, original bytes and desktop preservation.
The 272-test workspace suite, strict Clippy, formatting, release build and
52-tool smoke pass. Preview `0.1.0-20261021-225832` is running with all 44 history
entries, model settings, changes, trials, tests and Freeze records preserved.

This adds repeatable action execution to agent investigations. Choosing which
explanation to test, learning action meanings, establishing semantic controls and
validating the policy on independent games remain unfinished. Human takeover
wiring is retained; this native run tests editor exit rather than a Take control
click. The audit was written after execution without changing the frozen inputs.
