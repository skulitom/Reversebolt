# Automatic regions from retained observations

The shared `read_play_evidence` path now chooses and measures up to four fixed
image regions without manual coordinates. It uses existing provisional visual
observations, not new inference. Automatic discovery archives the same output.
This removes a manual query requirement but does not establish object identity,
gameplay meaning, motion accuracy or edit authority.

Selection is deterministic and generic: exact investigation/image/epoch/track and
transport bindings, provisional quality, original bounds rounded outward with no
padding, and a consecutive next capture with unchanged geometry/epoch. Regions
must be at least 32 pixels in each dimension. Small areas come first within each
pair; chronological rounds share the four-region budget across pairs. Identical
rectangles in a pair retain all original claims under one measurement. Missing or
uncertain bounds, unsupported sizes, source mismatches, gaps and last captures are
reported as omissions. Inactive tracks can still support exact historical images.

Every selected region preserves its source observations and their hashes. A plan
hash binds the policy, investigation, both full frame references, region and
sources. The selection also binds all requested frame metadata. Measurement
verifies image hashes, actual PNG dimensions and epoch. Region displacements use
a separate 256×144 normalization; the report gives source-pixel scale factors.
`auto_regions: false` opts out. The explicit `region` argument remains independent.
Image interpreter inputs, comparison decisions, names and edit eligibility are
unchanged. No report or extra controls are added to the human board.

## Frozen saved-observation replay

[The protocol](PROTOCOL.md), replay executable, 143 source files and all inputs
were frozen before execution under `test-results/observed-regions-v1`. The corpus
is the already exposed Tetris run from [visual object discovery](../visual-objects/README.md):
16 original images and seven original provisional observations, with no oracle,
game source, manually chosen boxes or new model call supplied to this replay.

The original case selected all three capture-2 regions for pair 2–3 and omitted
the four capture-16 observations because there was no later image. No observation
from capture 16 was carried backward or presented as a current tracked object.

| Retained observer claim (unverified) | Selected patches | Matched | Result |
|---|---:|---:|---|
| Orange L-shaped block piece | 5 | 5 | All five zero displacement; insufficient support for consensus |
| green tetromino piece | 10 | 0 | 3 search-boundary and 7 poor-match abstentions |
| Game Board | 6 | 0 | 4 search-boundary and 2 poor-match abstentions |

All three region computations completed without errors. No region had consensus.
There is no displacement ground truth in this evaluation: these results do not
verify motion or show improved motion accuracy. The two deliberately altered
manifest cases also behaved as specified: absent observations produced no regions;
omitting capture 3 produced three omitted-capture and four no-following-image
omissions, with no measurement. These are ablations, not new gameplay sessions.

All 18 input files, including the original status and 16 PNGs, remained unchanged.
One local replay of three cases exited successfully in 0.156 seconds; this is a
single run, not a performance benchmark. The compact [results](results.json)
retain hashes and measurement counts. Raw sources, plans, patch results and the
process receipt remain in the local frozen evidence directory.

## Checks and remaining work

All 338 workspace test executions and strict workspace Clippy pass. Tests cover
exact source rejection, claim/metadata hash sensitivity, unknown or invalid
bounds, outward rounding, historical tracks, chronological gaps, duplicate claims,
the four-region budget, actual shared reads and automatic archive retention.
Disabling the new analysis leaves base activity, patch motion and frame hashes
identical. Corrupted saved pixels still reject the whole reader request. Test
targets panic on capture and memory reads.

The replay does not prove that every automatic discovery run has timely usable
visual observations. The final observer pass has no following image in its batch.
Fixed regions can lose moving objects, admit unrelated pixels or distort aspect
ratio during normalization. Existing patch thresholds can abstain on small,
repetitive or fast-changing objects. These measurements are archived but do not
yet participate in automatic activity interpretation or trial validation.

Weighted goal progress stays at **35%**. The desktop remains unavailable after
the existing Windows quarantine; this change is source-only. No replacement was
published, no security setting changed and no executable was externally uploaded.
