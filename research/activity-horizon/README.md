# Appearance traces reach the selected activity intervals

The automatic reader now uses the selected activity's last image as its trace
horizon. Previously, a six-step limit stopped otherwise successful traces before
the second selected interval in both exposed saved-game cases. The new policy
retains the same twelve attempted matches, shared in rounds across at most four
regions. Every intervening image must still match the original template; an
unresolved match, missing capture or geometry/epoch change stops that source.
There is no skipping, template update or reacquisition. Without a selected
activity source, the existing six-step behavior and serialized report remain.

`activity_bounded_appearance_sequence_v1` records the complete activity source's
digest and its final image identity in `target`. Selected image identities,
hashes, dimensions, clock bounds, input digests and sampled-key classifications
are checked before tracing. `activity_horizon_reached` means no later requested
image is needed; individual summary outcomes still determine which selected
intervals have measurements. Budget exhaustion can leave an interval unmeasured.
The full source window remains limited to twenty captures. The matcher, its
thresholds and source-region selection did not change.

## Verification

All 357 workspace test executions, strict workspace Clippy and formatting checks
pass. Added controls cover reaching a later selected interval, no reads beyond
the target, the same twelve-attempt budget, fair sharing between four surviving
sources, stopping on intermediate failure/gap/epoch change, and altered activity,
image and input references. Review found and fixed a missing image-reference
check before the experiment; malformed references now return an error before any
image read. The selected-pair summary retains its source validation and missing
outcomes.

The [protocol](PROTOCOL.md), 151 source files, both replay binaries and 37 input
files were frozen before the saved-game run. No game, new screenshot, input,
memory operation or new object-observation inference ran. Cached observations
were replayed through the existing hash-checked session path. Base play evidence,
all observations and the selected regions remain exactly equal to the previous
run. Every shared-prefix step remains equal except its scheduling-policy digest.

| Saved case | Selected captures | x displacement, first / second interval | Trace attempts | Baseline / augmented consistency flag |
|---|---|---|---:|---|
| Hyperbolica | 2→3, 8→9 | −58.0078125 / −53.7109375 pixels | 10 | false / true |
| Manifold Garden | 4→5, 10→11 | +40 / +42.5 pixels | 12 | false / true |

Both y displacements are zero. One source per case supplied both measurements;
the other three selected sources stopped unresolved. Previously, the second
interval was unmeasured for every source. The surviving traces now accept seven
and nine steps, respectively, then stop at the activity horizon. Their original
rectangles and matching pixels remain the basis of the measurements, without
proving persistent object identity.

Four completed local activity calls compared baseline and augmented context on
identical images, model digest and options. Global patch evidence is equal within
each comparison. Both augmented interpretations now flag a repeated visible
change. Garden's description mentions the approximate 40–42 pixel shift, while
Hyperbolica's mentions leftward motion. However, the prose also extends to
unverified robot/pink-shape movement, camera explanations and claims that other
parts are static. An unresolved measurement does not establish no movement.
These are retained model claims, not accuracy ground truth or verified gameplay
controls. No causal link to an internal field, new-game performance or edit
authority is established.

The post-run audit verified all frozen sources, original inputs, generated
manifests and replay executables unchanged. The model digest was checked before
each call. [Compact results](results.json) retain every pair outcome, model
description and uncertainty. Full archives and call logs are in
`test-results/activity-horizon-v1`.

- Source manifest SHA-256: `ce9cd1a5edae52b4af65c9d30cc0e15ccc4ce045b33ed2586a2b5ac64228cc94`
- Model digest: `22130167c4c20e20c7b71454612966ca8e8171e9b3cc8ab6ce8aa6cbfec79643`

Overall progress remains 35%. Desktop publication remains unavailable after the
recorded Windows Security quarantine.
