# Sable development retest: completed naming, no verified gameplay control

Preview `0.1.0-20261023-012028`, built from product source `956553a`, completed
the unchanged automated evaluator in **204.368 seconds**. Its outer discovery
job completed, naming returned five cited hypotheses, and one product-planned
edit/comparison/restoration completed. There was no naming timeout in this run.
This is a development retest, not independent acceptance: the previous Sable
failure informed the request-context change. The original failure is retained.

The [protocol](LIVE-RETEST-PROTOCOL.md), current sources, evaluator, four binaries
and save backup were frozen before launch. The test used the normal executable
path in Anode and made no Windows Security changes. No canceled elevation was
retried. The current desktop editor stayed open on its existing release.

## Discovery and comparison

The catalog contained 339 types and 1,809 fields. Twenty-two metadata requests
completed in a summed **69.347 seconds**, followed by a **17.109-second** activity
request. The named hypotheses were `footstep raycast distance`, `face the player
during conversation`, `animSpeed`, `startRadius`, and `Grip Multiplier`. These
names are model proposals grounded in runtime metadata; they are not proof of
usefulness, correct semantics or sufficiently friendly descriptions.

The existing planner selected `footstep raycast distance`, changing **0.1 → 0 →
0.1**. The same recorded D/A/D/A input sequence was applied in baseline, changed
and recovery conditions. Its rendered-response medians were **9.133 / 8.941 /
8.701**; a separate full-resolution pixel measurement gave **12.102 / 12.001 /
11.618**. Neither measurement shows suppression. Guarded write/readback and
restoration records confirm the original bytes were restored.

This movement-oriented measurement does not test the proposed footstep-event
meaning. No audio was available in Anode, and no footstep event was measured.
The negative result must not become a claim that the field has no effect.
Choosing a test and sensor that match a hypothesis remains unresolved. The
activity model also returned a false consistency flag alongside prose describing
consistent orientation change; that conflict remains in the evidence.

## Reliability and cleanup

All metadata requests in this new run fit the smaller 16,384-token context.
The newly selected groups differ from the earlier saved-catalog replay, whose
large group required 32,768 tokens. This success therefore does **not** prove
that the prior timeout was fixed in every condition or that large live requests
are now reliable.

Read-only telemetry captured 290 complete global GPU samples and local model
state approximately every two seconds. Reported free graphics memory reached
688 MiB; no causal diagnosis follows from that observation. The telemetry child
was stopped after cleanup, leaving one incomplete final CSV row and potentially
unflushed trailing samples. The audit ignores only that incomplete row. No GPU
or model settings were changed by the telemetry process.

The audit checks 33 automatic image hashes, 16 bounded input holds, citation and
type-page coverage, seven helper completion/unload records, guarded readbacks and
the original-value restoration. Four setup images and zero measurement images
were manually viewed. The 150 recorded pauses had a maximum stop-to-resume time
of 5,672 microseconds, with no helper allocations/deallocations while paused.

All owned test and telemetry processes exited, Anode's lease was released, and
the user's desktop editor remained responsive. Antivirus, real-time protection
and tamper protection remained enabled; no new Reversebolt detection appeared.
The main save, its image, settings and Steam cloud descriptor are byte-identical
to their backups. The save index and two logs changed. Both before/after folders
remain local; this is not a full world rollback claim.

The tested directory was subsequently activated for the next desktop launch,
after verifying all four manifest hashes. Activation changed the launcher only;
it did not replace the user's connected window or copy into an allowed path.
`-StageOnly` and a separate manifest-checking activation step now make this
prepare/test/publish sequence reusable.

Run `python research/independent-sable/audit-live.py` for the offline
[artifact checks](audit-live.py). [Structured results](live-results.json) and
`test-results/sable-live-context-v1` retain the outcome and limitations. The
unchanged Rust code had passed 459 workspace tests, strict all-target Clippy and
formatting before this release build. Progress remains **36/100**, and the full
goal stays active.
