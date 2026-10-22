# Automatic follow-up regions: frozen saved-observation regression

Freeze this protocol, the source tree, replay executable and manifest before
running the replay. Preserve all results, including abstentions and failures.
No capture, model call, native target access, game input or field write is allowed.

Use the unmodified final observations and all 16 retained images from the existing
`visual-objects-v1/tetris-1-attempt-01` run. This corpus is already exposed and is
not withheld-game validation. Do not use its game-state oracle, crop annotations
or source code as product input. Existing labels are observer claims, not truth.

The fixed generic policy accepts provisional observations bound to the exact
earlier image, investigation, epoch and recorded transport. Original bounds round
outward to integer pixels with no padding; width and height must each be at least
32. Compare only the next requested capture when its capture number is consecutive
and geometry/epoch agree. Never carry a box over a gap or infer object identity.
Equal rectangles in a pair share a measurement while retaining every source.
Sort smaller areas first within each pair, then share a four-region budget in
chronological rounds across pairs. Report omission counts and preserve uncertainty.

Evaluate these three cases with the same frozen executable:

1. All original frames and original visual tracks.
2. The same frames with no visual tracks, representing absent or legacy evidence.
3. The original tracks with capture 3 omitted from the requested frame list,
   testing that capture-2 observations do not silently become capture-2-to-4 regions.

Cases 2 and 3 are deliberate ablations, not new gameplay. Case 1 has three
observations on capture 2 and four on capture 16. Expect three selected regions
on pair 2–3 and four omissions for no following image. Case 2 must yield zero
regions. Case 3 must yield zero regions, three omitted-capture omissions and four
no-following-image omissions. This is a source/selection expectation, not an
expected displacement. Record all selected, matched, unresolved and consensus
results; do not tune on them. Regional motion accuracy is unknown in this corpus.

Automated tests separately exercise hashes, contradictory claims, duplicate boxes,
budget ordering, uncertain or missing bounds, tiny/out-of-image boxes, changed
epochs, inactive historical tracks, the actual shared reader and automatic
discovery archive. Compare base image/input evidence with automatic regions
disabled to ensure this extra measurement does not change existing decisions.

Keep original input hashes unchanged. Freeze the result hash and process receipt.
The executable uses the product's selection and interval-measurement functions.
Automatic discovery archives the measurements but does not yet feed these regions
to its image interpreter or change comparison/edit eligibility. A successful
selection test is not semantic recognition, object tracking or verified editing.
