# Multiple automatic numeric checks

Reversebolt now freezes and checks several findings during the normal automatic
observation pass. Up to eight series share 200 candidate interpretations in total;
identical address/width reads are deduplicated. A series that exceeds remaining
capacity is deferred whole, preserving its competing explanations. Selection
still ranks investigation cost, not semantic importance or a game's labels.

Each check retains its own source, track ID, immutable mappings, fresh outcomes
and provenance. Cards use their matching check rather than a global selected
one. All checks freeze before new observations; a trial write invalidates all
passive checks because effects can cross values. Other prepared plans based on
that evidence expire before entering the write backend. This does not prevent
idempotent completed calls or guarded restoration of the original trial.

Current status exposes `live.prediction_checks`, a list. The historical evidence
file is `prediction-checks.json`, schema version 2. Old saved views with a single
`prediction_check` object or null still load; no process capability is restored.
The initial migration test caught Serde's untagged-buffer limitation with u128
timestamps; the implementation now reads the JSON shape directly and tests both
legacy and current records. The frozen native cases below used the corrected
implementation.

## Fixed native verification

The [method](PROTOCOL.md), [configuration](protocol-v1.json), owned fixture, runner
and changed product sources were frozen before either case. Protocol SHA-256:
`86a3bbd01cc30e6536400b4b6b93263bbee615a2fc57d96f62cde8fe37721407`.
Both cases use preview `0.1.0-20261021-091951` in fresh Anode processes. The fixture
has two independently varying floats at randomized addresses. Only the evaluator
knows their source layout; it never passes addresses to discovery.

| Case | Automatic checks | Explicit result | Other prepared plan | Restoration |
|---|---|---|---|---|
| Upper card selected | Two; one candidate each | Quantity 50 → 53; Reserve stays 43 | Expired without a write receipt | 50, 43 |
| Lower card selected | Two; one candidate each | Reserve 43 → 46; Quantity stays 50 | Expired without a write receipt | 50, 43 |

Each completed the default sixteen-capture job without manual capture, correlate
or prediction-check commands. Both checks froze at capture twelve; each retained
four agreeing, bracketed fresh comparisons without refitting. The two card scopes
referred to distinct check IDs. The selected byte write and its restoration had
successful native receipts; the visible result matched with expected complete
bytes at both capture endpoints. Both passive checks were invalidated after the
intervention. No failed case, candidate replacement or rerun occurred.

A separate read-only [accessibility observation](ui-accessibility.json) during
case zero showed Quantity and Reserve as separate cards, both arrange handles,
the selected card's response/restore actions and the other card's unapplied
status. This was a post-hoc observation with two accessibility warnings about
changing/unreadable controls. It verifies the reported visible names, states and
bounds, not all visual styling or execution of the UI buttons. No screenshot was
manually inspected and no game was controlled.

Jobs `j_ad6ebfb1b23a4e7093ed6c3ac59ddd35` and
`j_4ae868d4b9894328a4c7a4b3321a8a29` both exited zero. All six owned fixture,
engine and MCP process IDs were absent at cleanup; the Anode lease was released.
The [post-run audit](verify.py) checks hashes, identities, immutable mappings,
shared read bounds, fresh timing witnesses, planned patches and operation results.
[Compact evidence](evidence.json) omits duplicated input contexts; full raw
records remain in `test-results/multiple-checks-v1/`.

## Tests, delivery and remaining scope

All **158 workspace tests**, strict Clippy, release build and packaged checks pass.
Tests cover multiple independent tracks, contradiction isolation, source/track
identity, shared reads across OCR/external checks, late observations, the eight-
check and shared candidate limits, preservation after a failed second save,
blocked refitting, invalidated-plan expiry and legacy/current serialization.
Packaged 31-tool evidence is `test-results/smoke-ec1149a0/report.json`; logs use
`test-results/multiple-checks-{tests,clippy,build,smoke,audit}.log`.

The desktop preview is running with all eight history entries and the
`qwen3.8:latest` preference preserved. Executable SHA-256:
`853653bfa0a9c515c45c817b46461131464f1a57f702324477e70d21ee567a9b`.
Delivery receipt: `test-results/multiple-checks-desktop-delivery.json`.

Progress moves from **27% to 28%**, crediting general discovery from 5/20 to 6/20
for bounded simultaneous investigation with native card/operation evidence.
Other weights are unchanged: foundation 4/10, native access 6/10, observation
5/15, validation 2/15, intent 0/10, agent collaboration 5/10 and generalization
0/10. These are controlled fixtures, not independent games or semantic controls.
The method does not establish gameplay authority, continuous state retention,
hidden effects, complete recovery or reusable knowledge. Further trials still
need fresh evidence after intervention. The full objective remains active.
