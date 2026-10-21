# Rechecking known findings

The desktop now offers **Recheck findings**, backed by the same MCP operation.
After a trial is restored, known hypotheses can be revisited without another
broad memory scan. Every interpretation stays unchanged; new checks have new
identities, empty samples, and a link to the archived evidence. They still need
fresh holds and changes before another trial can be prepared.

This is same-process re-observation at existing addresses. It does not recover
relocated objects, survive restarts, establish allocation continuity, infer a
gameplay role or prove authority. All old rivals—including contradictions—remain
available for the new check. Missing mappings refuse the transition; changed
geometry invalidates it. Applied or uncertain trials block the transition so
that the current restoration scope is not lost.

## Frozen native runs

Protocol SHA-256:
`3dfcf6255418dc7eac6f71575fa476effbdbbb5a2a740c488016fcc42dae3444`.
The protocol, runner, fixture and source snapshots were frozen before either run.
Both used `dist/0.1.0-20261021-094100` in Anode session 2, with fresh processes,
random field addresses, no manual captures or game input, and no model calls.
The evaluator's private oracle was never given to the product.

| Case | Recorded result | Strict runner result |
|---|---|---|
| 0, fresh contrast | Discovered two values; edited/restored the upper; automatically rechecked both; edited the lower from 43 to 46, observed the effect and restored 43 | Passed |
| 1, constant fresh values | Discovered two values; edited/restored the lower; automatically rechecked both; preparation for the upper was refused after four constant readings | **Failed at error-wording assertion** |

The second runner expected the phrase “contrasting evidence.” The product
returned `This location has overlapping interpretations that still need fresh
evidence.` with `ok:false`, and no batch-2 edit record. Both fresh series had four
correct readings—50/50/50/50 and 43/43/43/43. This is evidence of the intended
refusal, but the original failed result is retained. No replacement run or
post-failure oracle assertion was performed. The post-run audit records this
distinction rather than converting the strict run to a pass.

Each fresh batch planned eight bytes and requested 64 bytes over four moments,
including the reads before and after images. Original fitting mappings and
addresses were preserved. Rechecking before restoration and stale batch retries
were refused in both cases. Raw evidence is in
`test-results/recheck-findings-v1/case-{0,1}`. Compact findings and raw hashes are
in `evidence.json`; `verify.py` is an explicitly post-run audit.

Anode jobs:
- `j_0c497acbfe6441feb2ec9de880893800`: exit 0, 80,938 ms.
- `j_8ac0cda49744461793e33765d59fa528`: exit 1, 72,130 ms.

All six owned app/fixture/MCP processes were confirmed absent, and the lease was
released. No manual screenshots or UI input were used. This verifies the shared
engine/MCP workflow, not invocation of the desktop button or independent games.

## Product verification and delivery

All **162 workspace tests**, strict Clippy, release build and packaged smoke pass.
The package exposes 32 MCP tools. Contract tests cover repeated restored trials,
all-rival retention, old-frame rejection, OCR/external series, shared reads,
constant observations, fresh contradictions, geometry/mapping loss, stale edits,
archive/write failures and read limits. The automatic recheck plan contains four
full captures without model calls or refitting.

Packaged smoke: `test-results/smoke-fa175690/report.json`.
Desktop receipt: `test-results/recheck-desktop-delivery.json`.
Preview `0.1.0-20261021-094100` was started on the user's desktop, preserving all
eight history entries and the `qwen3.8:latest` observer preference.
Overall weighted progress stays **28%**; the full goal remains active.
