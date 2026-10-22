# Corrected source identification — 2026-10-22

The history replay documentation in commit 02680d9 incorrectly identified
comparison `080b5643-5718-494f-9ed6-751bf8059402` as Manifold Garden. Its actual
source is the **owned ABI fixture and striped renderer** in
`test-results/planned-input-v1/case/data/runtime-tests`.

Authoritative evidence: the [planned-input evaluation](../planned-input/README.md)
explicitly describes that fixture; the saved candidate's root declares image
`OwnedPathFixture` and type `Owned.Roots, OwnedPathFixture`. The `WalkSpeed`
label was deliberately supplied by the fixture, as documented in the earlier
[hypothesis-test evaluation](../hypothesis-tests/README.md).

The original JSON hash remains
`dc0b968f58977d8f4b34be6ac1f4110a1ee85e808036d1d32cab169b7a9fbbd6`.
No stored input, image, receipt, replay output or frozen source copy is changed.
The nine-record restart and evidence-readability findings remain valid for the
fixture. They establish neither real-game acceptance nor automatic discovery.
Progress was 35% before and after the replay; no generalization credit was given.

The same labeling mistake entered patch-motion-v1's three comparison-case names.
That frozen result is retained with this correction. Patch-motion-v2 uses the
correct fixture names and adds separately identified genuine Manifold Garden
captures from comparison-planner-v1. It does not change measurement thresholds.
