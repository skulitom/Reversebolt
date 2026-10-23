# Metadata layouts on demand

Preview `0.1.0-20261023-074606` replaces eager loading of every declared type's
layout with cheap root eligibility followed by complete layouts on demand. This
addresses the failure mechanism found in the independent Vampire Survivors run.
That original failed run remains intact; no new game compatibility is claimed.

The native reader now streams field flags when finding potential static root
holders. It skips full names/layouts for unrelated classes. Eligible root holders
still receive the complete ordinary layout and static-storage checks. Encountered
object types, parents and inline dependencies are prepared outside managed pauses,
then traversal starts from fresh roots again. Discovery allows enough preparation
passes for its existing depth bound. No old object address becomes authority for
the next snapshot.

The loaded-layout limits remain 200,000 fields and 50,000 classes. Cheap root
eligibility has separate limits of 2,000,000 field headers and 50,000 declared
classes. The existing 20-second metadata and 5 ms managed-pause limits remain.
Successful, refresh and catalogue-error reports retain policy, counters and bounds.
Root inventory completion is explicitly distinct from graph coverage. A field
count exactly at the loaded limit can finish its current layout; the next actual
field is refused rather than treating the terminating iterator as another field.

The lightweight root pass is still one bounded pass. This is not a persistent,
resumable catalogue of arbitrarily large metadata. Existing root continuation and
fresh typed-path checks remain; games beyond the new header/class/time bounds can
still fail. No game-specific fields, profiles, names or source layouts are used.

## Verification

The [protocol](PROTOCOL.md) defines an owned ABI fixture with a configurable
rootless literal-field class before ordinary roots. Its optional
`large-metadata` feature leaves the original fixture mode available. The
[runner](native.py) freezes old/new readers, fixture, parser, patch tools and
sources before each matrix. The [audit](audit.py) rechecks retained data without
native execution, image inspection or inference.

Both debug and packaged-reader matrices passed seven cases:

| Case | Old published reader | New reader |
| --- | --- | --- |
| Small fixture | 23 candidates | Same 23 routes and values |
| 200,001 unrelated fields | Field metadata budget failure | Same 23 routes; 17 loaded fields |
| Exactly 2,000,000 root headers | Not rerun | Same 23 routes and values |
| One header above that limit | Not rerun | Explicit header-budget failure |
| Required layout above 200,000 fields | Not rerun | Existing loaded-field budget enforced |

The successful cases retain identical root fingerprints and complete typed field
routes. Each checks embedded values, arrays and enums, makes one guarded edit and
restores its exact original bytes, refuses a readonly write, and rejects null or
changed-layout paths. The packaged matrix has **51 native calls, 90 managed
pauses, four restored edits and four readonly refusals**. All retained pauses
record zero helper allocations/deallocations, and mock stop/resume counts balance.
Its maximum recorded stop-to-resume duration was 24 microseconds; this mock timing
does not predict real-game pause cost. Deferred preparation increased small-case
pause count from 12 to 26 across its twelve operations, a tradeoff to measure in
real games.

All **494** ordinary workspace tests, whole-workspace strict Clippy and formatting
pass; the pre-existing opt-in archive replay remains excluded from the ordinary
suite. The fixture also passes strict Clippy. The packaged preview passes isolated
headless startup/command dispatch and the **65-tool** MCP catalogue check.

[Results](results.json) link the two local artifact directories. Direct native
calls ran only in owned subprocesses: these checks establish neither remote
loader unload nor any actual game's gameplay effects. No game, desktop input,
screen capture or model was used in this implementation turn. The desktop
shortcut selects the checked preview; the user's open editor was preserved.

Progress stays **38%**, including **0/10** for independent-game generalization.
Next: separately freeze a native evaluation of this reader, check catalogue
coverage and latency, and establish a gameplay baseline before any effect test.
The earlier Vampire Survivors audit now defaults to frozen sources so that its
failed baseline remains auditable after product changes; `--verify-current`
explicitly requests the original working-source equality check.
