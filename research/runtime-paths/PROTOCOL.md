# Refreshable runtime paths v1 — fixed before native evaluation

Purpose: turn observed scalar values into paths that a shared Rust engine can
describe and a runtime reader can follow afresh. Paths contain exact process
identity, runtime-qualified type names, declared-layout digests, field slots and
array indices. They contain no memory addresses. A path names the current
occupant of a role, not a persistent object or established gameplay control.
No game-specific names, offsets, signatures or source layouts enter discovery.

Run unit/format/Clippy checks and build before freezing. Copy the exact helper,
loader, candidate converter, fixture and reused object-host binaries into
`test-results/runtime-paths-v1/bin`; native runners use only these copies. Freeze
source and binary hashes, then commit source and record that revision before
native execution. Do not rebuild or replace frozen binaries. Retain failures.

Owned fixture: automatically discover child scalar paths through a collection.
Refresh three times while objects are replaced and addresses reused. Require
coherent identity/hidden/inherited values and a primitive array element, no old
addresses in locators, and identical path IDs across changed values. Reorder the
collection and require the current slot occupant, not the former object. Null
roots, shorter arrays, changed generic arguments with the same short class name,
changed field layout and unavailable fields must abstain explicitly. Restore the
structure and require fresh reads to recover. A wrong process identity must be
rejected before loading; an unavailable root layout must not read a value.
Check zero helper heap operations during every pause, balanced stop/resume,
unload receipts and host exit. This fixture is an ABI simulation, not a real GC.

If the fixture passes, launch Blue Prince in Anode with no save selection or
gameplay input. Run exactly one unchanged discovery request (partial root
coverage is allowed and reported). Use the shared core candidate builder.
Select the first four candidates with at least one reference hop and distinct
terminal descriptors, with no game-name filtering. Run exactly one targeted
refresh; require four successful scalar reads, matching path IDs, no edit
authority and zero helper heap operations during pauses. Preserve the original
failure if any; never repeat a timed-out or uncertain remote thread. Confirm
unload and responsiveness, close the owned game and release the lease.

Refresh limits: 32 paths, eight steps each, 64 KiB file input and 32,768 UTF-16
units for the complete helper request. Every owner type/layout and declaring
field is checked against newly obtained runtime metadata. Array indices must
be below current length. Up to nine paused attempts may resolve newly encountered
types between fresh attempts. Five milliseconds is checked between operations,
not a hard real-time guarantee; no new attempt starts after 18 seconds. Metadata
limits remain inherited from traversal. No field writes or persistent object
handles are introduced. The shared API/converter is not yet wired to desktop/MCP.

Progress remains 31% until useful verified capabilities meet the existing rubric.
