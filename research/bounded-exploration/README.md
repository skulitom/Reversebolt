# Bounded response exploration

The shared Rust planner now supports an optional `comparison_limit` from 1 to 8.
Agents can continue after a complete, restored negative comparison without
discarding it or manually preferring a field. Positive, inconclusive, stopped,
missing or inconsistent results stop the search. The desktop/MCP share the same
engine records and limits; planning itself remains read-only.

The actual tested question is returned separately: does this edit suppress the
rendered response under the recorded inputs, with response returning after
restoration? That is narrower than proving a provisional gameplay label. A
negative screen result cannot disprove an interaction, sound or other unmeasured
effect. This change does not solve semantic ranking or choose new measurements.

## Behavior and checks

The policy counts every saved test matching investigation, catalogue, report,
source frame/input digest and action sequence. Removing a candidate from a later
review cannot erase a test from that budget. Each completed test's native trial
linkage/restoration and completion assessment are checked again on a private
copy. Regional outcomes use the existing retained-evidence review. Missing
candidate history or an unresolved outcome yields `needs_review`.

The existing candidate order, probe values, input sequence, write/restoration
guards, history and manual operations are preserved. Existing callers that omit
the limit retain their previous suggestion behavior. A `continue` decision does
not override busy state, capacity, or absence of another candidate. Full details
are in [the MCP contract](../../docs/MCP.md).

Thirty offline checks use genuine retained Electrician negative and Hyperbolica
positive comparisons. They exercise continuation, limits, missing/corrupt trial
data, stopped/pending/changed results, missing candidates and unrelated contexts.
Both original native artifact audits still pass. The replay executable, source
snapshots, input digests and all results are retained locally under
`test-results/bounded-exploration-offline-v1`. This is correctness evidence for
the decision rule, not independent gameplay validation.

All **511 ordinary workspace tests** pass, with one opt-in archive test ignored.
Strict all-target/all-feature Clippy, formatting and the release build pass.
Preview **0.1.0-20261023-104538** passes packaged headless dispatch, all 65 MCP
tools and the new optional-argument schema check. The desktop shortcut selects
it; the existing editor remains open. The core policy is in
`crates/reversebolt-core/src/runtime_exploration.rs`.

## Native development experiment

The [frozen protocol](PROTOCOL.md) preserves the original Electrician negative
trial and uses a fresh game process/engine/data directory. Product discovery,
model identity and candidate ordering are unchanged. The generic bounded runner
uses the shared API and asks for at most three comparisons, re-reading the plan
after each result. It never substitutes a preferred field or retries uncertain
input/writes. It is an agent workflow prototype, not a desktop feature that
silently edits on attachment.

Electrician is now a development target. This follow-up cannot be counted as
another withheld success, and a positive image response would still require
investigation of meaning, actor identity, scope and downstream effects.

The fresh run completed discovery but **did not reach a comparison**. Naming
accepted two provisional questions (in hand max length and player money), versus
seven in the original run. The current scene, observations and inference output
differ; no deterministic naming or relevance claim follows. There were 25
metadata calls (249.327 s summed), one activity call and six visual-object calls.
The visual-object pass completed, while the activity interpretation found no
consistent visible change. The evaluator finished in about 341 seconds.

All four initial key holds were recorded. However, the saved activity packet
classifies two intervals as unknown because capture timing was invalid,
overlapping or widely separated. The third probe (the second D hold) therefore
lacks a valid preceding quiet interval. Only three action episodes are bounded;
the existing planner requires four in this whole-image mode and correctly
refuses a replay. The measured contrast alone does not repair missing timing.
No preferred field, alternate probe or retry was substituted.

This means native continuation after a negative remains **unproven in this run**.
The 30 saved-native-history checks validate the decision rule, not execution of
a live multi-comparison investigation. Progress remains **40/100**, with unchanged
capability credits. Next: acquire missing qualified observations automatically
without relaxing timing rules, then evaluate bounded exploration again. Ranking,
reliable naming, semantic validation and independent generalization remain open.

The [audit](audit.py) verifies 15 image hashes, four input holds, frozen sources,
model/reference/schema evidence, the planner refusal and clean final state. One
native discovery helper completed/unloaded in 4.773 s; 127 recorded pauses had
zero helper allocations/deallocations, maximum 5.457 ms. No runtime test, write,
trial or Freeze was created. Setup used six images and seven menu/focus actions;
no measurement image was manually viewed. Saves and Steam app storage are
unchanged; only two game logs changed. Owned processes exited, Anode was released,
and the user's main editor remained responsive.

[Results and exact refusal](results.json) are committed. Raw evidence and storage
copies remain local in `test-results/bounded-exploration-v1`. Run
`python research/bounded-exploration/audit.py --verify-current` to recheck them.
