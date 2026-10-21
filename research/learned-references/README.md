# Learned references for moved values

Reversebolt now searches complete fitting snapshots for stable one-hop references
to its numeric hypotheses. It learns the offset from bytes and addresses, without
game profiles, source layouts or supplied pointer roots. **Recheck findings** can
use these references to propose a new location after an allocation moves.

New locations receive new prospective evidence using the original numeric mapping.
Old candidates remain as rivals, including unavailable ones. A reference-derived
location needs at least four timed agreements, a hold and a change. Pointer reads
enclose the value reads and captured image. Trial models carry the reference into
preflight checks for both application and restoration. A later move returns an
error before writing; the earlier operation record remains available.

The bounded search supports aligned little-endian 64-bit references with offsets
from 0 through 4096 bytes. At most four stable routes per interpretation are kept;
the limited flag records truncation. This is incomplete search, not verified
pointer semantics, object identity, allocation continuity or gameplay authority.
Multi-hop chains, x86 pointers and cross-process/restart locators are unfinished.
Sequential guards cannot exclude ABA changes or delayed display copies. The
existing 200-candidate and investigation read/image/archive limits still apply.

## Frozen native results

Protocol SHA-256:
`3315bd214b464780b3c8e89e4fae05c3aaabdb370ccac33d9e42e7e3d5e8e7dc`.
Method, source and binary hashes were frozen before the two predetermined runs.
Both used `0.1.0-20261021-100712`, fresh processes and random addresses/offsets in
Anode session 2. The evaluator oracle stayed outside product inputs. No manual
captures, game/UI input, screenshots reviewed by a model, or local-model calls.

| Case | Observed result | Strict result |
|---|---|---|
| 0, intended edit/restore after a move | Learned the actual root and offset 2680; final initial reading was missing, so the runner stopped before moving or editing | **Failed**, retained |
| 1, move again after preparation | Learned root and offset 2896; found the field at its new allocation, checked four fresh moments, prepared the selected location test, then blocked application after another move | Passed |

Case 1 selected the first reference-derived candidate, without using the oracle
to choose it. Its patch was then checked against the private oracle. All four
pointer/value/image brackets were ordered and stable. The original allocation
had been freed, and its candidate remained unavailable; the new location was
tested through the explicit location-probe path. Before application the fixture
moved again while retaining the just-tested allocation and unchanged bytes.
The reference guard refused the stale write. Both old and current fields remained
50. Windows reused the first allocation address on this second move; that did
not make the prepared plan for the intervening address valid.

Case 0 is not counted as a successful relocation or native edit. It retained
15 correct initial readings and a missing sixteenth. A post-run pixel-statistics
audit found the last image nearly uniform at the fixture's background color:
RGB variances about 0.058, 0.058, 0.060, versus 856, 846, 806 in frame 15. The
last OCR output was empty. No manual image review or replacement run occurred.
This points to capture stability as the next issue to investigate. Neither native
run exercised a successful edit through a learned reference; contract tests do
exercise that path, but cannot substitute for the missing native positive case.

Raw evidence: `test-results/learned-references-v1/case-{0,1}`.
`evidence.json` retains compact results, ordered witnesses, pixel metrics and raw
file hashes. `verify.py` was written after the runs and is an integrity/classification
audit, not a new acceptance protocol.

Anode jobs:
- `j_4fb687190ddf48999e978e6077d06413`: exit 1, 56,581 ms.
- `j_2c2b67777b134c7dbfcb78285a4e1adc`: exit 0, 73,355 ms.

All six owned fixture/app/MCP processes were confirmed absent; the lease was
released. The original failed method and evidence are unchanged.

## Verification and delivery

**167 workspace tests**, strict Clippy, release build and packaged smoke pass.
The new contracts cover stable reference learning and limits, moved fields with
retained/freed old allocations, rival retention, fresh comparisons, edit/restore,
reference changes during capture and before writes/restoration, and read-budget
refusal. Source and tests are original Rust; no Squalr implementation was copied.

Packaged smoke: `test-results/smoke-54227a6f/report.json` (32 MCP tools).
Desktop receipt: `test-results/references-desktop-delivery.json`.
Preview `0.1.0-20261021-100712` is running on the user's desktop with eight history
entries and the `qwen3.8:latest` preference preserved.

Weighted progress is **29%**: intent/editing receives its first 1/10 for learned
location reuse and bounded same-process revalidation, supported by native moved-
location preparation and stale-reference refusal. Other credits are unchanged.
This adds no independent-game, restart-recovery or gameplay-authority credit.
