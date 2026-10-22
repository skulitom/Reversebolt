# Compare measured displacement without relying on model prose

The earlier image interpreter sometimes contradicts known displacement directions.
A shared core comparison now derives a separate description directly from the
retained numerical evidence. It preserves the original interpretation, including
its mistakes and uncertainty. **The result describes selected image patches;
it does not establish a gameplay effect or name an editable control.**

The policy validates the source and v2 motion summary, then requires the existing
spatial consensus in both pairs: at least ten supporters, 75% of accepted matches,
six cells, two rows and two columns. Missing measurements and insufficient support
remain explicit. Consensus shorter than two normalized pixels yields
`below_threshold`, never a claim of no movement.

Above that threshold, vectors within one pixel on both axes yield
`matching_displacement`. Otherwise their dot product distinguishes aligned,
opposed and different directions using the fixed squared-cosine threshold 0.81.
These are heuristic categories. They compare displacement rather than velocity;
the observations can have different time intervals and changing scene geometry.
Signed vectors and template-generated text use x right / y down in 256×144 images.

## Shared workflow

New agent hypothesis receipts and comparison plans expose
`activity_motion_comparison`. Its scope is explicitly `discovery_activity_pairs`.
Image references, support/coverage counts and canonical source/summary digests stay
with the result. New receipts replay the exact archived comparison through later
handoffs and restart. Legacy receipts without it return null.

Historical test reads expose `origin_activity_motion_comparison` from the saved
question origin. This refers to the original discovery activity, **not the trial's
baseline, edit or recovery outcome**. It works without a connected target, new
images or a model. A dispatcher/store/restart test verifies that the original
test and its null assessment remain unchanged while opposing discovery vectors
receive the correct description. Existing eligibility and write guards remain.

## Frozen replay

The [fixed protocol](PROTOCOL.md) replays eight retained receipts: the prior two
real-game cases and two corrected, blinded image controls, each in image-only and
motion-augmented form. There were no new model calls, screenshots, image review,
target reads, game inputs or field writes.

| Receipt evidence | Derived comparison | Original interpretation |
| --- | --- | --- |
| Four image-only receipts | Null: no numeric summary | Preserved, including the positive Manifold answer |
| Hyperbolica with sparse matches | Insufficient support | Preserved uncertainty |
| Manifold Garden with sparse matches | Insufficient support | Preserved uncertainty |
| Controlled right/right shifts | Matching displacement: 18 px right, then 18 px right | Preserved the description that also incorrectly says “leftward” |
| Controlled right/left shifts | Opposed direction: 18 px right, then 18 px left | Preserved the model's separate explanation |

The run took 0.109 seconds. It uses the corrected controls from the prior study;
the earlier contaminated controls are not reused as acceptance evidence. These
are known-case regression checks, not withheld generalization or native gameplay
validation. The original sources, PNGs and model receipts remain unchanged.

All **320 workspace test executions** pass (319 distinct cases; the epoch guard
also runs in the image-replay example). New tests cover matching/aligned/opposed/
orthogonal vectors, signs, small/missing/sparse evidence, changed bindings and
malformed consensus. Real app handoff/archive/replay and historical-reader tests
verify that interpretation and trial assessment remain independent. Strict Clippy,
formatting and diff checks also pass.

Raw executable/source/input freezes and output are under
`test-results/motion-comparison-v1`. [The audit](audit.py) checks file hashes,
unchanged observations, image references, retained vectors and all eight outcomes.
[Results](results.json) include the original prose beside the new descriptions.
No new game compatibility, verified control or physical interpretation is claimed.
Weighted progress stays **35%**. The source increment is unpublished; the desktop
preview remains unavailable following quarantine.
