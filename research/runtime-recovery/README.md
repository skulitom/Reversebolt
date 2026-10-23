# Observed route recovery

The shared core can look for a previously observed rooted member route in a new
validated discovery. It requires the same runtime module location, root catalogue
fingerprint and typed member structure, then returns the candidate produced by the
new graph. It never substitutes a new process identity into the old path, copies
an old address, or transfers edit authority. A match still requires fresh reads,
an effect comparison and restoration before reusing the earlier effect claim.

An explicit second policy, `runtime_recovery::propose`, compares declared route
structure while retaining the strict result. It requires equal qualified owner
and declaring types, shapes, unique field names, representations, flags, embedded
type names and array indices. It does not use offsets, field ordinals, layout
hashes or old values as identity. Only one matching fresh candidate can become a
proposal for revalidation; zero or multiple matches publish no candidate.

These shared APIs now also support durable recipes, automatic desktop discovery
and explicit MCP recovery. See [the product integration and its evidence](PRODUCT.md).
All 494 ordinary workspace tests and strict workspace Clippy pass. New regressions cover
fresh process/address/value identity, changed catalogues/layouts, absent routes
and invalid graphs after an apparent early match, plus changed ordinals, duplicate
routes/names, array positions and representations. Formatting also passes.
The CLI is `cargo run -p reversebolt-core --example runtime_recovery -- OLD_REPORT
PATH_ID NEW_REPORT NEW_OUTPUT`; insert `--propose-declared-route` before `OLD_REPORT`
to explicitly use the second policy. It reads bounded saved reports and creates a new
output file. Changed runtime/catalogue identities and missing observations remain
unresolved; a malformed graph returns an error even if an earlier node matched.

## Strict baseline on 2026-10-23

An exploratory replay matched the prior Viewfinder route in two archived schema-4
reports from different processes. It used the later positive comparison as the
source and an earlier report as the destination. This retrospective result is not
fresh recovery or a repeated effect. Files are retained under
`test-results/runtime-recovery-development-v1`.

A separately frozen prospective run automatically selected the sole completed
suppression-and-return comparison, backed up storage, and started one fresh game
process in hidden Anode. Its runtime discovery completed without model calls or
measurement images. Strict recovery returned **catalog_changed** and stopped
before preparing a test or writing a gameplay value.

The prior report uses schema 4 with 6,398 roots; the fresh report uses schema 5
with 7,427 roots. Both the overall catalogue fingerprint and the selected types'
layout fingerprints differ. The recorded fields' shared descriptors agree, while
the newer reader includes additional inline/declared-type metadata. This is a
diagnostic lead for a separately specified migration policy, not evidence that
silently ignoring fingerprints is correct. No migration was attempted and the
original failure is retained.

The audit replays the frozen recovery executable against saved reports and checks
source/binary hashes, report provenance, completed job state, absence of trials,
helper unload and cleanup. The one helper unloaded; all 252 recorded pauses have
zero helper allocations/deallocations while paused. Setup used two viewed images
and one Continue click. No measurement image was captured or manually viewed.
The main save and backup bytes are unchanged; normal log/analytics files changed.
Owned test processes exited and Anode was released. The user's main editor remains
open on its existing build.

- [Frozen prospective protocol](PROTOCOL.md)
- [Offline audit](audit.py) and [audited result](results.json)
- Native artifacts: `test-results/runtime-recovery-live-v1`
- Prior effect: `test-results/viewfinder-followup-v1/comparison-2.json`

## Fresh declared-route revalidation

The first attempt timed out before recovery. Inspection later produced a complete
report, but the loader had already stopped at its 40-second limit and left the
helper loaded. No catalogue or trial was accepted. The owned process exited;
its error and uncertainty fence remain recorded.

The helper now buffers report output in 64 KiB batches and explicitly checks
flush errors. It previously streamed each small serializer fragment directly to
the file. Tests verify identical bytes, more than a thousandfold fewer underlying
writes for a representative document, and propagation of write/flush failures.
No timeout or managed-world pause limit changed. The next fresh native discovery
completed and unloaded in 8.470 seconds. This is a development observation, not a
controlled cross-run performance benchmark.

That fresh process produced exactly one structural proposal while strict recovery
still returned `catalog_changed`. An evaluator selection error stopped before any
edit. A separately frozen continuation verified the unchanged live process and
catalogue, selected the exact proposed ID through the existing search tool, and
performed one newly prepared comparison through MCP. The old question, action
sequence and zero test value were reused; the path, reads, baseline and restoration
were fresh. The original `3.5` bytes were restored exactly.

| Image difference median | Baseline | Changed | Restored |
|---|---:|---:|---:|
| Product metric | 14.0947 | 0.1397 | 14.0951 |
| Independent full-frame gray metric | 14.1229 | 0.5749 | 14.1246 |

All 18 images and 12 action holds are retained and checked automatically. No
measurement image was manually viewed and no model call ran. All seven helpers
unloaded; 258 recorded pauses show zero helper heap operations while paused.
Both saves are unchanged, owned processes exited, and Anode was released. The two
fresh launches used four viewed setup images and two Continue clicks in total.
The continuation used no further setup image or manual input. Both failed
attempts and the successful continuation remain in the evidence chain.

- [Frozen declared-route protocol and continuation](DECLARED-PROTOCOL.md)
- [Audit](audit-declared.py)
- [Timed-out attempt](runtime-recovery-declared-v1-results.json)
- [Selection failure after successful discovery](runtime-recovery-declared-buffered-v1-results.json)
- [Successful comparison](runtime-recovery-declared-continuation-v1-results.json)

This demonstrates reacquisition and revalidation of one action-specific rendered
effect across a process restart and reader change in a known development game.
It does not establish persistent actor identity, exact gameplay semantics,
physical speed units, complete world rollback or unfamiliar-game generalization.
Progress is **37%**: intent/editing rises from 1/10 to 2/10 for this reuse and fresh
revalidation; all other credits stay unchanged. Next: retain a portable learned
route in durable product state and expose reacquisition through the shared
engine/MCP and desktop, without transferring stale effect authority.

Preview `0.1.0-20261023-063356` includes the output-buffering change and is selected
for the next desktop launch. Its packaged headless startup and 64-tool MCP
catalogue checks pass; the user's existing editor remains open. The native game
experiment used the separately frozen library release build. The packaged helper
is the workspace release build of the same source, with a different binary hash;
the packaged check did not attach a game. Receipts are under
`test-results/runtime-recovery-release-smoke` and `runtime-recovery-release-catalog`.
