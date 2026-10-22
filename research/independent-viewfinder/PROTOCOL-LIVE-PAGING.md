# Live development evaluation after catalog paging

Freeze published preview `0.1.0-20261022-235934` before launch. Viewfinder is now
a development target: the previous native failure and saved-data replay informed
the generic catalog-paging correction. This run cannot be called a fresh held-out
evaluation or replace the original failure. No product code, prompts, model,
selection rules or budgets may change during this run.

Use `freeze.py --output test-results/viewfinder-live-paging-v1 --bin
dist/0.1.0-20261022-235934 --protocol
research/independent-viewfinder/PROTOCOL-LIVE-PAGING.md` to retain current source,
binary hashes, evaluator, Steam manifest and local storage. Use fresh editor
storage, Anode session 3, and the same installed Viewfinder (Steam 1382070).
Preserve the main-desktop editor and its connected session.

The original protocol's scope remains: at most four manually reviewed setup
images, Continue if available, zero manually reviewed measurement images, no
source/decompilation/game-specific layout or field supplied to the product.
Reuse `research/independent-runtime/run.py` unchanged, with qwen3.8:latest, one
automatic attachment, four 200 ms D/A/D/A probes at captures 2/4/8/10, and at most
one comparison chosen by the product. Do not manually select a candidate or
replace failures. The product supplies the baseline/change/recovery actions and
guarded edits. Preserve all failures, abstentions and restoration results.

Do not restart uncertain live jobs. Stop on focus/input uncertainty or unresolved
restoration; preserve the engine if recovery needs it. Close only owned processes
after settled recovery, retain save differences and the backup, verify protection
and desktop availability, then release Anode. No upload. A named hypothesis alone
does not verify a control; successful byte restoration does not establish world
rollback. Report native effects separately from semantic claims.
