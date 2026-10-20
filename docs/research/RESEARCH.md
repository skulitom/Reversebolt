# Discovering editable game state without game profiles

Research and experiments, 20 October 2026.

**Recommendation: pursue general discovery as the core of the product.** A plausible system combines screen observations, memory time series, learned numeric relationships, informative actions, and validation of what a candidate actually controls. This research supports that direction; it does not establish automatic support for arbitrary games.

The original profile-first proposal would shift the project toward maintaining a trainer catalogue. The user's intended differentiator is learning useful controls from an unfamiliar running game. This lab therefore excludes game names, predefined offsets, signatures, engine layouts, and manually authored mappings from its discovery function.

## What Squalr already provides

Inspected and built commit [`568aa7589b68b2fd4621cc66c6dde23fa14c7f50`](https://github.com/Squalr/Squalr/tree/568aa7589b68b2fd4621cc66c6dde23fa14c7f50). All observations refer to that checkout; no GUI evaluation was performed.

| Component | Evidence in source | Verification here |
|---|---|---|
| Scanning engine | Separate Rust crate; scalar and SIMD implementations, parallel dispatch, multiple types, array scans | Built; 22 scanning-crate tests passed, covering scanning/pointer infrastructure |
| Native process access | Separate target abstractions and native providers | Opened our child, read and wrote its memory |
| External automation | HTTP server exposes `/health`, `/command`, `/events` | Drove process open, read, write, new scan, repeated element scans, and result listing |
| Snapshot representation | Current and previous byte buffers, constructors for supplied snapshots | Source inspected; a longer aligned observation history would be an additional layer |
| Pointer recovery | Pointer scan and validation code | Included in passing scanning-crate tests; not tested against a real game's restart |
| Debugging | Windows DbgEng backend plus other platform backends and trace interfaces | Source inspected and compiled; debugger behavior not exercised |
| Symbols and layouts | Named, typed layouts, pointer fields, binary-symbol plugin | Source inspected; this is not evidence of automatic health/ammo identification |
| Generic semantic discovery | README proposes automatic symbol discovery using time-varying memory and screenshots | No corresponding end-to-end observer/inference loop found in inspected core, CLI, and plugin code |

The [scanner entry point](https://github.com/Squalr/Squalr/blob/568aa7589b68b2fd4621cc66c6dde23fa14c7f50/squalr-engine-scanning/src/element_scans/element_scanner.rs), [snapshot type](https://github.com/Squalr/Squalr/blob/568aa7589b68b2fd4621cc66c6dde23fa14c7f50/squalr-engine-api/src/structures/snapshots/snapshot_region.rs), and [HTTP implementation](https://github.com/Squalr/Squalr/blob/568aa7589b68b2fd4621cc66c6dde23fa14c7f50/squalr-cli/src/http_ipc_server.rs) are useful integration points. Its [Windows debugger backend](https://github.com/Squalr/Squalr/blob/568aa7589b68b2fd4621cc66c6dde23fa14c7f50/plugins/squalr-plugin-debuggers-native/src/backend/windows_backend.rs) is more developed than the README's older “no debugger” sentence suggests.

Another 39 selected command tests passed: seven memory-command tests and 32 scan-command tests, including parser coverage. These are targeted checks, not a claim that the whole project is bug-free.

The smoke experiment supplied a marker address only to check transport correctness. Squalr's whole-process scan then located that marker independently. A separate four-observation exact scan followed a visible numeric track and converged to **one display-copy address**. A write to that address did not change the underlying state. That is expected behavior for an exact scanner and demonstrates the missing layer.

I would reuse Squalr's process/scanning infrastructure behind a small backend interface and keep inference separate. HTTP is convenient for initial experiments; Rust crates and byte buffers are preferable for sustained capture, because JSON encoding large snapshots is expensive. Pin the integration to a known revision. Squalr uses [GPL-3.0](https://github.com/Squalr/Squalr/blob/568aa7589b68b2fd4621cc66c6dde23fa14c7f50/LICENSE); the independent lab code here has not copied its implementation.

## Approaches worth pursuing

| Approach | What it contributes | Main unresolved issue | Status |
|---|---|---|---|
| Screen-to-memory time-series matching | Connects a changing number or bar to candidate storage locations | HUD detection, missing frames, scale, lag, display copies | Numeric matching implemented here; screen perception untested |
| Small symbolic models | Learns scale, offsets, inversions, and eventually nonlinear transformations | Large expression searches find accidental fits | Robust affine model implemented; broader symbolic search researched |
| Active observation | Chooses an action likely to separate competing explanations | Needs a model of actions and a way to reset or compare contexts | Proposed |
| Causal interventions | Checks whether editing a candidate changes the expected observation | Small probes cannot prove type, boundaries, or wider consequences | Tested on disposable fixtures |
| Instruction/data-flow tracing | Follows a display copy back to the values and calculations that generate it | Capture overhead, incomplete traces, complex optimized code | Source/tool capabilities researched |
| Semantic inference | Uses HUD labels, icons, events, and inferred relations to name controls | Ambiguous visuals and plausible but wrong language-model guesses | Proposed |

**Research closest to this idea:** COMET learns relationships between visual/object properties and internal game state, then models transitions using symbolic regression. Its paper demonstrates Pong and Freeway and discusses errors caused by correlated variables. The experiments use OCAtari object extraction, and the repository's recorder explicitly uses its RAM mode. Thus it is useful evidence for the matching/modeling approach, not proof of generic perception from an unfamiliar modern game. Integrating interventions and broader reasoning remains future work in that paper. [COMET paper](https://arxiv.org/html/2504.07257v1), [recorder source](https://github.com/k4ntz/COMET/blob/master/record_transitions.py).

OCAtari's per-game extractors include fixed RAM locations, as visible in its [Pong extractor](https://github.com/k4ntz/OC_Atari/blob/master/ocatari/ram/pong.py). We can borrow the experimental structure while supplying an independent perception layer; depending on those annotations would not satisfy the no-profile goal.

For deeper tracing, [Frida](https://frida.re/docs/javascript-api/) exposes hardware watchpoints and memory access monitoring; its page monitor reports first accesses, not a complete byte-level dependency graph. [Triton](https://github.com/JonathanSalwan/Triton) supplies dynamic taint analysis, symbolic execution, and instruction semantics. My proposed use is targeted tracing after statistical narrowing, especially when a useful quantity is derived or only its display copy is found. Neither integration was implemented in this pass.

## What the Rust experiment established

See [the full results table](EXPERIMENT_RESULTS.md). The experiment ran 50 previously unused random seeds per scenario, two anonymous numeric tracks per seed, and 12 scenarios: 1,200 target searches in total.

Each world has a 256 KiB byte arena, randomized field locations, 512 changing distractor locations, an exact display copy, a transformed telemetry copy, and a stale copy. The matcher receives 20 raw snapshots and numeric observations. It fits on the first 10 frames and validates against the last 10.

It tries `i32`, `f32`, and `f64` at their natural alignments, with a broad finite-number prior. It learns `observed = a * stored + b`; the scale and offset are never supplied to discovery. Pair-based robust fitting tolerates a minority of corrupted/missing observations. The current absolute tolerance is 0.55 observation units and would need an uncertainty model for general continuous quantities.

The true fields were among the learned candidates in all 600 searches across plain integers, normalized floats, scaled integers, inverted values, offset doubles, and noisy/missing observations. The exact-number baseline recovered only 200 of those 600 true fields. **These are candidate-recall results, not a 100% usable-control rate.**

Small positive and negative probes removed the display/telemetry copies. However, they also accepted 299 noncanonical interpretations across the full suite. These overlap the true fields' bytes: interpreting part of a double or a shifted integer can produce a convincing numeric relationship and a successful small edit. Only 500 of the 600 searches in the six supported scenarios ended with one candidate of the correct type and boundary.

The matcher did not recover the true fields in the squared-value, rotating-XOR, relocating-address, constant-signal, or unaligned scenarios. The unaligned case was particularly instructive: it could still find a shifted interpretation that changed the observation while missing the actual field boundary. In the coupled case it found both true fields, but the strict “change only one observed track” check rejected them. This is a limitation of that validation rule, not evidence that coupled game controls cannot be supported.

The experiment's “authority” means the fixture's canonical field from which its idle observation is derived. Frame advancement samples new synthetic values; these are not realistic game trajectories. The probes establish influence on observed state in this fixture, not effects on combat, movement, or later gameplay.

The live Windows smoke test used real `ReadProcessMemory` and `WriteProcessMemory` calls against a separately launched Rust child. Both anonymous tracks resolved to one correct location, positive/negative edits had the expected observed effects, and original values were restored. The child supplied its entire arena's address and ideal numeric observations. This validates the transport and matching loop but does not validate full-process region selection or OCR.

## Architecture to build next

```text
Captured frames + inputs                    Memory snapshots
          |                                       |
Generic HUD/object tracking                Changed-region filter
          |                                       |
Time-aligned observations --------> Candidate relationship models
                                             |
                             Evidence and ambiguity tracking
                                  /          |          \
                          Observe more   Trace writes   Test a hypothesis
                                  \          |          /
                                    Validated controls
```

The observation layer should discover numeric regions and bars without fixed screen coordinates. Initially it can expose “Resource 1” if meaning is unclear. A language model can propose semantic labels from visible evidence and event histories; it should not manufacture addresses or turn a correlation score into certainty.

The inference layer should retain multiple hypotheses, including lagged observations and alternate data types. At process scale, filter inactive memory first, retain compact candidate histories, and expand the search when observations cannot be explained. Storing every byte of several gigabytes across many frames is not a practical default.

Learned session models are compatible with the generalization goal: the algorithm learns them from current evidence. Precomputed per-game profiles are unnecessary. A cached result must be revalidated after object changes, loads, or restarts rather than trusted as a permanent address.

For validation, distinguish “observed alongside the HUD,” “affects this displayed value,” and “confirmed to affect gameplay.” Detect overlapping candidates and unresolved width/type choices. Combine temporal direction, instruction reads/writes, and carefully designed experiments. Restoring a memory word is not equivalent to restoring every consequence of a game action; the disposable fixture's cloning/reset convenience must not be assumed for normal games.

Damage is a later target: a useful damage control may be a parameter in a calculation rather than a persistent observable scalar. A better route is to first locate affected health, then study the transition and the instructions/data that produced it. This is a research direction, not functionality demonstrated here.

## Next experimental gate

Build one generic screen observation pipeline and test it, unchanged, on at least three independently implemented offline games. Declare the algorithms and thresholds before evaluating the held-out games. Withhold names, symbols, offsets, source-derived layouts, and annotated HUD regions from the discovery algorithm.

Measure candidate recall, type/boundary accuracy, false confirmed edits, time to first correct control, required user actions, CPU/memory cost, and recovery after loads/restarts. Evaluate at least one game with bar-only health, one with numeric HUD values, and one with substantial duplication or derived state. Include a run where perception cannot confidently label anything.

That gate would test the product's actual differentiator. The current lab proves a useful narrow mechanism and exposes several failure modes; it is not yet evidence that the mechanism transfers to Half-Life 2.
