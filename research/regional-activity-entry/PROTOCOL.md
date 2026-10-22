# Regional activity entry regression protocol

Freeze production Rust/TOML/lockfile hashes, the replay executable, two existing
saved-game manifests, their source PNGs, cached observation receipts and prior
regional reports/summaries before replay. Output is create-only under
`test-results/regional-activity-entry-v1`.

The change admits a bounded regional investigation source when whole-image
contrast selects none. Existing whole-image sources must retain their exact
selection and regional measurements. The two exposed saved cases (Hyperbolica
and Manifold Garden) test this compatibility only. They are not withheld games,
new interventions, or evidence of the weak-contrast fallback succeeding in a game.

Use the production saved-image reader via `runtime-appearance-replay`, explicitly
with `--observations` to replay the prior hash-bound observations. The executable
has a backend that rejects target reads, writes and capture. Do not invoke a
model or launch a game. Compare the selected source with the previous source,
the complete regional report and compact summary with their prior counterparts,
and base image/input evidence and replayed observations with the original run.
Record differing fields rather than silently replacing expected outputs.
Audit all frozen hashes after both runs.

The new fallback is separately exercised by an automated generated-image test:
actual whole-image differences below the old gate, source selection, shared
reader, archived interpreter input, retained ambiguous interpretation and a
four-action proposal from two sampled episodes. It also checks missing regional
evidence, a nonmoving second pair, changed input, static images and failed image
reads. This is software integration evidence with no native gameplay semantics.
Run the full workspace suite, strict all-target Clippy and format checks.
