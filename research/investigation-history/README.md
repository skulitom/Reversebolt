# Keep earlier comparisons while starting another investigation

The old comparison store allowed only eight records across its entire lifetime.
Completing tests, switching games or restarting did not free a slot. The planner
could suggest another comparison even though preparation would reject it.

The shared store now separates **eight saved comparisons per investigation** from
**128 retained historical records**. Existing IDs cannot move investigations to
evade the limit. Old requests still return their original result, existing records
can save their final state at capacity, and restart stops unfinished work without
discarding history. Full history is still bounded; no automatic pruning, unlimited
retention or cleanup interface is claimed.

Planner responses include `record_capacity`, and `next` is null when no new record
fits or storage has faulted. This reports record capacity, not guaranteed capture
space. Before another capture, the store counts existing files, including prior
and interrupted evidence, against a shared **512 MiB** budget and the existing
**64 MiB per comparison** limit. It reserves headroom for image/input/record updates.
Traversal has one evidence-directory level and a 16,384-entry limit; unsupported
links or nested content stop reservation. Existing evidence remains readable at
capacity. Metadata settlement is allowed without requiring a new capture budget.

## Verification

All **306 workspace tests**, strict Clippy, formatting and diff checks pass.
New tests cover:

- Eight closed records from one investigation, followed by a ninth from another,
  preserved across restart without rewriting the original closed records.
- The separate 128-record limit, with existing records still able to save updates.
- The actual app command/checkpoint path: old request replay, conflicting-request
  refusal, capacity refusal before target access, nine published summaries and
  historical reads after engine restart.
- Shared filesystem quota accounting for previous/interrupted evidence and
  unsupported nested content, using small injected limits instead of large files.

The [frozen replay](PROTOCOL.md) also used a disposable copy of completed comparison
`080b5643-5718-494f-9ed6-751bf8059402` from the earlier planned-input **owned ABI
fixture and striped renderer** evaluation. The previous version incorrectly
called it Manifold Garden; [the provenance correction](PROVENANCE-CORRECTION.md)
identifies the source. It retained that record, added eight explicitly synthetic stopped
records and reopened the store. The ninth record in the original investigation
was refused; one record in a new investigation was accepted.

The original record and all **90 evidence files** are unchanged. Production readers
verified **18 image/input pairs and 18 execution receipts** after restart. The
earlier assessment remains historical evidence; it was not recomputed or promoted
to a new result. Synthetic records contain no observations, assessment, origin or
preparation. The run completed in 0.140 seconds with no target access, game/editor,
model call, input or gameplay write. [The audit](audit.py) and [results](results.json)
retain source/input/executable hashes and verify the original files remain intact.

This improves continuity of investigations, not semantic discovery or native
compatibility. Weighted progress remains **35%**. The desktop preview remains
unavailable after quarantine; no replacement or external submission was made.
