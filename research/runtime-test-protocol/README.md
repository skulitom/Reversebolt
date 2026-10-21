# Input-response test protocol foundation

The shared Rust core now models a bounded baseline / changed / recovered
comparison. This is a **library foundation**, not a live desktop or MCP feature.
The existing live edits, Freeze and Undo continue to use their established paths.
No game was opened, controlled or newly captured for this work.

One plan binds a process, investigation, catalog, exact typed runtime candidate,
distinct test/edit identities, proposed value, caller hypothesis and four input
actions. Each condition retains an idle image pair and four after-action images.
A weak or incompletely sampled baseline stops the plan before it becomes ready
to apply. Each observation needs an exact revision and a saved pending intent;
an unsettled observation cannot be repeated. Restart stops the comparison.
The library itself performs no capture, input, file persistence or memory write.
The future engine adapter must durably save these transitions and own the evidence.

Input summaries are recomputed from bounded sampled Windows key windows with
matching clock/generation, full foreground coverage, ordered polls and checked
coverage gaps. One bracketed key/chord hold must match the planned duration to
within 50 ms and end at least 250 ms before the next capture begins. Repeated
taps, unexpected keys, changed generation, late input and missing coverage fail.
These remain sampled key states, not proof of delivery to the game.

The comparison synchronizes with validated durable runtime trials. An early
edit/Undo interrupts it. Completing requires a restored trial, all three image
conditions, compatible sampled holds, and fresh post-condition reads with exact
scope, path, type, bytes and reference guards. An overwritten replacement,
changed reference, missing read or inconsistent chronology is inconclusive.
Reduced rendered response never grants edit authority or a verified semantic name.
Byte-level restoration cannot establish that all gameplay consequences recovered.

The versioned image metric normalizes grayscale images to 640×360, crops to
(24,40)–(616,336), applies Rust's Gaussian blur at sigma 1, and measures mean
absolute difference. The crop and thresholds inherit the earlier exploratory
Manifold work; they are not a held-out universal metric. Rust's blur differs from
Pillow's. The retained-image audit checks the resulting decisions and hashes,
not numerical equivalence.

Seven synthetic protocol tests cover valid suppression/non-suppression, weak
baselines, missing/wrong input, duration/focus/coverage, stale cursors, changed
geometry/generation/digests, moved references, overwritten values, chronology,
early Undo and restart. A separate analytic image test checks exact differences
on constant images. The 36 retained Manifold images yielded the same two visual
decisions over 30 pairs with the Rust metric. They have no engine-owned input
windows, so they **cannot verify the complete new protocol**. Full results and
source hashes are in [results.json](results.json).

Next integration must provide exclusive session ownership, saved capture intents,
bounded evidence storage/read-back, expiration, job/human takeover, interruption
on target/catalog/write changes, and desktop/MCP actions. It must not auto-resume
experiments or publish caller hypotheses as confirmed controls. No progress
percentage is added for this foundation alone.

The direct number editor also now steps from an uncommitted visible draft when
the +/- click and focus loss occur together. Its automated egui test checks the
exact unsigned 64-bit boundary without a floating-point conversion.

The full workspace passed 253 tests, strict Clippy, formatting and release build;
the frozen 45-tool smoke passed. Desktop preview `0.1.0-20261021-202940` preserves
all 44 history entries, the observer setting and all trial/Freeze records. The
four published binaries match the tested copies. Local release receipts are in
`test-results/runtime-test-protocol-v1/release-verification.json`.

Reproduce the retained measurement with the `response_audit` vision example and
`test-results/runtime-test-protocol-v1/pairs.json`, then run `audit.py` here.
