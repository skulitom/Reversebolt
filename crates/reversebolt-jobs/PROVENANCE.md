# Origin and boundary

The initial library and its 29 tests were brought from the independently tested
MIT experiment in `experiments/session-jobs` on 2026-10-20. The source license is
retained here. The fake executor supplies test evidence only; the desktop does
not execute its payload language.

This crate owns durable job transitions, request receipts and events. The
application adapter owns authenticated transport provenance, typed observation
plans, target identity checks and the sole live-engine operation slot. Generic
effect/consent types in the library do not authorize live application writes.

The append-only journal is bounded, checksummed and synchronized. It is not an
exactly-once effect guarantee or a transaction with engine evidence files.
Restart invalidates active work; it never automatically resumes observations or
effects. Preserve the ledger: migration and rotation are unfinished.
