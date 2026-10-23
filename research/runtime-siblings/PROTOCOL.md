# Rooted sibling observations v1

This development experiment asks whether nearby scalar fields on the same rooted
runtime object path can reveal repeated input-associated activity. It does not
infer actor identity or authorize an edit. Sable and Viewfinder are already known
development targets; this is not independent-game acceptance.

Freeze the implementation, evaluator and exact reader/analysis binaries before
measurement. First replay selection on the two retained native discovery and
naming receipts from `sable-live-context-v1` and `viewfinder-live-paging-v1`.
Use every accepted primary and alternative path, in receipt order, as seeds.
Select up to 128 supported boolean/numeric fields across their exact rooted
roles, round-robin in report traversal order. No label/value ranking, manual
field selection, path substitution or new model call. Retain per-role coverage,
all omissions, and batches of at most 32 paths / 60 KiB.

The response analyzer requires fourteen ordered samples:
quiet, quiet, action A, quiet, quiet, action B, quiet, quiet, action A,
quiet, quiet, action B, quiet, quiet. Each quiet pair must agree before and after
an action. A state association returns to its prior value after each action;
a step association changes during each action and retains that value afterward.
Require all four repetitions of the same association type, complete valid scalar
reads and unchanged nonzero field/object-chain addresses. These conditions do
not exclude address reuse, coincidental activity, animation or input buffering.
Exact byte equality deliberately misses noisy or delayed responses.

Each final native sampling pause must carry QPC start/end bounds. Both QPC calls
are outside the managed pause. The same-session evaluator records foreground and
all sampled virtual-key states with QPC bounds. Require matching frequency,
chronological polls, at most 50 ms between polls across the entire sampling
interval, and an enclosing poll on each side. Reject intervals with wrong focus,
unexpected held keys or missing coverage. This observes Windows input state, not
delivery to game logic. Record earlier/later gaps without attributing them to the
sample. Evidence and plan hashes bind each analysis result.

Before game measurement, run the native owned path fixture with the exact new
helper, retaining unload completion, all final values, clock bounds and zero
Rust heap operations while paused. The fixture is only ABI/clock validation.
Any live-game continuation needs its own frozen execution supplement, including
bounded input duration and maximum batches; do not improvise longer input to
make a failed interval pass. Do not replay uncertain native operations.

Keep the connected desktop editor and its tested preview in place. All artifacts
remain local. Do not change Windows Security or retry cancelled elevation. No
gameplay-field writes, screenshot inspection or model generation is required
for the saved-data and native-fixture stages.
