# Evidence references instead of copied citations

Automatic naming now asks the model to select evidence entries printed beside
complete field labels, type labels and same-object text. The engine owns the
table, binds it to the exact report and rows, resolves the selected IDs to the
original canonical citation, then applies the existing validation. It never
infers an index, repairs an unknown ID or borrows another candidate's evidence.
Manual MCP submissions still use the canonical citation format.

The evidence table is bounded at 8,192 entries and 2 MiB. Labels are complete;
overflow fails explicitly. Repeated identical strings keep their own sources and
object-text indexes. A valid reference still does not establish a field's actual
effect, owner, liveness, bounds or edit authority.

## First frozen trial: improved attribution, a naming regression

The seven-case first trial used the unchanged local model and saved inputs.
The previously failed eight-candidate relevance batch now resolves its citations
correctly. Reversing its order yields the same categories: three plausible
gameplay parameters, two application settings, one implementation detail and two
unclear fields. All four saved Vampire candidates are plausible gameplay; all
four become unclear when their meaningful labels are removed. These are
development classifications, not independent semantic validation.

The owned fixture retains its deterministic object-text question, `Speed`.
However, full Vampire and menu naming stop after 20 and 10 model requests:
the model uses evidence entry IDs as alternative candidate IDs. Four alternatives
are outside their groups (115, 157, 102 and 94). The engine rejects the runs and
publishes no batch. No citation ownership error occurred in those responses.
The staged preview `0.1.0-20261023-092222` was **not activated**.

The complete failure remains in
`test-results/evidence-references-v1`, with frozen source, executables, model,
schemas, raw requests/responses and input hashes. [First results](results.json)
and [protocol](PROTOCOL.md) are retained. Its artifact audit passes, including
rejection of incomplete naming; that does not mean its naming behavior passes.

## Follow-up: make the group boundary explicit

The generation schema now restricts both the primary candidate and every
alternative to candidate row IDs in the current group. The prompt explicitly
distinguishes alternative row IDs from evidence entry IDs. The existing runtime
group check remains; neither namespace is inferred or repaired after generation.

The [follow-up protocol](GROUP-PROTOCOL.md) freezes three fresh full naming runs
and checks the four retained invalid alternatives with an independent JSON Schema
validator. The relevance implementation is unchanged and its four first-trial
requests are reused as evidence rather than repeated. Follow-up results and
package verification are recorded separately in `group-results.json`.

All three follow-up naming runs complete: fixture 1 retained question / 2 model
calls; Vampire 6 / 25; menu 5 / 16. No generated proposal is rejected by the
lexical validator in this follow-up. Vampire retains `move speed`, `max hp`,
`startingSpawns`, `destructibleFreq`, `move speed multiplier` and `power`.
These are still unverified questions. The menu proposes `global noise scale`,
`dithering factor`, `Raycast Max Dist`, `Spherecast Radius` and
`joystickAxisSensitivity`: a mixture of rendering, interaction and input
parameters, not evidence of resolved gameplay relevance.

The independent schema audit rejects all four first-trial invalid alternatives.
Offline checks resolve every fresh reference and reproduce the canonical batches
and fixture fallback without inference. Both frozen artifact audits pass.
[Follow-up results](group-results.json) retain names, errors, hashes and limits.

All 510 ordinary workspace tests pass (one opt-in archive test remains ignored),
along with strict Clippy, formatting and release builds. Preview
`0.1.0-20261023-093127` passes packaged headless dispatch, the 65-tool MCP catalogue
and unchanged manual submission schema. Its exact helper passes seven owned ABI
cases: 51 calls, 90 allocation-free recorded pauses, four restored writes and
four readonly write refusals. This checked preview is selected by the desktop
shortcut; the existing editor window is preserved.

Relevance remains research-only, without hiding candidates. The earlier
order-sensitive aim-sensitivity classification remains unresolved. Progress
stays **38%**: this work improves evidence handling but establishes no new causal
gameplay effect or independent-game generalization. No game, game input or
screenshot was used; native regression writes affect only the owned fixture and
are restored.
