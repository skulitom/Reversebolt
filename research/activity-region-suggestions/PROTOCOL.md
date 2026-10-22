# Passive activity region proposals — frozen protocol v1

Question: can the unchanged baseline tile-ranking method propose a repeatably
matched moving region on two already recorded games, without observer boxes,
manual screenshot review, fresh model calls, live input or memory operations?

Use exactly the retained cases `runtime-appearances-v1/case-0.json` (Hyperbolica)
and `case-1.json` (Manifold Garden), with their original activity sources from
`appearance-context-v1/case-{i}-baseline.json`. Preserve all fifteen frame
references and sampled inputs. These are passive recordings, not edit trials.
No regions, names or outcomes from the old visual observer enter selection.
Copy only the first action's before/after PNGs, its immediate preceding quiet
PNG, and the second action's before/after PNGs into the new replay directories.

Freeze all Rust and Cargo sources, this protocol, runner, original case/source
JSON files and the ten selected PNG paths before building or running. Generate
file-only replay manifests that retain the original full frame metadata and
source; remap image filenames to hash-named relative paths. Audit those generated
inputs, original files, frozen sources and executable before/after both repeats.
Use `activity-region-suggestions-replay`, linked to the same vision implementation
used by the shared application reader. Do not launch the desktop executable or
package a replacement for a quarantined build.

Method, fixed before results: policy `passive_activity_region_suggestions_v1`.
Normalize to 512x288 using the existing grayscale/Triangle path. Rank tiles of
16/32/64 pixels at half-side stride by mean positive residual of first-action
absolute change minus preceding quiet absolute change. Threshold 8; sort score
descending, tile size/y/x ascending. Map outward to original pixels; reject
regions that fail existing minimum-size bounds instead of padding. Keep at most
four, greedily suppressing overlap greater than a quarter of the smaller tile.
All constants and matching thresholds are shared unchanged with the earlier
baseline region suggester and existing immutable template matcher.

Each candidate uses the first-action before image as its immutable root. Check
root-self, preceding quiet, first after, second before and second after, stopping
at the first unresolved match. At most twenty checks. Retain all diagnostics,
root-relative positions and three interval deltas. Recommend the first ranked
candidate with quiet Chebyshev displacement <=1 and both action displacements
>=2 normalized pixels. Do not require equal magnitude/direction or infer meaning
from the key. Missing/corrupt pixels, mismatched source metadata, mixed selected
input monitors or known capture/epoch/geometry discontinuity return an error.
The quiet interval must have valid sampled no-key evidence. No fallback search,
region adjustment, threshold tuning or second chance after an unresolved match.

Run each case twice. Preserve failures and abstentions as outcomes. Record exact
read frame IDs, ranked count, attempted count, all candidate regions/scores,
statuses, displacements, recommendation index, complete JSON equality, elapsed
times and audit results. Reject unexpected reads outside the five selected PNGs.
Generated-image and no-target-access application tests are separate software
checks, not game evidence. Source corrections after a frozen run require a new
study version; retain the original run.

Interpretation limits: even a recommendation only establishes heuristic image
correspondences on two independently checked intervals. Intervening pixels are
not read; occlusion, replacement or scene changes there may be unknown. A matching
background/camera region or identical object can pass. There is no ground-truth
object label, physical quantity, game-delivery guarantee, verified internal field,
causal intervention or edit/restore validation. Do not present a positive match
as a named control, persistent tracking, or a generalization success percentage.
