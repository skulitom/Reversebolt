# Lock an appearance choice before the linked edit

The engine should store a one-time region choice only after the six baseline
captures and before any linked runtime edit record exists. Bind the complete
baseline and immutable comparison plan (target, value, actions, provenance and
scope), record the selection revision/time, and reject later replacement/removal.
An exact request replay must return the saved choice without mutation, including
after stop/restart. Existing tests without a choice must remain compatible.

Use automated protocol and real dispatcher/store tests with generated images.
Test wrong actor/cursor, an already prepared linked edit, altered baseline/action
metadata, conflicting selections, unchanged retry bytes, restart, automatic
comparison from the saved choice and source corruption after selection. A missing
source must leave historical test metadata readable. No target, input or field
write is needed. The generated scenario is not a native intervention experiment.

Freeze sources, replay binary and inputs before the compatibility replay. Reuse
all three OwnedPathFixture records and the exact region-selection manifests from
trial-appearance-v1. Do not add prospective plans to these already completed or
interrupted records. Replay each once, and require the entire JSON result to equal
the previous result, not just its success/status field. Retain every error and
ambiguous outcome. Audit unchanged frozen sources, binary and inputs afterward.

No new screenshot, game, model call, input or memory operation is allowed. This
verifies protocol/data compatibility and persistence, not meaningful automatic
region selection, game understanding, causal validity or protection from edits
outside Reversebolt.
