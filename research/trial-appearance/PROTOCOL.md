# Retained trial appearance comparison

Freeze the source, replay binary and inputs before this replay. Read the same
three `planned-input-v1` records used by trial-motion-v1. Their provenance is
OwnedPathFixture; they are not Manifold Garden or independent game evidence.
Never modify the records, images, sampled inputs or prior assessments.

For each record with a baseline image, select one centered 64 by 64 pixel rectangle
in baseline condition 0 frame 0, using metadata alone. Bind its original SHA-256.
Do not inspect screenshots to adjust the rectangle. For the empty record, submit
an explicitly unavailable all-zero hash and a 64 by 64 rectangle at the origin,
and retain the expected missing-baseline error rather than supplying an image.

Run the current shared file replay once per record, including both the unchanged
global patch comparison and the new appearance comparison. Keep every outcome.
Compare the complete global report with trial-motion-v1 and verify that original
assessments remain identical. At most eighteen template checks are permitted per
appearance comparison. Every image is independently matched to the same baseline
template; each condition stops at its first failed match. Conditions are separate
branches, never one trajectory across the intervention gaps.

This is a provenance and failure-handling regression, not an accuracy evaluation.
The center region has no verified semantic meaning. Synthetic automated tests
separately cover a small textured subject against static background, alternating
action directions, unchanged edited positions, return to baseline displacements,
failed recovery, disappearance without reacquisition, idle movement, mixed input
monitor generations, missing data, stale image selection and history restart.
Do not equate those images with gameplay or causality.

No model call, game, new capture, input or memory operation is allowed. Audit all
frozen sources, binary, selection manifests and original evidence afterward.
Record errors and uncertainty without promoting the result to an editable control.
