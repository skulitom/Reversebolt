# Reusing input interpretations with exact image scope

Implemented 2026-10-22. Full-goal progress remains **35%**. Verification uses
generated images and input records, mock inference and synthetic metadata. It
does not establish useful discovery in an independent game.

## Problem and behavior

An input experiment analyses ten images. The session also contains earlier
observations, but comparison planning previously hashed every session image.
Consequently an otherwise valid experiment interpretation could not reach a
comparison plan. Later metadata inference also interpreted the same images again.

New activity contexts carry the exact ordered image IDs, including the quiet
observations. Handoffs resolve and validate this sequence against its complete
metadata hash and source citations. Missing, duplicate, changed or reordered
source references fail, as do a changed current epoch or geometry. Additional
unrelated images with unchanged geometry can coexist with the historical context.
Legacy contexts retain their serialized shape and complete-image-list behavior.
Legacy experiment records can gain an explicit binding from their saved frame
list after validation; archived results remain untouched.

Local metadata review and agent hypothesis submission reuse the latest completed
experiment interpretation after checking its result and image hashes, original
receipt, published context and investigation scope. Local preparation also checks
that its repeated image measurements match the retained interpretation's source.
The original image producer, positive or negative verdict, measured basis and
uncertainty remain separate from the new metadata author. The receipt records
the reused experiment and source hash, with zero new image-model calls.

Metadata questions still undergo their independent inference or agent-submission
validation. An invalid, unavailable or cancelled experiment supplies no context
and no implicit image retry or fallback to an older positive result. Valid
metadata questions can still proceed, with the reuse error recorded. Cancellation
or a failed metadata pass does not publish the reused context. A fresh image
context that fails binding also leaves independently checked metadata intact.
Storage failures and inconsistent inference receipts still fail explicitly.

This change does not launch discovery, send input, read target memory, assign a
semantic meaning or grant edit authority. Comparison planning still proposes
investigations under the existing action and authorization rules.

## Verification

All **399 workspace tests** pass. A strengthened integration test covering the
last image-binding error change then passes separately. Strict workspace/all-target
Clippy and formatting pass. [Results and hashes](results.json) identify the logs
and frozen local source snapshot.

The generated integration fixture exercises both positive and negative image
interpretations. Its metadata callback receives no images when reusing evidence;
the original image receipt and context are preserved. It covers cancelled and
failed metadata work, altered published context, changed input metadata and a
cancelled latest experiment. Image failures retain independent metadata results.

The same test calls the real `submit_runtime_hypotheses` and
`plan_runtime_comparisons` commands with a generated catalogue. It reproduces the
old all-session-frame failure, then obtains four observed actions, one proposed
comparison and a next choice through the corrected command. There are still
eleven captured images (one preceding image plus ten experiment images), zero
memory samples and no edit authority. Replayed agent submissions return their
original reuse outcome; a later corrupted interpretation is discarded while a
new valid metadata submission completes.

A core test checks that an explicitly bound source survives an unrelated later
image and rejects changed input, image hash, capture numbering, missing or
duplicate references, reordered bindings and a new current epoch. The legacy
context still rejects an expanded complete image list.

## Limits and next evidence

No live model, native input, game, production app launch or security submission
ran. The unavailable preview was checked read-only. All artifacts remain local.
These fixtures verify the handoff and its failure handling, not game semantics,
causal control or generalization. Native validation, automatic investigation
selection and linking observed responses to controlling internal state remain
required. The fixed capability weights and **35/100** total are unchanged.
