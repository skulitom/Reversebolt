# File-only export detection — frozen evaluation

Replace local `LoadLibraryExW(DONT_RESOLVE_DLL_REFERENCES)` checks with bounded
PE file reads. Microsoft discourages that flag for new use. This is a compatibility
and unnecessary-loading fix, not an established explanation or cure for the
Windows behavioral detection. Do not launch, restore or republish the quarantined
editor, change protection, or run an attachment test in this evaluation.

Before execution, freeze the evaluator, parser, required-export declarations,
loader integration, dependency lock, example binary and all input file hashes.
Run the file-only example once over eight existing files: three previously
selected games' GameAssembly DLLs, two UnityPlayer DLLs, one game executable,
Windows kernel32, and the published Reversebolt helper. File paths belong to this
evaluation only; product detection enumerates the selected target's modules and
uses the API names shared with its runtime interface.

Expected: three complete runtime providers; the two players, game executable and
kernel32 are not providers; the helper has a direct executable ReverseboltInspect
export and is not a provider. Each file read stays below the 8 MiB parser budget.
No game/helper code is mapped or called, no process is attached, and no gameplay
state is read or written. Reading files does not establish that their current
on-disk bytes are the same as a live process's mapped code.

Retain the full output and failures; do not alter cases after seeing results.
Unit tests also cover truncated headers/export data, wrong architecture, bad
ordinals, null/out-of-image/non-executable targets, duplicate names, forwarded
exports and allocation/read budgets. The loader still revalidates target identity
and runtime exports during real inspection; native integration remains untested
while the Windows detection is unresolved.

References: [LoadLibraryExW](https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-loadlibraryexw),
[PE specification](https://learn.microsoft.com/en-us/windows/win32/debug/pe-format),
[object 0.40.0](https://docs.rs/object/0.40.0/object/).
