# Runtime probe v1 — frozen before native runs

Question: can a generic IL2CPP API adapter expose field descriptions AND existing
live scalar storage without starting from screen numbers or game-specific layouts?
This study does not claim meanings, causal authority or working gameplay controls.

Build the standalone Rust workspace. Keep production binaries unchanged. Record
source and binary hashes before native runs. Use Anode session 2, preserve desktop
Steam, acquire/release a lease, and clean up only processes launched for this study.
No screen captures, player inputs, save selection or field writes are prescribed.

First run the owned ABI fixture. Inspect twice with the same helper, separated by
at least 200 ms. Require four scalar reads with expected charge 37, hiddenBias
1.75, enabled true and increasing ticks. Require metadata-only reasons for the
thread-local sentinel, literal, instance and reference fields. Require helper
unload after each call. Reject incorrect creation identity, helper hash, session,
and an already-existing output path before instrumentation. Keep each receipt.
This fixture checks the ABI harness only; it is not a real IL2CPP implementation.

If that passes, launch installed Blue Prince (Steam 1569580) through Anode. Record
its PID, creation identity, session and runtime file hash. Wait for its runtime
module and main window; do not select a save. Run two unchanged probes on this
process, separated by at least 200 ms. Retain every report and loader receipt.
Stop if a helper thread times out: do not reload/unload or free its arguments
while execution is uncertain. Record the failure and verify thread termination.
Do not retry a failed game probe under the same case identifier.

Success at this stage requires a completed real-runtime enumeration, at least one
non-system assembly with field descriptions and existing scalar reads, no game
name/field filtering inside the adapter, and verified helper unload. Report counts
by image, omissions, unsupported kinds, and matching/differing two-pass values.
Names and samples are hypotheses. Static storage can contain defaults or caches;
instance objects, lifetime handling, authority, scene/restart revalidation and UI
integration remain future work. Do not increase the overall 31% estimate for this
feasibility study alone.

The helper executes native code in the target and attaches a runtime thread.
Metadata calls may initialize metadata or perturb timing/allocations. It requests
no managed class constructors, getters, setters, GC controls or arbitrary methods.
Reads are timestamped and non-atomic. Primitive values use public type codes and
runtime-reported offsets/storage bounds. Open generics, literals, unavailable
storage, unsupported types and thread-static offsets abstain. Budgets are 256
assemblies, 50,000 classes, 200,000 fields and 25 seconds between API calls; a
single stuck API call cannot be cancelled safely. Loader waits 40 seconds per
thread, retaining code/arguments on uncertainty. An ABI mismatch could crash the
owned test process; this is research instrumentation, not an automatic production
attachment path.
