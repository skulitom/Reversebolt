# Shared property-question budget

Development experiment, frozen before revised inference. Reuse the five receipts
from `test-results/object-label-selection-v1/revised` as the baseline; do not
repeat baseline calls. Inputs are the same latest/earlier gameplay, menu, owned
context fixture and opaque-label variant. They are already explored reports,
not held-out game evaluations.

The existing policy gives each selected type one or two questions according to
selection order. Replace the per-type calls with one shared table and a budget
of twelve distinct property questions. Common type names are supplied once and
referenced by type ID, keeping complete field labels and exact object-local text.
No row is deleted to make the prompt fit: refuse an oversized table. Keep type
selection, inventory sampling, exact citation validation and write authority
unchanged. Use a 49,152-token context and 10,200-token output budget for the shared
question pass, with the existing input-headroom check. There are at most two
metadata model calls instead of nine.

Clarify that a specific same-object label can support a question about a generic
field, and that duplicate-looking paths are not a reason to drop a whole property.
Explicitly exclude peripheral assignment/calibration and input bindings from
gameplay-property questions. These are general domain distinctions, not a game
allowlist, preferred field dictionary or source-derived layout.

Freeze the proposed observer source and offline executable before one revised
run per case. Retain raw responses, timing, model identity, source/report hashes,
abstentions and rejections. Do not change prompts after seeing results. Compare
type selections as well as accepted questions: differing selections would limit
attribution to the second stage. No game, process inspection, input, memory edit
or image review is needed.

Promotion requires all reference/budget checks to pass; no accepted peripheral
configuration questions in the menu case; no invented meanings in the opaque
case; at least one exact object-text-grounded question in the context fixture;
and retention of plausible movement questions in both gameplay cases. These are
development regression criteria, not proof of semantics or causality. Preserve
all failures; if criteria fail, retain the experiment and restore the previous
production policy. Do not tune and rerun to obtain a preferred result.

No new named card, tested effect, range, write permission or generalization credit
follows from a model question. The native Windows detection remains separate.
