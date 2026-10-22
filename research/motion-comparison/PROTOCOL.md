# Deterministic comparison of retained patch displacement

The prior image interpreter can contradict measured direction. Derive a separate
versioned description from retained numerical evidence, without rewriting the
model's observation or making it a gameplay control. This compares two discovery
activity intervals, not baseline/changed/recovered trial outcomes.

Validate the existing source and v2 motion summary before computing anything.
Missing measurement yields missing evidence. If either pair has no retained
validated spatial consensus, report insufficient support. A consensus requires
the existing >=10 supporting patches, >=75% of matches, >=6 cells spanning two
rows and two columns. If either consensus vector has squared length below four,
report below the two-pixel threshold, never no movement.

For two supported vectors above that threshold: differences <=1 normalized pixel
on both axes are matching displacement; otherwise positive dot product with
dot² * 100 >= normA² * normB² * 81 is aligned direction; the same bound with a
negative dot product is opposed direction; remaining pairs have different
directions. Thresholds are fixed heuristics, not calibrated confidence. They
compare image displacement, not velocity, physical speed, objects or causality.

Return exact vectors, selected/matched/support counts and cells, image bindings,
source and summary digests, policy ID, and template-generated description. The
sign convention is x right / y down in 256×144 normalized images. Zero components
must not become a scene-stationary claim. Preserve the model's prose independently.
No planner eligibility or stored experiment assessment changes.

Test signed and matching directions, differing magnitudes, opposite/orthogonal
motion, threshold cases, missing consensus/measurement and malformed bindings.
Verify app handoff/archive/replay behavior and historical reads without target I/O.

Freeze a file-only replay of eight receipts from motion-interpretation: V1's two
real-game cases and V2's two blinded controls, each image-only/revised variant.
Original PNGs, receipts and all model interpretations stay unchanged. Baselines
have no summary; revised real pairs lack consensus; the corrected controlled
pairs should yield matching and opposed displacement respectively. Retain the
earlier contradictory prose and show both outputs separately. Do not infer
generalization from these known cases. No model, game, screenshot, target read,
input or gameplay write runs. No new verified-control credit is planned.
