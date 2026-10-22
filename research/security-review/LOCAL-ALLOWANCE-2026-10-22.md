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
