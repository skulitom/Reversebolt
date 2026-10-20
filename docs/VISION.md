# Reversebolt

Reversebolt is the Bolt family's open-source Rust workbench for understanding
and changing live interactive software. A novice opens an unfamiliar game and
gets useful, understandable controls. An agent can investigate the same session
through MCP, with the person able to watch, intervene, and take over.

The destination is automatic discovery without a library of game names, memory
offsets, signatures, source layouts, or hand-authored trainers. Learned reusable
knowledge is welcome; predefined answers are not a substitute for inference.
Generalization and honest uncertainty take precedence over a polished illusion.

Observation must include numbers and bars, but also movement, animation, model
appearance and geometry, inputs, sound, relationships, and the consequences of
actions. Hidden and derived properties such as damage, acceleration, cooldown,
collision, or an animation rule matter even when no number appears on screen.

The application forms hypotheses, explains the evidence, plans useful experiments,
distinguishes display copies from authoritative state, and verifies an edit by its
actual effect. Ambiguity remains visible. An unfamiliar value is not called
"health" merely because it is 100. A successful memory write is not evidence of a
successful gameplay change.

People express intent in ordinary language: move faster, change appearance, make
this jump possible, change a rule. The tool resolves the affected object, scope,
representation, and consequences, offers useful controls, records changes, and
can restore them where possible. It detects stale discoveries after loads,
restarts, reallocations, and game updates and revalidates instead of blindly
reusing addresses.

MCP is a first-class interface to the same engine, observations, experiments,
durable jobs, change history, and recovery mechanisms as the desktop. Neither
interface owns a private parallel interpretation of the session.

## Completion evidence

The full goal is complete only when these capabilities work together in the
desktop and agent workflows, are documented and reproducible, and the unchanged
discovery pipeline has passed a withheld evaluation on multiple independently
implemented offline games. The evaluation must include hidden behavior and
appearance, not only HUD numbers. Record failures and false positive rates.
Practice fixtures, research benchmarks, UI screenshots, and an MCP handshake are
useful intermediate evidence, not completion.

Arbitrary game compatibility cannot be promised. The finished system should know
when it lacks evidence, present useful partial results, and keep investigating
within its supported access and observation capabilities.
