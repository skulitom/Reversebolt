# Temporal evidence integration

## Decision

The isolated visual-behavior and visual-challenge experiments were reviewed,
without modifying their source or promoting their activity regions to objects.
The challenge's held-out clips reported 4 true proposals, 89 false proposals
and 22 missed visible extents at loose overlap; Celeste's idle negative still
produced 46 false entity proposals over seven scored frames. Those are the
experiment's reported results, not newly reproduced measurements. Its images
and source remain in the user-owned `experiments/` directories.

The integration therefore establishes multi-image evidence and timing before
claiming automatic entity semantics. No game-specific profile, detector, label,
source layout or experimental tracking algorithm was added to production.

## Production behavior

Every new captured image has a capture-operation interval in microseconds on
an investigation-local monotonic clock. Each memory-read receipt has bounds on
the same clock, including its mapping queries. The bounds include capture/IO
work, not game presentation time. Images and memory remain sequential. UTC is
retained for human provenance but is not used to measure elapsed time.

`read_sequence` resolves 2–8 ordered image identities in the current batch,
verifies their saved PNG hashes, and returns the original images plus timing.
The sequence records lower/upper elapsed bounds, largest unsampled gap and
omitted capture attempts. It never fills gaps or certifies continuous motion.
Images must share geometry, epoch and clock. Combined PNG bytes are limited to
16 MiB; the existing 10 MiB per-image limit still applies. Old captures without
monotonic bounds remain readable individually but cannot support this protocol.

`submit_temporal_observation` accepts declared appearance, screen-motion and
observed-change interpretations, with source, stable key/label/unit, typed
value, uncertainty, explanation and optional bounds on each inspected image.
The engine validates every frame before publishing anything. Exact replay is
idempotent. Conflicting evidence ending at an occupied frame needs a new key.
Late arrivals are sorted by ending frame and cannot displace later observations.
The transport source is engine-assigned, distinct from a declared observer name.

At most 16 temporal series and 20 observations per series are retained per
batch. The engine saves `temporal-evidence.json` before publishing a changed
collection. Failed saves preserve the old collection. Batch advancement archives
the observations and clears the current series. Resize/process exit invalidates
the series; restart leaves evidence historical without a target connection.
The manifest and session metadata are separate writes, not a cross-file
transaction. Exact retries remain meaningful only in the current active scope.

Desktop cards show these interpretations with a name, short description,
value and Inspect action. Earlier observations are not shown as current values
after another capture. Inspect retains the explanation, observer and timing.
No slider or editable game control is created. Observation submissions respect
human takeover and run on the existing serialized engine worker. They may join
the investigation between automatic captures, just like single-image evidence.

## Limits and next work

No production semantic model is invoked. Values, labels and cross-frame
correspondence are interpretations supplied by an observer. Temporal values
are deliberately excluded from the instantaneous scalar matcher: a windowed
rate or appearance change needs a suitable lagged model, identity evidence,
timing bounds and confounder checks. This interface does not verify speed,
animation frequency, world position, causal effects or editable memory.

The automatic pass still samples slowly with OCR and memory work between
images. It cannot resolve fast animation. A bounded higher-cadence capture path,
evaluated image/behavior models, input timing and prospective temporal matching
remain needed. This is infrastructure toward the full goal, not generalization.

## Verification

Seventy-two workspace tests passed, including five new temporal contract and
integration tests. Tests cover clock jumps, missing/overlapping timing, image
ordering, gaps, source separation, type/resource limits, immutable replay,
late arrival, changed hashes, failed persistence, resize, batch archive and
process exit. The two temporal integration tests were rerun after adding
memory-read clock bounds. Strict Clippy and formatting pass.

The native test uses an owned WinForms window changing between two shapes.
The automated observer decodes images returned by actual stdio MCP, verifies
their hashes and compares the pixels. It receives no shape geometry or semantic
truth. It detects a difference and return to the original pixels, then records
that limited interpretation through the engine. This is protocol/plumbing
evidence, not object recognition. It uses no game input or manual screenshots.

UI Automation verified the appearance card and its Inspect explanation. The
test also rejected wrong hashes, conflicting replacements, reversed images and
duplicate image identities; an exact observation retry was a no-op.

Reproduce using `scripts/appearance-fixture.ps1` and
`scripts/temporal-smoke.py`, with an isolated engine on the same Windows desktop.
Raw local evidence is retained under `test-results/temporal-v1/`.

The final native rerun also verified that memory reads follow image operations
on the same clock. Restart preserved the temporal evidence exactly as published
while leaving no active target connection (`restart-proof.json`). The published
23-tool practice/MCP smoke passed in `test-results/smoke-a56f4bf3/report.json`.

Preview `0.1.0-20261020-235611` is running on the normal desktop. Its published
executables match the tested release; model hashes and the launcher were
verified. All seven prior history entries were retained. Delivery proof is
`test-results/temporal-v1/desktop-delivery.json`. Owned Anode test processes
were closed, their job reached a terminal state, and the lease was released.
