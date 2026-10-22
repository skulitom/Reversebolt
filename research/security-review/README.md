# Windows security detection investigation

**Latest status, 2026-10-22:** Artem explicitly authorized allowing Reversebolt
locally. The latest source was built as `0.1.0-20261022-233909`; a Windows Defender
file exclusion was added for that release's `Reversebolt.exe` only after checking
its SHA-256. The application is running on the main desktop and its Controls
screen was verified. Antivirus, real-time protection and tamper protection remain
enabled. No upload was made. The classification remains unexplained; this is an
explicit local allowance, not vendor clearance. [Scope and evidence](LOCAL-ALLOWANCE-2026-10-22.md).

**Earlier status, 2026-10-22:** a second published build was detected after a
native discovery run. Windows quarantined it and closed the desktop preview;
hashing its build-output copy is also refused. No replacement was published.
The [follow-up review](FOLLOWUP-2026-10-22.md) and local text-only packet retain
the new events and frozen identities. Artem answered the submission request with
**"Keep it local"** on 2026-10-22. Do not submit the executable or review packet.
The original investigation below describes evidence at its collection time.

An additional event-log read confirms event 1117 at 00:26:26 UTC records quarantine
of `target/desktop-preview/release/Reversebolt.exe`, following the earlier read
refusal. [Retained build-copy events](build-copy-events.json). The existing review
packets remain unchanged; no restore, relaunch or upload followed.

Subsequent source inspection found a compatibility concern:
export detection used the discouraged DONT_RESOLVE_DLL_REFERENCES flag. Source
now uses bounded PE file parsing; an eight-file offline evaluation and 296 tests
pass. This removes unnecessary local DLL mapping, not the runtime helper's actual
instrumentation. It is **not** evidence that the detection is resolved. The change
is unpublished; both exact review samples/packets remain separately documented.
[Implementation and evidence](../file-export-detection/README.md).

The preceding native evaluation could not restart its headless engine after
Defender quarantined the test executable. The same condition remains relevant
to desktop publication and new live-game validation. Offline model/code work
does not establish that the behavioral detection has been resolved.

Read-only investigation identifies the exact sample, an unsigned 24,252,416-byte
build with SHA-256 `a99d229af90a32e21425a43b8b5c353a90b914437c462b15f629bf6b2cd4060a`.
At collection, the original build output was present and matched the pre-detection
frozen hash; the quarantined test copy was absent. Defender events identify
`Behavior:Win32/DefenseEvasion.A!ml`, threat ID 2147738096, and `python.exe` as the
initiating process. They do not identify the precise underlying behavior that
caused the classification. Neither a keyword search nor our source inspection
can establish that the alert is a false positive.

A custom scan of that exact existing build was run without executing it or
changing protection settings. Matching start/completion events confirm the
scan finished at 2026-10-21 22:58:37 UTC; no new detection appeared. The newer
security-intelligence version was 1.461.140.0, versus 1.461.139.0 in the original
alert. A clean on-demand scan does not clear a prior behavioral alert or prove
that repeating the live workflow would be accepted.

The [prepared review](REVIEW.md) describes the tool's intentional process access,
DLL instrumentation, editing and optional input capabilities. It distinguishes
bootstrap instrumentation from gameplay-field writes and avoids calling the
program harmless or the detection incorrect without a vendor determination.
Its accompanying local packet excludes saves, screenshots, credentials and
unrelated security events. The packet contains text records only, with the
original executable identified for a separately approved submission.

`scripts/collect-security-diagnostics.ps1` reproduces the read-only collection
while the exact original build remains present. It refuses a different sample
hash or an existing output directory. It neither scans nor launches a program,
changes settings, restores quarantined content, or uploads anything. The scan
above was a separate, explicit on-demand scan of one generated artifact.

Microsoft's documented developer-review route requires submitting the file to
its analysis service. That external submission is prepared but not authorized
or performed. At packet preparation the desktop preview was quarantined; progress remains
**35%**. The full goal is still unfulfilled; resolving native validation remains
necessary before claiming new verified gameplay controls or generalization.

The completed 2026-10-22 packet is
`test-results/security-review-v1/Reversebolt-security-review.zip` (9,894 bytes).
It contains exactly `REVIEW.md`, `diagnostics.json` and `provenance.json`; no
executable is inside the archive. [Packaging checks](results.json) retain each
member hash and the archive hash. The original 24,252,416-byte executable was
rehashed against the pre-detection manifest; its quarantined copy remains absent.
Matching scan IDs were checked, and retained events contain no detection in that
scan window. None of this establishes behavioral clearance.

The prepared review was opened for Artem and explicit approval for external
submission was requested. Artem declined external submission: **"Keep it local"**.
Nothing has been uploaded and no Microsoft sign-in or account consent has been
performed. Continue local development and testing. The frozen review packets and
follow-up narrative retain their original contents and pre-answer status;
this current instruction supersedes their references to pending approval.

Official sources: [file submission](https://learn.microsoft.com/en-us/defender-xdr/submission-guide),
[developer FAQ](https://learn.microsoft.com/en-us/defender-xdr/developer-faq),
[custom scan syntax](https://learn.microsoft.com/en-us/powershell/module/defender/start-mpscan).
