# Discovery direction — 2026-10-21

Status: bounded IL2CPP discovery and refresh are integrated into the desktop and
MCP, with automatic discovery on supported attachment. General causal controls
do not yet exist. The full goal and 31% weighted estimate
remain unchanged. The user's requirement is named, understandable, interactive
cards for humans and agents, including values and rules that never appear as HUD
numbers. Generalization excludes per-game profiles, offsets, signatures and
source-derived game layouts.

## Diagnosis

Reversebolt already combines native memory, images, temporal observations,
input context, frozen hypotheses and guarded trials. Nevertheless, its useful
live numeric path originally started with readable screen numbers and sought
correlated byte sequences. That remains the fallback for unsupported runtimes;
supported attachment now supplies runtime-derived candidates without images.
The old path biases discovery toward visible counters and presentation copies.
More OCR accuracy alone cannot reveal an invisible damage multiplier or prove
that editing a correlated field changes gameplay.

The latest independent-game watch study exposes two specific problems: an
evolving 2048 field continues matching its frozen mapping while exact requested
bytes stop persisting, and a four-moment Tetris selection abstains for insufficient
repeated contrast. Neither an exact-value match nor a continued correlation
proves causal authority. See [retained evidence](../research/game-change-watch/README.md).

## Recommended discovery architecture

Use several candidate producers through one shared investigation API:

- Runtime structure, where available: detected runtimes, assemblies, types,
  fields, live objects and references. Preserve exact provenance and supported
  runtime versions. Do not assume a field named Health belongs to the player or
  controls health. Metadata is a source of candidates, never edit authority.
- Memory behaviour: changing and stable regions, repeated object shapes,
  relationships and event-linked changes without requiring an OCR number.
  Retain the native fallback for opaque or unsupported runtimes.
- Observation: text, objects, motion, appearance, inputs and useful audio.
  Use these for context and consequences; they must not gate all discovery.
- Active validation: choose experiments that distinguish competing objects or
  fields, examine downstream behaviour and changing mappings, preserve failed
  predictions, and revalidate after scene/load/restart changes. Establish paired
  or otherwise discriminating evidence before claiming an effective control.

An agent should receive structured candidates and evidence, decide which test
would reduce ambiguity, and use the same bounded operations as the desktop.
Image/model calls should be occasional and purpose-driven. Discovery success is
a useful, validated control, not a large set of correlations or readable labels.

## Runtime-aware feasibility and next work

Installed Blue Prince uses IL2CPP, so the first feasibility probe targets its
exported runtime APIs instead of the initially proposed Mono/.NET path. The
unchanged Rust probe listed 87,225 fields and read 989 scalar storage locations
twice in the real game, including 38 in its main code image. It used zero images
and no game-specific filters. Both helpers unloaded. This is explicit temporary
instrumentation, not a passive remote-memory API or a production feature.
See [method, evidence and limits](../research/runtime-probe/README.md).

A subsequent bounded object reader now follows supported static roots and
reference fields during brief managed-thread pauses. In Blue Prince it read
1,871 instance scalar values per snapshot, including 45 in the main game image.
The owned stress fixture demonstrated reference replacement and address reuse;
object IDs deliberately belong only to one snapshot. Both real-game snapshots
hit the 1,024-node limit; 492 objects had uncatalogued runtime types, and the full
root queue prevented discovering additional child objects.
See [object evidence and contract](../research/runtime-objects/README.md).

A reusable runtime component now batches roots and describes encountered types
between fresh snapshots. One unchanged Blue Prince request read 4,267 supported
root slots and reached 7,747 non-root node occurrences, including 26 deeper
main-game scalar reads. It also read text and supported arrays without images.
These occurrences can repeat between batches; they are not distinct controls.
Coverage remains bounded and partial. At that evaluation the component was not
connected to app/MCP dispatchers. See [traversal results and limits](../research/runtime-traversal/README.md).

The shared core now derives typed scalar paths, and the runtime reader can
resolve them afresh with type/layout and process checks. Owned tests cover
replacement, reordering and structural refusals. Four paths refreshed in Blue
Prince, but the fixed selection picked unchanged framework fields. This exposes
candidate usefulness as the next bottleneck; those paths establish no gameplay
authority. See [path evidence](../research/runtime-paths/README.md).

Discovery/refresh now reach the shared dispatcher and simple cards. A frozen
product evaluation published 64 Blue Prince cards without images and refreshed
four non-core declarations, but those selections still described flags and
bookkeeping; the main-game image cards were localization state. Structural
diversity alone does not identify useful gameplay state. See
[product workflow evidence](../research/runtime-product/README.md).

The catalogue now supports bounded search before the display limit, including
direct same-object text. A fresh Blue Prince run searched 27,052 candidate paths,
selected four beyond the first 64 and refreshed them without images. However,
those four still concerned input/rendering internals. The saved exploratory
`RookPos` query finds four paths that the automatic selection still omits, even
after preferring text context. Search makes evidence accessible; it does not
solve automatic relevance or semantic inference. See
[context/search evidence](../research/runtime-context/README.md).

An exploratory metadata query found a first-person controller with walking,
running, jumping and gravity fields. Its Active reference was read as null in
the menu, and no instances of that exact type were observed. Before expanding
the reader, evaluate a fresh gameplay catalogue and targeted behavioural reads.
Two bounded setup attempts could not enter gameplay because Anode's GameInput
helper blocked focus/input; they establish no control effect. This input issue
does not establish a memory-reader failure. See the
[gameplay-state investigation](../research/runtime-gameplay/README.md).

Next select by object context, relationships and behaviour, and validate useful
effects. Neither snapshot
addresses, stable path IDs nor field names establish
persistent identity or edit authority.
Static values may be defaults from uninitialized classes. Keep automatic reads
separate from mechanisms that execute code in the target, and expose unsupported
paths. Runtime structure should get priority when available; memory-change
discovery remains necessary where metadata is absent or incomplete. Images,
motion and sound provide context and outcome evidence without gating discovery.

Supporting a runtime does encode platform knowledge, much like supporting PE or
numeric encodings. It must not encode particular game classes, field names or
addresses. Unity Mono and IL2CPP require distinct treatment; stripped or
obfuscated names and changing runtime versions remain coverage limits. Later
Unreal reflection support is a separate adapter, not a generic promise to read
all shipped Unreal state. Evaluate usefulness against the raw-memory path using
unchanged adapters on independent software, including hidden values.

The next acceptance milestone is a useful hidden control with a refreshable
reference and an independently checked effect. The existing memory engine, change
journal, MCP authority and card interface should be reused. Compare approaches
by validated controls, incorrect labels, time to a useful result and runtime/model
cost. Raw field counts and screenshot counts are not success metrics. Improve
adaptive observation and dynamic mapping checks in support of this direction
rather than optimizing visible-counter matching as the sole route.

## Decision after the visual-only concern

Use runtime and memory evidence to propose candidates independently of screen
numbers. Use selected images, movement, appearance and other observations when
they can distinguish consequences. A model should propose hypotheses and tests
from compact structured evidence; its confidence or a plausible field name must
not authorize a gameplay label or establish a successful edit. The same
investigation operations and evidence must remain accessible through MCP and the
desktop. This is a prioritization decision, not a claim of measured optimality.

The next product acceptance target is one useful hidden control with a verified
effect. Finish the existing search delivery, then prioritize that experiment over
more catalogue plumbing, card counts or broad screenshot collection. Test during
actual relevant behaviour: menu-only samples cannot establish player ownership
or gameplay effects. Some rules are computed in code or distributed over several
objects and may have no single editable scalar. Such cases must remain explicit
limitations; expanding to code/data-flow analysis is later work.

Before calling an approach better, compare a fixed visual/memory baseline,
memory-change discovery without images, runtime/context discovery where supported,
and the combined approach under matched time and target-impact budgets. Use the
same initial conditions and repeatable event sequences. Score the time to the
first verified useful control, correct and incorrect labels, abstentions,
post-reload recovery, model usage and target pauses. Separate candidate generation
from effect validation so an approach does not grade its own predictions.

Automated test cases should include a hidden scalar, a display copy, a misleading
name, two similar actors, a derived value and a behaviour that is not represented
by one scalar. Keep ground truth out of the discovery input. Controlled cases
test mechanisms; unchanged evaluation on independently implemented games tests
transfer. Include both runtime-supported and opaque programs, and reserve unseen
games/names for evaluation. Use existing automation and evidence formats rather
than building another general testing framework first. No benchmark result or
additional progress credit is claimed by writing this plan.

## Primary-source basis and limits

The [Mono embedding documentation](https://www.mono-project.com/docs/advanced/embedding/)
describes native inspection of managed code, API/ABI boundaries and moving
objects. It establishes available runtime mechanisms, not working external
attachment in arbitrary shipped games.

[Unity's backend comparison](https://docs.unity.com/en-us/engine/6000.3/manual/programming-environment/scripting-backends/intro)
distinguishes Mono JIT from IL2CPP native AOT, existing-metadata reflection and
stripping constraints. [Unreal's reflection documentation](https://dev.epicgames.com/documentation/en-us/unreal-engine/reflection-system-in-unreal-engine)
describes registered objects and properties; it does not guarantee every C++
field or editor-only description exists in a packaged game.

[Squalr's own README](https://github.com/Squalr/Squalr) treats automated symbol
discovery from temporal snapshots and screenshots as a future AI direction.
Its scanning infrastructure is useful precedent, not evidence that automatic
semantic editing is solved. The choice of a hybrid design is our engineering
judgment from these mechanisms and measured failures, not a proven global optimum.
