# Separate window observation from the memory address space

The independent-browser baseline scanned the process owning the window. That
does not necessarily contain gameplay state. Reversebolt can now retain the
selected window for images and input context while observing one accessible
descendant's address space. This is a generic process relationship, with no
executable-name rules, game profiles, offsets or source layouts.

The desktop selector is under **Inspect application → Memory source**. MCP adds
`reversebolt_list_memory_processes`, taking the total to 30 tools. Attachment's
optional `memory_pid` and `memory_created` must appear together. Omission retains
the original window-process behavior. A selection always creates a new
investigation, discards current matches and uses the existing automatic job
path. Previous addresses, predictions and prepared edits cannot migrate into it.

`live.process` remains the window owner. `live.memory_process` identifies the
single address space for every current region, sample and candidate;
`live.memory_sources` is a refreshable inventory. Historical records without
`memory_process` retain their old window-process interpretation. Numeric,
prediction and rule evidence files record both identities. Trial-change
`scope.process` now identifies the actual memory process. The core and native
write checks both enforce that process's creation identity. Window capture and
input sampling continue to use the original window owner.

## Inventory and lifetime checks

Windows' [process snapshot API](https://learn.microsoft.com/en-us/windows/win32/api/tlhelp32/nf-tlhelp32-createtoolhelp32snapshot)
supplies entries; their [parent-process field](https://learn.microsoft.com/en-us/windows/win32/api/tlhelp32/ns-tlhelp32-processentry32w)
identifies the creating process. This is ancestry, not proof of gameplay ownership.
Each accepted edge additionally needs accessible live metadata, the same Windows
session and a child creation time no earlier than its parent. A reused parent
PID cannot qualify an older unrelated child. Missing/exited/inaccessible parent
metadata excludes that branch. Cycles are bounded; more than 64 examined family
members or 32,768 snapshot entries is an explicit inventory error. No partial
list is presented as complete after a budget error. Ordinary attachment can
still use its selected memory source when refreshing the family fails.

Attach refreshes ancestry and creation identities, then retains separate window
and memory-process handles. If either process exits or the selected window loses
its original owner, the connection invalidates. Changing an identity or choosing
an unrelated process rejects attachment before replacing the existing session.
The unchanged 1–32 MiB per-capture budget applies to one address space, not the
whole family. Sibling values cannot share an unqualified address trajectory.

## Automated evidence

All 136 workspace tests passed. New tests cover ancestry, reused parent PIDs,
cross-session exclusion, missing parents, cycles and inventory limits. The live
trial contract also runs with a window process and a different memory process:
it discovers mappings, prepares/applies/observes/restores a trial, records the
correct source in evidence, and rejects another or restarted memory process.
This contract uses a supplied renderer and is not a native game test.

The owned Anode fixture has a WinForms parent and two headless child workers.
It displays numbers reported by one worker. The normal native attachment and
capture path independently inventories and samples that child's memory; the
worker's arena address is **never supplied to the product**. Twelve captures
retain the arena's start and produce six correspondence hypotheses within it.
This is a known practice implementation, not independent-game acceptance or
proof of controlling state. No integration-test memory writes were requested.

The same real 30-tool MCP test rejects an unrelated engine PID, stale child
creation identity and incomplete source identity without replacing the existing
investigation. Selecting the sibling starts empty, and its subsequent exit
invalidates capture. Native accessibility inspection confirms the selected
child, optional source selector and unchanged simple cards. Invoking the desktop
selector starts a fresh automatic job that completes all 16 captures from the
window-owning process. No screenshots were viewed and no game was controlled.
Owned test windows/workers were closed and the Anode lease released.

A separate owned native write check retains distinct parent/child identities,
writes and restores four bytes in the child, and refuses stale bytes, changed
allocation metadata and a changed child identity. Its known arena is explicit
test knowledge, not discovery evidence. Strict Clippy, formatting, release build
and the packaged practice/MCP smoke pass.

The native UI run used the retained candidate executables in
`test-results/memory-sources-native-v1/binaries`. The final build additionally
corrects encoding in existing error text; the native write and packaged smoke
checks run on that final build. Both sets of executable hashes are retained.

## Limits and next step

Source selection is explicit. Reversebolt does not yet infer which family member
owns game state, scan several address spaces concurrently, reconnect an exited
worker, or recover learned controls after a source change. A browser family can
include unrelated tabs and service processes. The inventory exposes possible
sources without assigning them gameplay roles; names alone are not a selection
rule. Processes outside the accessible current descendant chain are unsupported.

This earns one point of native-access credit, **26% overall**, with all discovery,
causal-validation and withheld-generalization credits unchanged. Next work must
test selecting useful process scope on unfamiliar applications and finding actual
controlling state, rather than treating a child-process match as a verified edit.

Raw results are under `test-results/memory-sources-native-v1/`, with the standalone
write receipt, build/test logs and desktop delivery alongside it. The compact
[evidence receipt](evidence/v1.json) records hashes and outcomes. `verify.py`
checks those saved artifacts without launching a game or performing inference.
To repeat the integration, run `native.py <binary-directory> <new-output>
<Anode-session> <parent-session>` through a leased Anode seat. Every run needs a
new output directory. The runner waits for `finish-ui` after its checks so the
desktop can be inspected, then closes its owned processes.
