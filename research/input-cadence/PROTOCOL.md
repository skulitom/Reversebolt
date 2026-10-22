# Input/capture timing sensitivity study

Use all four original `independent-games-v1` final-status files, in the fixed
order 2048-1, Tetris-1, 2048-2, Tetris-2. Read only retained FrameRef/input metadata.
The original protocol specifies 80 ms presses at a 350 ms period. Do not read
game source, oracle data, delivered-event logs, images or external annotations.

First run the unchanged production exposure classifier on the original frames.
Then construct hypothetical capture-operation intervals over each retained poll
stream: fifteen captures, costs of 100/200/400 ms, post-capture delays of
50/100/250/500 ms, and initial offsets of 0 through 325 ms at 25 ms increments.
Each start is relative to the original first capture's start. Keep all 672 grid
cases, including failed coverage. Do not optimize parameters on intermediate
results. These are sensitivity scenarios, not predicted or measured throughput.

Preserve every available poll's time, foreground state and key set. Deduplicate
identical polls only; conflicting duplicates or changing monitor generations
fail. Use the production bounded Buffer to extract hypothetical windows. Do not
fill missing polls or treat sampled input as delivered game action. Synthetic
frame IDs/hashes explicitly identify timing-only records, which have no images.

Report the production classifier's complete interval classifications, reasons,
and counts. Count single-key intervals whose immediate neighbors are quiet and
same-key pairs of such intervals as timing-only opportunities. They are not
activity evidence: no image changes, stable regions, semantics, planning witness
or editable control have been measured. In particular, do not call these
quiet-bracketed image episodes or submit them into a live investigation.

Independently audit accepted classifications for pressed samples that overlap
either capture operation. Run the same program before and after correcting the
classifier, retaining both binaries and all outputs. Freeze all production
source/lockfiles, runner/protocol, input statuses and the corresponding binary
before each run; audit afterward. Store create-only outputs in
`test-results/input-cadence-v1`. No game, target access, model, capture or input
delivery is permitted. Rejecting additional ambiguity is not generalization
success. Do not change capture cadence based solely on hypothetical results.
