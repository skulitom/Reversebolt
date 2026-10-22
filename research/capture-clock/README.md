# Selected-image clock diagnostics

2026-10-22. Full-goal progress remains **35%**. This adds timing instrumentation,
checked conversion and local evidence, without changing activity discovery or
claiming game validation. Security-review artifacts stay local. The published
desktop executable remains unavailable; no application was relaunched.

## Why investigate this

The [input cadence study](../input-cadence/README.md) found that short actions
overlapped image acquisition. Treating the entire operation as an unknown image
time prevents sound isolated-action inference on those recordings. Shortening
fixed delays alone did not solve that problem. A timestamp for the actual
selected image might permit a more useful timing model, if its meaning and
uncertainty can be established.

Microsoft documents `Direct3D11CaptureFrame.SystemRelativeTime` as the QPC time
when the compositor rendered the frame. Its `TimeSpan.Duration` uses 100 ns
units. The installed `windows-capture` 2.0.1 dependency returns that value from
`frame.timestamp()`. These units must be converted using QPC frequency, rather
than treating the raw value as a counter tick or wall-clock time.
[Frame timestamp documentation](https://learn.microsoft.com/en-us/uwp/api/windows.graphics.capture.direct3d11captureframe.systemrelativetime),
[TimeSpan units](https://learn.microsoft.com/en-us/uwp/api/windows.foundation.timespan.duration).

QPC frequency is fixed at system startup, and cross-thread counter ordering has
a one-tick ambiguity. The conversion uses integer arithmetic and outward
rounding to retain this uncertainty without subtracting large floating-point
epochs. [Microsoft's QPC guidance](https://learn.microsoft.com/en-us/windows/win32/sysinfo/acquiring-high-resolution-time-stamps).

Compositor timing does not establish a game update or presentation event.
A 2024 report in Microsoft's capture sample repository describes repeated
captures at compositor cadence; a maintainer discusses Windows-version
limitations. That report is a reason to investigate real images, not evidence
that every current system behaves the same way.
[Reported capture-cadence limitation](https://github.com/microsoft/Windows.UI.Composition-Win32-Samples/issues/127).

## Implementation

Native capture now samples QPC immediately around its existing screen operation,
with each read bracketed by the investigation's `Instant` clock. The optional
receipt preserves frequency, ticks, local bounds, policy and investigation ID.
Clock collection errors remain explicit without discarding an otherwise
successful image. Compatibility capture and old receipts keep their old shape.

Core conversion intersects the intervals derived from both anchors. It rejects
wrong clocks, reversed chronology, changed frequency, invalid counters, brackets
over 5 ms and timestamps outside the calibrated capture. Integer calculations
include one QPC tick, one 100 ns unit and local microsecond truncation.

The shared saved-image reader verifies the PNG checksum, decoded geometry,
selected RGBA hash and fixed selection policy before returning a correlated
interval. It retains failures per image. New metadata participates in the
existing integrity hashes. The optional `capture_clocks` result is diagnostic;
capture-operation bounds still govern input classification and comparisons.
It creates no controls, labels, permissions or new editing routes.

## Frozen experiment and checks

[Protocol](PROTOCOL.md), [runner](run.py), [summary and log hashes](results.json).
The one-shot probe calls local clock APIs through the production wrapper and
core conversion. It uses a simulated timestamp sampled inside each operation,
alternating between the calling thread and a worker. It calls no compositor
API, captures no image, accesses no target and uses no model.

| Group | Consistent measurements | Interval widths, min / upper median / max |
|---|---:|---:|
| All | 100 / 100 | 1 / 2 / 4 us |
| Calling thread | 50 / 50 | 1 / 2 / 4 us |
| Worker thread | 50 / 50 | 1 / 2 / 3 us |

All 170 frozen source, method and executable files passed the post-run audit.
The complete local records and failures are retained in
`test-results/capture-clock-v1/`; no samples were retried or removed.
Results SHA-256: `48f160650e4c42649208da3441caae64e405c13b76d06b61fa57c42f452e28c9`.

All **389** workspace test executions pass, along with strict all-target Clippy
and format checks. Added tests cover rounding at large uptimes and non-divisible
frequencies, stale/future and incompatible clocks, selected-image binding,
missing timestamps, legacy receipt serialization and shared-reader behavior.
The first integration assertion incorrectly expected metadata hashes to stay
unchanged after adding receipts; the corrected test checks updated hashes while
the selected activity and measurements stay the same. The failed log is retained.

## Limits and next evidence

The 1–4 us intervals describe this clock-only probe, not end-to-end capture
precision. Both clocks originate in the same machine's timing facilities; this
is an implementation check, not independent hardware calibration. No actual
Windows image timestamp has yet been correlated or validated by this change.
Older recordings cannot acquire missing clock anchors retrospectively.

A future image study needs an independently timed visual transition, retained
selected pixels and compositor receipts, including stale/duplicate frames and
different frame rates. Only then can we assess whether selected-image intervals
can support finer input association. Game-update timing, mixed actions,
semantic meaning and causal editing remain separate unresolved questions.
