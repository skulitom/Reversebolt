# Bounded region appearance correspondence

Freeze source, protocol, executables and input hashes before the replay. Do not
tune on saved-game results. Preserve failures and abstentions. Run no game, input,
capture, native helper or model. No external upload is part of this experiment.

The hypothesis is limited: a fixed rectangle can lose a translated appearance;
a bounded whole-image template search can sometimes locate that same appearance
elsewhere. A visual correspondence does not establish object identity, causality,
semantics or physical speed. Identical objects can replace one another unnoticed.

## Frozen method

Use RGB601 grayscale and Triangle resize to 512×288. Map the source region outward
using integer bounds. Reject normalized rectangles with either side below eight
pixels or area above 16,384. Search every fitting target position using an evenly
spaced grid of at most 16×16 source pixels and zero-mean normalized correlation.
The source and target samples must each have standard deviation at least eight.
Require correlation at least 0.92 and a margin of 0.06 over the best position
outside a two-pixel square around the winner. Border-touching winners abstain.
Verify the winner using every normalized pixel of the rectangle at correlation
at least 0.92. Do not substitute a second candidate after failure. Perform the
same unambiguous search backward; it must return within one normalized pixel of
the original origin. Only then return a displacement, scaled to original pixels.
All thresholds are declared heuristics, not calibrated probabilities.

Do not fit rotation, scale or deformation. Do not propagate a match to another
image. The existing four-region source-selection policy and input bindings stay
unchanged. The new result is separate from fixed-region patch motion, and does
not alter automatic interpretation, trial assessment or editing eligibility.

## Inputs and assessment

Generate five analytic image pairs from a fixed seed, with one source observation:

1. A textured 64×64 appearance moves by (+160,+64) in a 512×288 image, with +20
   brightness. Expect the exact displacement despite leaving the original region.
2. The same appearance stays in place. Expect zero displacement.
3. Two identical copies appear in the next image. Expect unresolved correspondence.
4. Flat source and target images. Expect unresolved correspondence.
5. Replace the appearance with unrelated texture. Expect unresolved correspondence.

Keep the scorer's descriptions and expected displacements in a separate truth
file, never supplied to the matcher. Unit tests also cover reverse ambiguity,
sparse-sampling aliases rejected by dense verification, boundaries and geometry.

Replay all three unchanged cases from `observed-regions-v1/cases.json` afterward:
the 16-frame Tetris session, no-observation ablation and omitted-capture ablation.
These already exposed images have no displacement ground truth here. Report every
appearance result without treating accepted matches as correct. Compare each
earlier selection and fixed-region result exactly with the original frozen replay.
No game-state oracle, source code, game-specific offsets or labels enter matching.

The executable uses the actual app's automatic-interval helper. App tests cover
the shared reader and automatic evidence archive with exact hash/epoch checks.
Record runtime as one local observation, not a benchmark. Freeze all result hashes
and verify unchanged original inputs. Passing analytic cases is not independent
game acceptance or proof of improvement in real-game motion estimation.
