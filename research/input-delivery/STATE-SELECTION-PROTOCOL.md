# Separate current-state selection from adjustable-parameter selection

The previous sibling-input experiment reused the normal type-selection prompt,
which explicitly prefers adjustable parameters. Constancy during play is often
expected for such parameters; using that selection as a changing-state detector
mixes two different goals. Add an explicit current_state purpose to the shared
observer selection API and offline CLI. Preserve the default parameter prompt,
all type evidence, paging, 24-request/12,000-byte bounds, and no-edit semantics.
Legacy receipts default to adjustable_parameters; record the purpose explicitly.

Before model inference, freeze the changed source, evaluator binary, exact saved
Sable report and installed local model identity. Run one current-state selection
on the same report as runtime-embedded-activity-v3, with the game closed. Do not
revise the prompt after viewing the selection. Feed its completed selection into
the unchanged enclosing-role sampler with its 128-path limit. Retain all selected
types, omissions, model stages and sampled paths. Compare overlap with the prior
parameter selection descriptively. Different labels or a plausible type list do
not establish better recall, actual activity, player identity or useful controls.

This mode is available for an agent investigation; it is not a replacement for
static-parameter discovery and is not yet the desktop default. A later native
measurement needs a new frozen plan and independent rendered-response evidence.
