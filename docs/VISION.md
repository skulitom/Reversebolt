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

## The human interface

Selecting an application starts investigation automatically. The main surface
is a small collection of direct controls, not a report: each has a meaningful
name, a short description of its effect, and an appropriate interaction such as
a draggable number, slider, toggle or action button. Findings appear as they
become supported. Investigation logs, screenshots and raw evidence are secondary
details. Do not make people operate screenshot or recording tools to get started.

Learning and uncertain observations must be visibly distinguished from controls
the engine can actually edit. Do not invent names or enable a cosmetic slider
when the causal meaning and editable state have not been established. These
intermediate cards can be inspected while the shared engine keeps learning.

Keep the main view focused on those cards. People can arrange them by dragging
or with accessible move actions, adjust supported values by dragging or typing,
and click the relevant action. Activity and application diagnostics open only
when requested. An observed name or a user-supplied label is not proof of an
effect; short descriptions must reflect what has actually been established.

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
