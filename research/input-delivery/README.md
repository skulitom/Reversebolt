# Input response and selection purpose

Two distinct problems were investigated after the negative embedded sibling run:
whether research input reached a visible response, and whether a parameter-focused
selector was appropriate for finding state that changes during play.

## Native input diagnostic

Frozen [protocol](PROTOCOL.md) and artifacts: `test-results/input-delivery-v1`.
The same published `0.1.0-20261023-034437` shared engine captured eighteen images
from Sable in Anode. Eight nominal one-second D/A holds compared the old research
`keybd_event` call and the application's `SendInput` scan-code format in the
fixed legacy/send/send/legacy order. Only menu setup was viewed manually: four
images and two keys. Measurement used no manual image review, model call, target
memory read, runtime helper or field write.

Every action interval had greater rendered difference than either adjoining
quiet interval. Action differences ranged from 26.15 to 39.28 on the existing
0–255 grayscale metric. Ratios to the larger adjoining quiet difference ranged
from 1.79 to 188.46. Both input methods produced substantial rendered changes;
this result does not support blaming the earlier negative memory result on a
simple legacy-input failure. It does not retroactively establish delivery in
that earlier run, equal effects between APIs, or a specific gameplay meaning.

Quiet differences ranged from 0.09 to 15.64. Some visible change continued after
release. All eight action pairs lacked distributed patch-motion consensus (seven
had zero accepted patches, one had one), so these images do not establish tracked
actor displacement or physical speed. Large image change is not itself a useful
control, and camera/animation/environment effects remain possible explanations.

The QPC records verified selected foreground and the requested held key throughout
all recorded polls, with no gap reaching 50 ms. Both SendInput transitions were
accepted in all four of its trials. Actual bracketed durations were 984.15–1003.10
ms. The frozen runner used Python's monotonic clock, which this installation
reports as GetTickCount64 with 15.625-ms resolution; its nominal one-second
deadline was not an exact QPC duration. The current runner now uses perf_counter
for future deadlines. The frozen old runner, durations and measurements remain
unchanged; this correction has not been retested natively.

Direct captures have no job-owned input monitor. Accordingly, the product's
input classifications remain `unknown` and its activity_source remains null.
The diagnostic labels above come from the externally recorded fixed sequence
and QPC-bracketed holds; they are not substituted into a product assessment.
All eighteen compositor clock checks pass, and image hashes plus seventeen
rendered-difference values match an offline replay of the unchanged Rust metric.

The audit initially used the wrong shape for capture_clocks and assumed exact
one-second QPC timing. Those audit assumptions were corrected; no native trial
was retried or dropped. The actual durations are retained without a newly tuned
acceptance threshold. The owned game, headless editor and child processes exited;
Anode's lease was released. The main save, its image, settings and cloud metadata
are unchanged, while logs and SaveManager changed. The pre-launch folder is kept.
The main desktop editor remains open and responsive.

## Selection for the question being asked

The normal prompt explicitly prefers adjustable parameters. Parameters can be
useful controls while remaining constant during input; their inactivity is not
evidence that they are useless. Using only that preference to find changing state
was a mismatch in the previous experiment.

The observer now offers `runtime_type_selection_for` with explicit
`adjustable_parameters` and `current_state` purposes. The default prompt is
unchanged. The CLI accepts `--select-only --current-state`; full naming does not
silently switch purpose. Receipts persist the purpose, older receipts default
to adjustable parameters, and unknown purposes are rejected. Both paths retain
the existing evidence, paging, cancellation and request bounds. Neither grants
semantic meaning or editing authority.

A separately frozen [saved-report evaluation](STATE-SELECTION-PROTOCOL.md) used
the exact earlier Sable catalog, the same local model identity and unchanged
enclosing-role sampler, with the game closed. It completed 22 requests; the
largest prompt was 11,984 bytes. It selected eight observed types, sharing three
with the earlier parameter selection. The new plan has 128 paths across fourteen
roles, including 35 embedded paths; 62 paths overlap the earlier plan. Another
535 eligible fields were omitted. Observed labels now include Character and
GroundedState, but also ButtonPrompt. Labels and changed selection alone do not
prove relevance, actual activity or improved discovery.

No new native measurements were made for that plan. It belongs to the old exited
process and must never be used against a new PID. The next measurement needs
fresh discovery and selection, independently recorded image response and a
settling policy that retains ongoing post-release changes. Static parameters
remain a separate useful investigation path, rather than being filtered out for
not changing naturally.

The desktop preview remains the last native-tested build; this selection mode
is available in the shared observer and research CLI and is not its default
automatic investigation. Full-goal progress remains **36/100**.

All 475 all-feature workspace tests, strict all-target/all-feature Clippy and
formatting pass. A new compatibility check covers old receipt defaults, explicit
purpose round-tripping and rejection of unknown purposes. The first Clippy run
found a test fixture missing the added field; that fixture was corrected and the
complete check rerun. No default naming or semantic-promotion rule was changed.

Audits, without game or model access:

```powershell
python research/input-delivery/audit.py C:/DEV/Reversebolt/test-results/input-delivery-v1
python research/input-delivery/audit-selection.py
```

The first audit uses `target/release/examples/response_audit.exe`, built from the
unchanged response metric. Original sources/binaries and exact inference receipts
are retained under the frozen artifact directories. Nothing was uploaded.
