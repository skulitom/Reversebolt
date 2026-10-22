# Shared comparison results on named control cards

Regional intervention results previously reached agents through completion,
history and planning responses but were absent from shared session status. The
desktop received only the old scalar summary and displayed no completed regional
result on the affected card.

The engine worker now publishes a compact historical `last_regional_review`
snapshot from successful completion, shared-reader and planner results. Test and
trial digests bind the snapshot; a review timestamp records when it was produced.
It carries the original regional assessment or a bounded unavailable explanation.
Normal status publication preserves an unchanged snapshot without repeating
image matching or reading evidence files. Changed test/trial records invalidate
it. Explicit analysis errors and mismatched results replace earlier support.
Unrelated commands cannot supply comparison results.

The desktop shows one muted line beneath an existing named card's controls:
response changed, no change found, uncertain, or unavailable. Hover and the
existing details view explain what was measured and its limits. The details
view includes the review's age. The note matches the exact current investigation,
catalogue and candidate path. A newer comparison for that path supersedes the
older note, even when the new test has no completed review yet. No unnamed card,
new control, semantic promotion, edit authorization or automatic retry is added.

Snapshots describe **past reviews**, not current live-game validation. Files can
change after a review; status does not silently re-read them. A failed command
that produces no review leaves the prior snapshot and timestamp historical.
Restart discards persisted display snapshots while retaining the actual test,
trial and source evidence. A shared read or plan can derive a new snapshot.

Automated verification extends the existing generated-PNG/input integration test
with synthetic edit/restoration receipts. It checks publication from all three
result shapes, no-op status preservation, digest mismatch, explicit analysis
failure, unrelated-command rejection, source invalidation, persisted restart,
and planner publication of supported, inconclusive and negative results. A
separate presentation-selection test checks exact scope/path matching, newer-test
supersession and the four short labels. These tests exercise the publisher and
selector directly; they do not launch the native editor or validate gameplay.

All **378 workspace test executions**, strict workspace/all-target Clippy and
format checks pass. Verification logs and source hashes are retained in
`test-results/comparison-card-status-v1`. No screenshot, game input, model request
or native target operation is required. Desktop publication remains unavailable
after quarantine, so this UI change has not been checked in a newly launched
native window. The executable and security review packet remain local as asked.

Overall progress remains **35%**. Shared evidence is easier to use, but gameplay
semantics, fresh real-game interventions and withheld generalization evidence
remain outstanding.
