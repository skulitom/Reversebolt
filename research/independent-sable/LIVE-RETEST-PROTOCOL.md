# Sable development retest after request-context changes

Freeze a freshly staged build of source 956553a, its four binary hashes, current
Rust sources, the evaluator, this protocol and a new save-folder backup before
launch. The staging-script change only avoids activating the desktop shortcut.
Sable is now a development target: the previous native failure and saved-data
diagnostic informed this correction. Preserve those failures and do not call
this an unchanged held-out acceptance test.

Launch the staged executable normally in Anode with fresh editor storage. Do not
change Windows Security or retry the previously cancelled elevation. If Windows
blocks the executable, retain that result and do not use a renamed/modified copy
or an allowed release path to bypass it. Leave the user's desktop editor and
launcher in place throughout testing.

Use installed Sable build 10132013 and the existing Continue action. At most four
manually viewed setup images, zero manually viewed measurement images. Keep the
viewer hidden and use the unchanged `research/independent-runtime/run.py`:
fifteen automatic captures, four bounded 200 ms D/A/D/A probes, the same local
qwen3.8:latest model, and at most one product-planned baseline/change/recovery
comparison. Do not manually select a field, value, label or action meaning. No
game source, decompilation, symbols, offsets or game-specific layout. No prompt,
quota, budget or binary change during the run. No experimental comparison ranking.

Start read-only GPU/model telemetry before game launch and stop it after owned
process cleanup. Sample global graphics memory/utilization once per second and
the local model's loaded context/digest approximately every two seconds. Record
errors rather than changing device settings. These measurements are diagnostic
associations, not a causal proof of GPU contention. They add no screenshots,
game input, model generation or target instrumentation.

Never restart an uncertain live job. Stop on unresolved restoration or focus/
input uncertainty. After authoritative runner completion and settled recovery,
close owned test processes, retain any save differences and the original backup,
verify the main desktop and Windows protection, and release Anode. Keep the
staged build separate until its native outcome has been reviewed. Everything
produced by the evaluator remains local; no vendor submission is made.
