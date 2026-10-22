# Comparison history across investigations

Run the new file-only `history-replay` example against a disposable copy of the
completed comparison 080b5643-5718-494f-9ed6-751bf8059402 from
`test-results/planned-input-v1/case/data/runtime-tests`. This is an earlier,
hand-selected Manifold Garden comparison, not an automatic discovery success.

Freeze source, executable and all input-file hashes before running. Archive the
executable. The replay must create a new output directory and never open the
original directory as a writable store. Preserve any failed attempt.

Load and validate the original closed record, copy its evidence, then add seven
explicitly synthetic stopped records to its investigation. A ninth record in
that investigation must be refused. Add one synthetic record to a new
investigation, reopen the store and verify all nine records remain available.
Synthetic records must carry no observations, assessment, origin or preparation.

The original record must remain byte-for-byte unchanged. Use the production
readers to verify all eighteen saved image/input pairs and recorded input intents
and outcomes after restart. Do not recompute or claim a new gameplay effect.
No game, editor, helper, model, input worker or process-memory access may run.

Automated tests separately cover dispatcher replay and publication across
restart, 128-record saturation, continued settlement of existing records, shared
capture quota accounting, unsupported evidence layout and the existing failure
cases. Complete the workspace tests, strict Clippy, format and diff checks.

The change is source-only. No generalization credit or desktop-security clearance
is claimed; the quarantined preview is not restored or replaced.
