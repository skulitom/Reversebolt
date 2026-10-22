# Compare a selected appearance across a retained edit trial

The shared history reader and MCP now accept an `appearance` selection for
baseline condition 0 frame 0: its exact SHA-256 and an original-image rectangle.
They return a separate `trial_appearance_comparison`. The choice is retrospective
and can bias a result; automatic selection of a meaningful subject for a trial
is not implemented. No desktop report or verified control was added.

Every frame is matched against the same original template. Each condition is an
independent branch of up to six frames, stopping at its first unresolved match.
There are at most eighteen template checks, with no template update or within-
condition reacquisition. Cross-condition similarity does not prove object identity.
The algorithm uses the existing appearance matcher and thresholds unchanged.

The original global-patch and new regional comparisons now share streaming
verification of canonical evidence references, image/input hashes, size limits,
dimensions, chronology, clock and recomputed input witnesses. The new report
retains matching diagnostics, skipped frames, per-pair input failures, signed
displacements and source/test digests. A missing baseline or stale selection is
an error. Missing displacement is never replaced by zero.

Each of four action indices is compared separately across baseline, changed and
recovered conditions. Displacements use normalized 512×288 pixels; pairs also
give original-pixel deltas. Idle displacement must stay within one pixel per
axis, and every baseline action must reach two. A changed action must differ by
at least three pixels, and recovery must agree within one; the two-pixel gap
remains inconclusive. All four actions must support the pattern. These thresholds
are heuristics, not probabilities. Camera motion, background matches and identical
replacements can confound them. The result changes no saved assessment or edit
authority and establishes no physical speed, gameplay meaning or causality.

## Evidence

The dispatcher/store/restart tests pass with real image and sampled-input files
but no target. One fixture has a small textured subject on a static textured
background: baseline and recovery alternate +16/−16 normalized pixels, while
the changed condition remains stationary. The regional comparison recovers those
exact vectors; the global method remains `insufficient_support`. An initial test
expected a supported zero global displacement, which the actual estimator did
not provide. The expectation was corrected without changing images or algorithms.

Countercases cover a failed recovery action, moving idle interval, different
input-monitor generations, disappearance without reacquisition, partial records,
stale baseline hashes and corrupted image/input files. Restart preserves the
derived result. The saved test and its null assessment remain unchanged, and
numeric reads do not implicitly return PNGs. These are synthetic observations,
not native gameplay or a causal edit experiment.

All 358 workspace test executions pass. Strict workspace Clippy passes after a
test-loop cleanup, with all five history tests rerun afterward. Formatting passes.
Logs are `test-results/trial-appearance-tests.log`,
`trial-appearance-history-tests.log` and `trial-appearance-clippy-final.log`.

The frozen [protocol](PROTOCOL.md) replayed the same three older
`planned-input-v1` records using centered 64×64 rectangles chosen from metadata.
Their provenance is **OwnedPathFixture**, not Manifold Garden or independent
games. The complete record stops as ambiguous on the first frame of each
condition (three checks total), yielding `unresolved_appearance`. The two-frame
record remains `incomplete`; the empty record reports a missing baseline.
Every complete global comparison report equals trial-motion-v1, and all original
scalar assessments remain unchanged, including `suppression_observed` on the
complete record. The experiment does not resolve its striped appearance.

The audit verifies 153 frozen source files, the replay binary and 49 bound input
files: 43 original record/image/input files, three prior outputs and three new
selection manifests. No model call, game, capture, input or memory operation ran.
[Compact results](results.json) retain all outcomes; the full archive is
`test-results/trial-appearance-v1`.

Frozen manifest SHA-256:
`c93f249d6cd35ec188fcdd2e47a38e82197f7873e0e88674b84298884913d132`.

The next required evidence is a real offline-game comparison with a meaningful
selected appearance and a verified field intervention. Overall progress remains
35%; the desktop preview is still unavailable after its recorded quarantine.
