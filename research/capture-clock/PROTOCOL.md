# Capture clock correlation v1 — protocol frozen before execution

Question: can locally sampled Windows performance-counter values be converted
into the investigation's `Instant` clock with a conservative, checked interval?
This experiment does not measure Windows Graphics Capture, game rendering,
presentation latency, input causality or discovery accuracy.

Use the production `capture_clock::capture` wrapper and core `Receipt::project`
conversion in a dedicated clock-only example. Run exactly 100 samples once.
Alternate the central counter read between the calling thread and a new worker
thread, yielding 50 samples of each. Each iteration sleeps 1 ms, reads a central
counter sample bracketed by `Instant`, sleeps 1 ms, then closes the outer clock
bracket. The OS can lengthen either sleep; do not discard or retry those samples.

Convert the central QPC reading to a simulated 100 ns timestamp with integer
arithmetic. Correlate it using the two outer production samples. Retain every
anchor, central sample, operation bound, interval, uncertainty and error.
Count a measurement consistent only if its projected interval overlaps the
central sample's measured local interval, including the local clock's 1 us
truncation allowance. Summarize successes, failures and interval widths for all
100 samples and each thread group. Acceptance is 100/100 consistent; any other
result requires investigation and remains in the evidence.

Freeze this protocol, runner, all Rust/Cargo sources and the dedicated example
executable before execution. Write a new output directory, never replace an
existing run. Hash-check all frozen files afterward. Preserve process output
and every result, including failure. Verify counts and group allocation.

The executable only calls local clock APIs, starts its own short-lived worker
threads and writes its new result file. It captures no image, uses no compositor
API, opens no game, reads or writes no target process, sends no input, calls no
model and changes no security setting. Its synthetic `Screen` carries no image
or usable pixel hash. It does not launch or replace the quarantined application.

Separate automated tests cover mathematical rounding, incompatible clocks,
missing/stale/future timestamps, selected-image binding, legacy serialization
and the shared saved-image reader. Even a successful probe is clock arithmetic
evidence only. Capture-operation bounds remain authoritative for input
classification and comparisons. A real image/compositor timing study is still
required before considering any change to those rules.
