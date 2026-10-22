# Current desktop preview check

Preview `0.1.0-20261022-010125` contains the current Rust source at `b557930`,
including the named-card filters, prominent numeric entry with −/+ and up/down
keys, sliders only for known small ranges, and Freeze availability checks.
It also includes the shared agent context and internal object-text questions.
Questions remain separate from visible card names and edit permission.

The workspace release build completed successfully using the normal Cargo
`--target-dir target/desktop-preview` option. This preserves the exact original
executable and packet prepared for the pending Windows detection review.
`scripts/publish-preview.ps1` now accepts the same optional target directory;
its default remains `target`.

Anode launched the new application with an isolated data directory. Accessible
controls verified window startup, picker opening, search to zero matches,
disabled Connect without a selection, cancellation, and normal window close.
No application was selected, game attached, model called, memory edited, or
screenshot reviewed. The test window exited and the Anode lease was released.
The 292 workspace tests already cover control edits and naming rules; this
native smoke check only proves window/picker behavior.

The old desktop preview was idle with no live connection, active hold or
in-flight job. It accepted a normal close. The new published preview started
on the user's desktop and returned a responsive shared-engine status, preserving
44 history entries and `qwen3.8:latest`. The version manifest matches the four
published binaries. Its MCP stdio catalogue check passes with 53 tools, no
engine connection and normal process exit.

The new application hash is
`5a88e59668b6aa165f889309b8731d25fe8f3925deb38cad88e1df3f05fdbfe9`.
The original review sample remains
`a99d229af90a32e21425a43b8b5c353a90b914437c462b15f629bf6b2cd4060a`.
Real-time protection, antivirus and tamper protection remained enabled; no new
Reversebolt detection event was observed in this check. This does **not** clear
the earlier behavior alert, establish a false positive, or validate game
attachment/edits. Nothing was submitted externally and no protection settings,
exclusions or quarantine state were changed.

Local evidence is under `test-results/desktop-preview-ui-v1`: build identity,
accessibility assertions, normal-restart record, status/manifest/protection
verification and MCP receipts. The build log is
`test-results/desktop-preview-build.log`. The isolated test directory and saved
pre-update session are retained locally; authentication tokens are not published
in these notes. Weighted goal progress remains 35%.
