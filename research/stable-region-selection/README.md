# Stable texture before region matching

Automatic passive region selection now finds appearances that pass the existing
correspondence checks in both retained game recordings. The old change-only
selector still returns exactly the archived failures. **This is improved region
selection on known development recordings, not verified gameplay controls or an
accuracy measurement.** No region was manually chosen or adjusted after replay.

| Recording | Old supported / tried | New supported / tried | Old / new frame checks |
| --- | ---: | ---: | ---: |
| Hyperbolica | 0 / 4 | 4 / 4 | 6 / 20 |
| Manifold Garden | 0 / 4 | 1 / 4 | 6 / 12 |

The new default `passive_activity_stable_texture_regions_v2` keeps the existing
first-action-minus-quiet change gate, then screens source texture and quiet
stability before spending its four-candidate search budget. It reuses the motion
module's original 9x9 minimum-eigenvalue corner scorer and the matcher's exact
sparse/dense template statistics. The underlying minimum-eigenvalue feature
concept is documented by [OpenCV](https://docs.opencv.org/4.12.0/dd/d1a/group__imgproc__feature.html).
No new dependency or upstream source is incorporated.

Candidates require both source standard deviations >=8, corner score >=16 and
quiet mean absolute difference <=4. Survivors rank by corner score divided by
one plus quiet difference, with deterministic ties and the existing overlap
suppression. The source statistics use the exact rounded matcher rectangle.
Neither second-interval pixels nor match outcomes influence candidate selection.
The matching policy, twenty-check limit, stop rules and motion thresholds remain
unchanged. Read the [frozen protocol](PROTOCOL.md) for full details.

| Recording | Change-eligible tiles | Low texture | Weak corner | Quiet change | Remaining |
| --- | ---: | ---: | ---: | ---: | ---: |
| Hyperbolica | 835 | 179 | 397 | 19 | 240 |
| Manifold Garden | 367 | 105 | 203 | 0 | 59 |

These rejection categories are exclusive in that order. Four survivors per case
were tried, without replacing failures. The first ranked candidate in each game
passed all five checks and was recommended:

- Hyperbolica: original region `[189,416,35,34]`; normalized quiet/action/action
  deltas `[0,0]`, `[-30,3]`, `[-28,3]`. All four candidates passed both intervals.
- Garden: original region `[640,80,40,40]`; deltas `[0,0]`, `[14,0]`, `[15,0]`.
  The other candidates stopped on first-action ambiguity, inconsistent reverse
  matching, and root self-match ambiguity respectively.

These are image displacements in normalized 512x288 coordinates. They do not
identify a character, distinguish camera from actor motion, prove game delivery
of sampled keys, or measure world speed. The intervals are independent template
correspondences; intervening images remain unexamined. Identical appearances can
mislead, and no ground-truth motion map is available. The new method was designed
after seeing the previous failures, so an independent recording is still needed
to evaluate generalization. No named card, edit authority, trial baseline or
automatic lock follows from a recommendation.

Both methods ran twice per case. Entire JSON outputs matched within each method,
and the v1 outputs matched the preceding study exactly, including source hashes
and diagnostics. Each run read only the five selected PNGs. Local debug-profile
times were 1.219–1.250 seconds for the new Hyperbolica analysis and 0.781–0.797
seconds for Garden, versus 0.297–0.360 seconds for the old analysis. More candidates
reached expensive later checks; these runs are not a general latency benchmark.
There were no fresh captures, model calls, gameplay inputs or target operations.

The shared application/MCP read path uses v2 automatically and preserves its
existing opt-out. Reports expose exclusion counts and each tried candidate's
selection quality. The library and file-only replay retain explicit
`Ranking::ChangeOnly` / `change_only` for the old derivation. Neither policy
changes saved investigation state, assessment gates or UI card presentation.

All **367 workspace test executions**, strict workspace/all-target Clippy,
formatting and diff checks pass. New controlled checks demonstrate recovery of a
known translation despite a larger untextured flashing distractor, unchanged
candidate selection when later-pair images change, rejection of quiet flicker
and one-direction texture, and exact normalized-rectangle/source-stat agreement
with the matcher. Existing input/source integrity, opposite-direction motion,
stop/abstention and no-target-access application checks also pass.

Frozen artifacts are in `test-results/stable-region-selection-v1`; compact
[results](results.json) preserve both methods and all failed candidates. All 161
source files, 16 original inputs, 12 generated replay inputs and the executable
pass the unchanged audit.

- Manifest SHA-256: `694689b9ef53725d86e7c7ce4a59d57b07f6596e331bbc84da06db7328d6ece4`
- Executable SHA-256: `25a0f14ebbcdbacb1488c5b25eec41c6b63a478c174fa3b4f6eed8dc4b2e6e29`

The runner refuses to overwrite this study. Overall weighted progress remains
**35%**: this evidence does not verify controlling fields, causal edits or
independent generalization. The desktop preview remains unavailable after its
earlier Windows quarantine; no replacement was published or launched.
