# Progress against the full vision

This is a weighted capability estimate, not an estimate of elapsed development
time or a measured probability of success. The denominator stays fixed. Credit
requires a concrete implementation and evidence. Partial credit is allowed.

| Capability | Weight | Evidence needed for full credit |
|---|---:|---|
| Foundation and delivery | 10 | Open repository, shared persistent engine, accessible desktop, versioned runnable delivery |
| Native target access | 10 | Target selection, reads and captures, typed writes and history, efficient snapshots |
| Multimodal observation | 15 | Screen values, motion, animation, model geometry, synchronized input and audio |
| General discovery | 20 | Numeric matching, experimental planning, objects and relationships, hidden and derived rules |
| Validation | 15 | Authority vs copies, correct representation and scope, actual gameplay consequences |
| Intent and editing | 10 | High-level requests, composed changes, reusable knowledge, recovery and revalidation |
| Agent and human collaboration | 10 | MCP, durable jobs, observable shared sessions, takeover and interruption |
| Generalization and release | 10 | Withheld independent games, regression/performance evidence, usable open-source release |
| **Total** | **100** | **Full VISION.md acceptance criteria** |

## 2026-10-20 — starting point: 2%

Inherited independent MIT research demonstrates affine numeric matching on
randomized byte layouts and live owned-child reads/writes. The Squalr study and
benchmark failures are preserved in `research/`. No credit for a general screen
observer, real-game inference, or autonomous semantic understanding.

Subsequent entries must identify what changed and what remains unproven.

## 2026-10-20 — first desktop preview: 10%

| Capability | Credit / weight | Evidence and limits |
|---|---:|---|
| Foundation and delivery | 4 / 10 | Rust workspace, native desktop, persistent shared engine, versioned preview and desktop launcher; early UX, local distribution |
| Native target access | 1 / 10 | Real Windows reads/writes to an owned process; advertised arena only, no arbitrary target inventory or scan |
| Multimodal observation | 0 / 15 | Numeric fixture oracle only |
| General discovery | 2 / 20 | Affine matching and held-out evidence on randomized layouts; no semantic, object, motion or rule inference |
| Validation | 1 / 15 | Rejects display copies with two practice interventions, verifies both tracks; hidden side effects and representation aliases unresolved |
| Intent and editing | 0 / 10 | Low-level numeric controls only; no high-level intent or recovery across loads |
| Agent and human collaboration | 2 / 10 | Nine working MCP tools, same desktop session, visible activity and history; no durable cancellable jobs or takeover protocol |
| Generalization and release | 0 / 10 | No independent real-game acceptance gate passed |
| **Total** | **10 / 100** | **The full goal remains active** |

Release binaries passed the real-process/MCP smoke suite, and the desktop passed
Anode UI checks at 1280×720. See [validation](VALIDATION.md). The two inherited
discovery regression tests and strict Clippy checks pass.

Next: target inventory and bounded capture, a real screen/behavior observation
path, then an unchanged discovery pipeline evaluated against an independently
implemented offline target. Keep the supplied-label practice as a regression
fixture; do not let it become the product's general inference architecture.

## 2026-10-20 — application connection and observation: 13%

| Capability | Credit / weight | New evidence |
|---|---:|---|
| Foundation and delivery | 4 / 10 | Dark Bolt-family desktop; existing delivery remains |
| Native target access | 3 / 10 | Searchable window picker, exact process identity, read-only attachment, bounded snapshots, disconnect and exit invalidation |
| Multimodal observation | 1 / 15 | Window image + native OCR + numeric screen tracks; 12/12 varied values in an owned WinForms fixture |
| General discovery | 2 / 20 | Existing matcher accepts screen tracks; no new real-game discovery credit |
| Validation | 1 / 15 | Existing practice evidence only |
| Intent and editing | 0 / 10 | No external application writes or high-level intent |
| Agent and human collaboration | 2 / 10 | Five new MCP tools share the application connection and history; 14 total |
| Generalization and release | 0 / 10 | No independent real-game acceptance gate passed |
| **Total** | **13 / 100** | **The full goal remains active** |

The native fixture test found no correspondence in its 4 MiB sampled memory;
this is recorded as zero hypotheses, not successful discovery. Unit evidence
covers excluding incomplete pages and revoking hypotheses on process exit.
Anode checks cover search, selection, connection, capture, empty search,
cancellation, and the corrected sidebar layout. See `VALIDATION.md`.

Parallel research owns isolated `experiments/` directories for visual behavior,
causal validation, and hidden-rule inference. Their results receive no
integration or generalization credit until reproduced and integrated.
