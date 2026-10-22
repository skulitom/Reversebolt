# Authorized local desktop allowance

Artem first requested local review and then explicitly said **"you can allow it
in windows security"**. The earlier **"Keep it local"** instruction continues
to apply. No executable, review packet or diagnostic artifact was submitted.

The current source, including the complete module-inventory correction at
`2309d7f`, was built with the normal release publication script. Version
`0.1.0-20261022-233909` is now selected by the existing desktop shortcut.
All 445 workspace tests and strict all-target Clippy passed before publication.

The Windows Security window opened but was not returned as a targetable window
by the desktop automation API. A local elevated PowerShell helper instead used
Microsoft's `Add-MpPreference -ExclusionPath` for exactly:

`C:\DEV\Reversebolt\dist\0.1.0-20261022-233909\Reversebolt.exe`

The helper checked the executable's SHA-256 first:
`7536e30c9d8cfe62e1bd0bdd6fdf63ae75a1a8ed1a4d659f63aa43011460607b`.
It verified the path appeared in Defender's exclusions afterward. This is a
**path-based** exclusion; Windows does not bind it to the checked hash. It does
not include the project folder, a wildcard, a process-wide exclusion, the
helper DLL, the runtime loader, the MCP executable, or a threat-wide Allow rule.
Older quarantined artifacts were not restored.

The successful local receipt is
`test-results/allow-desktop-20261022-233909.json`; the reviewed helper is beside it
as `allow-desktop-20261022-233909.ps1`. Both pre- and post-action checks show
antivirus, real-time protection and tamper protection enabled. To undo this
one allowance, an administrator can use:

```powershell
Remove-MpPreference -ExclusionPath 'C:\DEV\Reversebolt\dist\0.1.0-20261022-233909\Reversebolt.exe'
```

The new executable started as process 98332 in interactive session 1. Its
responsive window and Controls screen, including **Connect to application**,
were verified on the main desktop. No target was attached and no gameplay
instrumentation was exercised during this launch check. There were no new
Reversebolt detections in the checked launch interval; this is not evidence of
future native-workflow acceptance or a resolved behavioral classification.

Build output is retained in `test-results/desktop-authorized-publish.log` and
the four published binary hashes in this release's `manifest.json`. The
full-goal capability estimate remains **35%**.

Microsoft documents exact-file exclusions in
[Add-MpPreference](https://learn.microsoft.com/en-us/powershell/module/defender/add-mppreference?view=windowsserver2025-ps).

## Subsequent published update

After the independent Viewfinder test exposed a large-catalog timeout, source
`1cd8e03` was published as `0.1.0-20261022-235934`. The standing authorization to
allow Reversebolt was applied to this release's exact `Reversebolt.exe` path
using the same reviewed helper, after checking SHA-256
`53d7944b1d622962d3a73a683d0fd5b5a492c7928e9c7a6020e99fcbbfab0c94`.
`test-results/allow-desktop-20261022-235934.json` confirms the file exclusion and
the same enabled protection settings at 2026-10-22 23:00:13 UTC (23 October locally).

The user's existing desktop session was attached to an application, so its
process 98332 and original release were left running. The desktop launcher now
points to the new release for the next close/reopen. Both exact executable paths
are allowed while that earlier session remains in use; no folder, process-wide
or threat-wide rule was introduced. The new release passed binary-hash and
launcher-availability checks. Its revised discovery has been exercised against
saved evidence, not yet in a new native game run.

## Live validation and the next reporting update (23 October local time)

The `235934` build subsequently ran discovery and five guarded comparisons in
Viewfinder through Anode. All five original values were restored; 31 helper
runs confirmed completion/unload. One measured response was suppressed and
returned after restoration. Checked protections remained enabled and no new
Reversebolt detection appeared in the interval. This still does not establish
vendor clearance or explain the earlier detection.

The resulting reporting fix, source `f979c64`, is published as
`0.1.0-20261023-002433`, SHA-256
`76b6feadbe2071f1998ba24f86e8ea5f22128b6e15ce1d982ea391ed59e36e84`.
All four manifest hashes and the launcher-availability check pass. The user's
connected desktop process 98332 remains on the earlier `233909` release.

The same exact-file allowance helper was prepared for `002433` under the
standing authorization. At this documentation checkpoint its Windows
administrator confirmation is pending, and
`test-results/allow-desktop-20261023-002433.json` does not exist. Do not infer
allowance from publication or restart the helper while the original operation
is live. The pending-operation record is
`test-results/pending-allowance-20261023-002433.json`. No broader rule or upload
was requested. Recheck the existing operation and its receipt before updating
this status.
