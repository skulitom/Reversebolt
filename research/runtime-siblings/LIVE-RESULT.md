# Sable: valid input-bracketed reads, no repeated association

Progress remains **36%**. The selected 128 scalar paths did not produce a repeated
input response under the predeclared D/A/D/A activity. There are no new verified
gameplay controls. This is a development result on a known game, not independent
generalization evidence.

The original automatic investigation failed before measurement: fourteen
type-selection requests completed in 38.854 seconds of model generation, but the
first naming proposal timed out after 90 seconds. Its prompt was 50,909 UTF-8
bytes with a 32K context. The entire evaluator ran for 208.822 seconds. The exact
request, failure and earlier completed stages remain retained. This reproduces
the large-request reliability gap; it does not establish the cause of the timeout.

A separately frozen continuation used all 256 paths from that completed partial
type selection. It collapsed those paths into ten rooted roles, then selected
128 neighbouring fields without ranking their names or values. All roles were
represented; 154 additional eligible fields were outside the budget. Type
selection was still model-assisted. The continuation required no additional
model request, screenshot or setup input. This route is currently a shared Rust
API and research command, not an automatic desktop/MCP fallback.

## Retained defects and corrections

Before live launch, the saved-data v1 batches were found to exceed the loader's
separate UTF-16 message limit despite fitting its byte/file limit. V2 reserves
message space as well as limiting the file size. The saved 117-field Sable case
now takes seven batches and the 128-field Viewfinder case eight. The earlier
four-path fixture had tested clock bounds, not these large batch sizes.

During the continuation, all 98 native reads succeeded, but all seven original
analysis calls rejected their batches through a UI task restricted to four
values. Those failures remain intact. Native refresh validation is now a shared
`runtime::validate_refresh` API, independent of UI task limits. The existing UI
handler and offline analyzer use the same process/path/representation checks.
A 32-field regression verifies successful responses, a missing read and a
contradictory value beyond the fourth field. UI limits were not enlarged.

The repaired analyzer was separately frozen and replayed against the same saved
observations. There was no additional target access or input, and no change to
timing or association thresholds. All seven repaired analyses accepted their
foreground/key/QPC evidence and all 1,792 scalar readings. Every path retained
its observed address chain throughout its fourteen samples; this still does
not establish persistent actor identity or exclude address reuse.

## Native outcome

| Measurement | Result |
| --- | --- |
| Rooted roles / selected paths | 10 / 128 |
| Eligible fields omitted | 154 |
| Read batches / samples per batch | 7 / 14 |
| Native helpers, including discovery | 99, all unloaded |
| Recorded managed pauses | 340 |
| Maximum stop-to-resume interval | 5.681 ms |
| Rust heap operations while paused | 0 |
| Constant sampled paths | 124 |
| Changing sampled paths | 4, changing in every sample |
| Four-repetition input associations | 0 |

The four changing paths lead to two snapshot addresses, labelled `_hoverHeight`
and `_lastGroundedTime` in the runtime metadata. Both changed during quiet
samples as well as action samples. The analyzer correctly refused to present
that drift as a repeated input response. These labels remain unverified clues.

The full retained report lists **998 value-type field descriptors** with zero
supported width, including **22 in the selected types**. These fields were never
measured. Together with the 154 eligible but omitted scalar fields, this is a
concrete coverage gap. Extending generic value-type observation and measuring
activity before completed semantic naming is the next investigation. A negative
result from the selected scalar subset does not show that relevant state is
absent from the process.

## Costs, cleanup and verification

The continuation took 218.188 seconds. The complete session used fifteen
automatic images, four manually viewed setup images, four 200 ms discovery
inputs and twenty-eight nominal three-second measurement holds. Measured hold
durations ranged from 2,984.984 to 3,003.484 ms; scheduling is not a hard real-time
guarantee. Zero measurement images were manually viewed. No gameplay fields were
written or frozen.

The owned game, engine, evaluators and MCP clients exited after all native work
settled. The original save backup remains local. The main save, image/settings
and cloud metadata are unchanged; `Player.log`, `Player-prev.log` and
`SaveData/SaveManager` changed. The user's desktop editor remains responsive.
Windows protections remain enabled with no new Reversebolt detection, and no
security allowance was changed. Nothing was uploaded.
The native-tested preview `0.1.0-20261023-021904` is selected by the desktop
shortcut for the next launch; the existing connected window was preserved.
The partial-review and repaired analysis work remains in research/shared APIs.

All **471 workspace tests**, strict all-target Clippy and formatting pass. The
original failure, frozen continuation, repaired replay and coverage diagnostics
have separate retained artifacts under `test-results/runtime-siblings-sable-v1`
and `test-results/runtime-siblings-sable-selected-v1`. See
[live protocol](LIVE-PROTOCOL.md), [partial-review continuation](SELECTED-PROTOCOL.md),
[reanalysis protocol](REANALYSIS-PROTOCOL.md), [failure audit](audit-failure.py),
[native audit](audit-live.py), [coverage diagnostics](coverage-diagnostics.py), and
[compact results](live-results.json).
