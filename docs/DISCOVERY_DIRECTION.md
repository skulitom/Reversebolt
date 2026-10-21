# Discovery direction — 2026-10-21

Status: runtime feasibility demonstrated in a separate research probe; no runtime
adapter is integrated into the product and general causal controls do not yet
exist. The full goal and 31% weighted estimate
remain unchanged. The user's requirement is named, understandable, interactive
cards for humans and agents, including values and rules that never appear as HUD
numbers. Generalization excludes per-game profiles, offsets, signatures and
source-derived game layouts.

## Diagnosis

Reversebolt already combines native memory, images, temporal observations,
input context, frozen hypotheses and guarded trials. Nevertheless, its useful
live numeric path starts with readable screen numbers and seeks correlated byte
sequences. Runtime type/object inspection does not currently supply candidates.
This biases the product toward visible counters and copies of presentation data.
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

Next widen constructed-type, collection and root coverage, establish refreshable
reference paths and validate useful effects. Neither snapshot addresses nor
field names establish persistent identity or edit authority. Static values may
be defaults from uninitialized classes. Keep automatic reads separate from
mechanisms that execute code in the target, and expose unsupported paths.

Supporting a runtime does encode platform knowledge, much like supporting PE or
numeric encodings. It must not encode particular game classes, field names or
addresses. Unity Mono and IL2CPP require distinct treatment; stripped or
obfuscated names and changing runtime versions remain coverage limits. Later
Unreal reflection support is a separate adapter, not a generic promise to read
all shipped Unreal state. Evaluate usefulness against the raw-memory path using
unchanged adapters on independent software, including hidden values.

The next implementation milestone is evidence that runtime structure yields
useful nonvisual candidates and live values. The existing memory engine, change
journal, MCP authority and card interface should be reused. Improve adaptive
observation and dynamic mapping checks in support of this direction rather than
continuing to optimize visible-counter matching as the sole route.

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
