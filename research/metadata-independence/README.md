# Discover properties before interpreting the current clip

Metadata discovery no longer requires a property to explain the current visible
response. Type selection and per-type field questions now run before the
separate image interpretation. Hidden, inactive and nonvisual properties remain
eligible. The image description is still retained with its original source
hashes and uncertainty for subsequent investigation, but is not inserted into
metadata prompts. Existing citation checks and edit guards are unchanged.

The prior grouped prompt demanded an observed response even on its metadata-only
path. It also advertised twelve questions while a group could accept only one or
two. The revised wording uses the actual group budget and asks for potentially
useful labelled properties. This removes a discovery prerequisite; it does not
establish that a label is correct, that the value controls anything, or that a
comparison will activate the relevant behavior. No card is renamed or promoted.

The offline observer runner accepts `--activity MANIFEST`, containing a retained
`source` and exactly four PNG paths in `frames`. It validates the 64 KiB manifest
budget, source timing/geometry and image hashes before connecting to the model.
This runner reads saved evidence and has no target-process access. The desktop
executable quarantined during the previous native run was not restored, copied,
recompiled or executed as part of these replays.

## Frozen paired development replay

The [protocol](PROTOCOL.md) was saved before evaluation. Baseline and revised
observer binaries, source snapshots and report hashes are retained under
`test-results/metadata-independence-v1`. Each case ran once per version; an extra
revised case included the original four images from the latest saved run. No
prompt was changed after inspecting the revised results. No game was launched,
no gameplay input or memory edit occurred, and no image was manually reviewed.

| Saved evidence | Baseline accepted/rejected | Revised accepted/rejected |
|---|---:|---:|
| Latest Manifold gameplay report | 0/0 | 10/1 |
| Earlier Manifold gameplay report | 0/0 | 10/1 |
| Menu report | 0/0 | 0/2 |
| Owned context fixture | 0/0 | 0/0 |
| Earlier gameplay with opaque research labels | 0/0 | 0/0 |
| Latest report plus saved activity images | — | 10/1 |

Each gameplay result contains ten distinct declared fields across seven types,
including movement, growth, carrying and filling parameters. Those are model
questions from observed labels, not verified gameplay controls. For example,
some explanations speculate about player relationships and physical effects
that the evidence does not establish. The menu's two proposed graphics settings
were rejected for invalid alternatives, not because a semantic classifier proved
them irrelevant. The context fixture still abstains before its useful same-object
text reaches field selection. Opaque abstention is limited to this case.

Adding the saved activity images produced identical metadata prompts, schemas,
options, selection and candidate/alternative references. The independent image
interpretation still described an uncertain, subtle perspective shift. Six
accepted questions had different generated wording despite the fixed seed and
temperature. The first audit incorrectly required an identical full batch and
failed; that failure and both original outputs are retained. The corrected audit
reports matching references and wording variability separately. No inference was
repeated to obtain a preferred result.

Four additional malformed-manifest runs reject wrong image count, reordered
images, a mismatching image hash and an oversized document before inference or
output creation. All eleven replay runners and the four validation runs exited.
Seventeen observer tests, strict workspace Clippy, formatting and the offline
release build pass. The [artifact audit](results.json) checks frozen inputs,
receipts, provenance, original images, reference equality and process outcomes.

These are development replays on previously explored reports, not held-out
games or causal validation. Progress remains **35%**. Desktop preview
`0.1.0-20261021-225832` remains unchanged while the Windows security detection is
unresolved. Next: make same-object hints available during type selection and
choose explicit test conditions for broad candidate properties, without making
current visibility a prerequisite for discovering them.
