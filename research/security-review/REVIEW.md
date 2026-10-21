# Reversebolt detection review — prepared, not submitted

Proposed destination: Microsoft Security Intelligence's software-developer
submission service. The executable and this technical summary would leave the
local computer only after explicit approval. No game saves, captured images,
authentication tokens or unrelated security history are included in this packet.
The compiled executable may contain ordinary build paths and library metadata.

## Exact sample

- Application: Reversebolt, an in-development, MIT-licensed Rust desktop tool for
  authorized offline-game inspection, experimental editing and agent access.
- File: `C:\DEV\Reversebolt\target\release\Reversebolt.exe`.
- Size: 24,252,416 bytes; unsigned development build.
- SHA-256: `a99d229af90a32e21425a43b8b5c353a90b914437c462b15f629bf6b2cd4060a`.
- This original build output matches the pre-detection frozen hash. Its separate
  test-directory copy was quarantined and has not been restored.
- The source snapshot predates current main. It includes changes after parent
  commit `b24fb2a`; exact frozen source hashes accompany the local diagnostics.

## Proposed explanation to Microsoft

Please review the detection of our unsigned Reversebolt development build as
`Behavior:Win32/DefenseEvasion.A!ml` (threat ID 2147738096). We suspect a possible
incorrect detection, but have not established that conclusion and are not
requesting that the detection be ignored without analysis.

On 2026-10-21 at 22:15:15 UTC, Defender detected a test copy of this executable.
Event 1117 records quarantine at 22:15:41 UTC. The initiating process recorded in
the events is `python.exe`. A Python evaluator had launched the headless engine,
collected observations from a selected offline game, terminated the owned engine
and attempted to restart it. That restart failed with Windows error 225.

The product intentionally provides process-memory inspection and guarded edits.
For supported IL2CPP runtimes, a separate loader allocates arguments in the
selected process and uses `WriteProcessMemory`, `CreateRemoteThread` and
`LoadLibraryW` to load our helper DLL. The helper inspects runtime metadata and
briefly pauses managed threads during bounded snapshots. This instrumentation
is not side-effect-free process access, even when no gameplay field is edited.
The product also supports explicit `SendInput` comparison actions, process and
foreground identity checks, and guarded restoration of experimental edits.

In the detected run, the model proposed no comparison: there were no gameplay
field edits or product-executed input actions. The evaluator supplied four short
key holds and the runtime helper completed and unloaded. The owned engine and
MCP process exited. These are retained application receipts, not independent
proof that every behavior was benign.

At detection, the event reports Defender product 4.18.26090.9, engine 1.1.26090.9
and security intelligence 1.461.139.0. A later custom scan of the exact original
build output completed at 22:58:37 UTC under security intelligence 1.461.140.0.
Scan events 1000/1001 share ID `{F2ABC9B7-FBFE-4E8C-A479-00C69F81D90C}`. No new
detection was observed for that scan. This does not explain or clear the earlier
behavioral alert, and we have not rerun the detected build.

Please determine whether this detection is expected for the described behavior
or is incorrect, and provide any available explanation of the triggering
behavior and appropriate compatibility guidance. Real-time antivirus and tamper
protection remain enabled. No exclusion, restoration, repackaging of the detected
executable or protection-setting change was used to continue execution.

## Local review materials

`test-results/security-review-v1/diagnostics.json` contains the selected Defender
status, relevant redacted events and exact file identity. `provenance.json`
contains the pre-detection source/binary hashes, excluding game storage records.
The diagnostic ZIP contains these text records and this review only; it contains
no executable. If approved, the exact existing executable above would be supplied
separately, with its hash rechecked immediately before upload. No submission has
been made and no sign-in or account consent has been performed.

Microsoft directs developers disputing a detection to submit the file for
analysis and await a determination. See the [submission guide](https://learn.microsoft.com/en-us/defender-xdr/submission-guide)
and [developer FAQ](https://learn.microsoft.com/en-us/defender-xdr/developer-faq).
