# Retained first watch evaluation — 2026-10-21

This frozen first revision **failed before its watch could start**. The original
plan serialized to 261 bytes with real session/process identifiers, exceeding
the existing 256-byte journal step limit. Unit fixtures had used short IDs and
missed this integration issue. The accepted discovery jobs had completed; the
trial byte was restored by the declared checked cleanup, with no game input sent.

The fixture's immediate observation correctly retained the old displayed value
after the memory edit. No delayed-response result is established by this case.
All four owned processes exited and Anode was released. The post-run
[audit](audit.json) verifies 24 source image hashes, frozen method/binary hashes,
and the restoration receipt. `source/` was copied after the run and before the
repair; its files match the original manifest's source hashes.

The raw runner's `outcome` and `final_oracle` fields were last assigned during
discovery. They are not a successful watch result: `errors`, final status and
the audit retain the failure. Revision 2 separates these metadata fields.

The repair keeps the existing size bound and uses shorter journal-only field
names. Public command parameters are unchanged. A regression with realistic
UUID/process identifiers reproduces the missing job before the repair. The
[second evaluation](../change-watch-v2/README.md) uses a new binary, frozen method
and output directory; it does not replace this failed case. Raw evidence remains
in `test-results/change-watch-v1/case-0/`.
