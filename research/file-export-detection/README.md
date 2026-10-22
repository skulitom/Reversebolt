# Detect runtime support through file reads

The loader no longer maps third-party modules locally just to identify runtime
exports. Both runtime detection and the helper's `ReverseboltInspect` address
lookup now parse ordinary PE file data. The list of 39 required functions is
generated from the same declaration used by the actual runtime interface.
This adds no game profile, game-specific address or field layout.

The earlier implementation used `LoadLibraryExW(DONT_RESOLVE_DLL_REFERENCES)`.
Microsoft [discourages that flag](https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-loadlibraryexw).
Removing it fixes unnecessary local DLL loading and possible loader interactions;
the security events do **not** establish it as the cause of the quarantine.
Real runtime inspection still loads the helper into the selected application.
That behavior, its safeguards and the unresolved Windows detection are unchanged.

The parser uses the pinned `object` 0.40.0 PE reader under its MIT license, with
an 8 MiB cumulative read budget checked before cache allocation. It reads headers
and export data, bounds name scanning, and distinguishes missing functions from
forwarded exports. Direct functions must point into executable, file-backed
image data. The helper's lookup uses the same locked bytes that were hashed.
There is no local game DLL mapping or entry-point call in this lookup.

Detection requires a stable module list and matching image sizes. It retains
bounded diagnostics for unreadable or malformed modules and does not advertise
support after an incomplete check. Forwarded required functions remain
unvalidated. On-disk exports are a compatibility hint: they do not prove the
current contents or behavior of a live process. The helper still rechecks runtime
functions at actual use.

## Verification

The [frozen eight-file evaluation](PROTOCOL.md) passed. Hyperbolica, Freight
Hopper and Manifold Garden each exposed all 39 required functions, using 15,952,
21,864 and 15,382 requested bytes respectively. Two UnityPlayer DLLs, Hyperbolica's
executable and kernel32 were correctly classified as non-providers. The published
helper exposed its one direct inspection entry point. Original input hashes were
unchanged after inspection. [Results](results.json).

No game, loader, editor or model was launched for that evaluation. The new
file-only diagnostic was the only application run by its evaluator. Three new
tests exercise malformed/truncated files, wrong architecture, ordinals, forwarded
and invalid function targets, duplicate names, bounded strings and read budgets.
All **296 workspace tests** and strict all-target Clippy pass. These tests do not
validate the changed native loader against a running game or clear the Windows
alert. Saved raw evidence is in `test-results/file-export-detection-v1`.

The desktop shortcut also now reports unavailable files or launch errors through
a small message instead of disappearing silently. `launch-preview.ps1 -CheckOnly`
reports availability without launching or focusing anything. The actual current
preview reports its missing `Reversebolt.exe`; PowerShell parsing passes. The
message dialog itself was not opened during automated verification.

The source changes are unpublished. The quarantined preview remains unavailable,
protection remains enabled, no external submission has occurred, and the full
goal remains active at **35%**. This is compatibility work, not new generalization
or causal-control evidence.
