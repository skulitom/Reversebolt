# Inspect questions before an executable comparison exists

`reversebolt_inspect_runtime_investigation` is a default MCP operation backed by
the same shared engine as the desktop. It returns accepted metadata questions
and separately checked activity without requiring a usable action replay first.
The previous comparison planner keeps its existing eligibility rules.

Every primary and cited alternative is resolved again against the retained
discovery inventory. Separate alternatives keep their own path and selected
metadata; the displayed name/question still belongs to the primary hypothesis.
Questions with zero values or values outside the current zero-probe bounds remain
available for investigation. These values are discovery samples, not fresh reads.
Stale process, catalogue, review, path or metadata bindings are rejected.

Activity is checked independently against retained frames, input metadata and
image evidence. If it is missing or changed, the packet keeps valid questions and
reports an activity error. Otherwise it retains the original interpretation,
uncertainty, motion comparison and structured replay assessment. Out-of-range
duration estimates include their original bounds and episode/key references.
They are not clamped into the 100–1000 ms execution range. The original planner
still rejects an unsupported sequence.

This read does not prepare a trial, propose an edit value, alter cards, interrupt
an active experiment, access target memory, capture images or call a model. It is
an evidence handoff for an agent deciding what to observe next. It does not infer
terrain conditions, prove a question's meaning or supply an autonomous gameplay
plan. Automatic use by a local investigation policy remains future work.

The [protocol](PROTOCOL.md) separates generated integration checks, saved native
evidence replay and packaged transport verification. All **482 workspace tests**
and strict all-target/all-feature Clippy pass. Tests cover supported replay,
retained timing failures, hypotheses outside probe bounds, alternative provenance,
missing image evidence, stale process identity and unchanged shared session state.
The app integration target rejects target reads and new captures.

The saved native Sable replay passes in
`test-results/investigation-inspection-v1/`. It retains the original uncertain
interpretation and exactly three duration failures: 1004, 1003 and 1007 ms, with
their original sampled bounds. The 988 ms episode remains within range, but there
is still no eligible four-action plan. Original evidence hashes are unchanged;
there were no new model calls, gameplay inputs, captures or memory operations.

Preview **0.1.0-20261023-054738** passes its 64-tool MCP catalogue check and a
fresh headless app/MCP round trip to the new handler. With no target attached, the
handler returns the expected connect-an-application error. Both owned processes
exited. The desktop launcher now selects this preview; the user's existing editor
stays open on its earlier build. No live-game claim follows from this startup check.

Reproduction after building the release workspace and `activity-assessment`
example:

```powershell
python research/investigation-inspection/replay.py
python scripts/mcp-catalog-smoke.py <staged-mcp-executable> <new-output-folder>
python research/investigation-inspection/native-smoke.py <staged-binary-folder> <new-output-folder>
```

Each runner creates a new output folder and retains its process/terminal evidence.
Do not restart it after an uncertain outcome; inspect the existing receipts and
process first. Native transport checks attach no game and cannot establish game
generalization. Full-goal progress remains **36/100**.
