# Complete runtime module inventories

The runtime loader previously treated every `Module32NextW` failure as the end
of a module list. A failed or truncated enumeration could therefore appear to
prove that the helper was absent or that an IL2CPP provider was unique.

The shared collector now requires `ERROR_NO_MORE_FILES` after at least one
module, captures Windows errors immediately, and rejects inventories larger
than 512 entries. The existing detection limit now also applies to load and
unload checks. Partial and empty inventories propagate errors to callers.
This establishes completeness of a snapshot, not continuing absence afterward.

Three generated unit tests cover normal completion, failed first reads,
interrupted reads, misleading success error codes, and the exact/overflow
budget boundaries. All **445 workspace tests**, strict all-target Clippy and
formatting pass. The generated tests do not load a helper or inspect a game.
Logs and source hashes are listed in [results.json](results.json).

This is a concrete reliability correction found during the local security
review. It does **not** identify the cause of the Defender alert or establish
that the application is a false positive. The full-goal estimate stays **35%**.

Microsoft documents the end-of-list sentinel in
[Module32NextW](https://learn.microsoft.com/en-us/windows/win32/api/tlhelp32/nf-tlhelp32-module32nextw)
and first-entry failure behavior in
[Module32FirstW](https://learn.microsoft.com/en-us/windows/win32/api/tlhelp32/nf-tlhelp32-module32firstw).
