# Observed route recovery

The shared core can look for a previously observed rooted member route in a new
validated discovery. It requires the same runtime module location, root catalogue
fingerprint and typed member structure, then returns the candidate produced by the
new graph. It never substitutes a new process identity into the old path, copies
an old address, or transfers edit authority. A match still requires fresh reads,
an effect comparison and restoration before reusing the earlier effect claim.

This is a shared Rust API and offline CLI, not automatic desktop/MCP recovery.
All 485 workspace tests and strict workspace Clippy pass. New regressions cover
fresh process/address/value identity, changed catalogues/layouts, absent routes
and invalid graphs after an apparent early match. Formatting also passes.
The CLI is `cargo run -p reversebolt-core --example runtime_recovery -- OLD_REPORT
PATH_ID NEW_REPORT NEW_OUTPUT`. It reads bounded saved reports and creates a new
output file. Changed runtime/catalogue identities and missing observations remain
unresolved; a malformed graph returns an error even if an earlier node matched.

## Evidence on 2026-10-23

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

The next step is an explicit, tested compatibility policy for reader changes,
followed by a separately frozen fresh-process effect comparison. Matching a route
alone cannot establish persistent actor identity, exact gameplay semantics or
generalization to unfamiliar games. Progress remains **36%**.
