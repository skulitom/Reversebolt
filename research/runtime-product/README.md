# Internal-value cards through the shared desktop/MCP workflow

Runtime discovery and fresh path reads are now integrated into Reversebolt's
managed jobs, desktop cards and MCP. Connecting a process with the supported
IL2CPP export set automatically starts one internal-value discovery job. Other
processes retain the existing visual/memory observation flow. The new cards show
a metadata-derived name, current reading, short context, age and Refresh value
action; they use the existing arrangement and inspection controls.

These are provisional internal readings. Their names are not verified gameplay
descriptions, and they do not authorize runtime edits. The product still lacks a
useful hidden control with independently verified gameplay effects. Weighted
progress remains **31%**, and the full goal remains active.

## Implementation

- The shared core builds at most 64 displayed candidates from the bounded graph,
  prioritizing declarations outside the runtime-reported core library and
  distributing slots across images. No game/image/field names select candidates.
- Refresh accepts one to four IDs from an exact catalogue/revision. It resolves
  current path occupants afresh. Unavailable paths clear their current values;
  previous candidate observations remain explicitly historical. A collection
  slot is not a persistent actor identity.
- The normal job controller owns discovery/refresh, replay, stop and human
  takeover. Runtime requests are bounded, immutable SHA-256-identified documents;
  their references keep steps inside the existing 256-byte journal ceiling.
- The detector only examines locally mapped module exports. Actual inspection
  loads a pinned helper into the selected process, attaches to its runtime and
  briefly pauses managed threads. This is instrumentation, not passive remote
  reading. A durable unresolved-operation record prevents blind reinjection;
  cancellation waits for an active helper to settle. No gameplay fields are written.
- MCP now exposes 37 tools, including `reversebolt_start_runtime_job`. Desktop
  and agent requests reach the same engine and current catalogue.

## Native evidence

[Protocol](PROTOCOL.md), [repair protocol](PROTOCOL-v2.md),
[artifact/result audit](audit_postrun.py), [results](results.json).
Exact source/binary manifests remain under `test-results/runtime-product-v1`
and `runtime-product-v2`; all ten binary copies and 128/129 source identities pass
the post-run audit against their committed revisions.

The v1 run (`cb1e0120e375b354967d95135db7a73a8792ef3d`) found a real integration
defect: automatic attachment produced 64 cards, but a four-value refresh exceeded
the journal's small step limit. Its helper unloaded, and the fixture exited with
five balanced pauses. That failure and its original frozen artifacts are retained.

The v2 repair (`82d0eae7a91908acf64e0dd230407ea526dcb6ed`) passed the unchanged
runner against an owned windowed ABI fixture. Automatic discovery produced 48
cards. Four values refreshed, exact request replay reused the completed job,
and a fresh request with the old cursor was rejected without another helper.
Null roots cleared current readings; restoring structure allowed fresh reads.
All four helper operations unloaded, and the fixture exited with eight starts
and eight stops. Helper Rust heap operations during pauses were zero.
Accessibility inspection found named cards, arrangement controls and Refresh value
actions. This fixture simulates a runtime API; it is not independent-game evidence.

The same frozen product/runner then connected to a fresh Blue Prince process
(PID 61364, creation `01dd616cde246d0e`, session 2). Its GameAssembly SHA-256 was
`88a2507c06bd540ddd5f78e4d039b4cc21de7ac7774a2004d5771271b26ab111`.
The automatic job visited all 4,267 supported root slots and published 64 cards
spread across 24 code images. Full helper discovery took 21.849 seconds; graph
coverage remains partial despite root completion.

Four generically selected non-core declarations refreshed through MCP in 2.688
seconds, including a five-step path. They were `ignoreRecompile`, `IsEnabled`,
`inspectorUpdater` and `isGameIntent`: three flags and one numeric value, all
unchanged. Fresh metadata preparation took 2.271 seconds; two managed pauses took
147 and 194 microseconds, with zero helper Rust heap operations. Exact replay and
stale-cursor refusal also passed. Both helpers unloaded, the game remained
responsive, and the owned app/game/fixture processes were closed. Steam remained
in the user's session; the Anode lease was released.

Both native cases used **zero screenshots, zero gameplay input and zero gameplay
writes**. The app cards were inspected through accessibility rather than images.
No gameplay control, semantic label or cross-game generalization is established.

## What the selection result means

The structural priority avoids letting the core library consume every slot, but
it is not sufficient for useful discovery. The three displayed fields in the main
game image belonged to localization state (`IsEnabled`, `LanguageIndex`,
`_lastIndex`). Merely preferring application assemblies would still surface those
before health, damage or movement. Main-menu observations also cannot establish
what objects exist during play.

Next expose a bounded searchable candidate catalogue instead of making the first
64 cards the agent's only selection surface. Rank hypotheses using object context,
relationships and changes over time, then test actual effects. Preserve the
native-memory fallback for unsupported runtimes and use images/motion only when
they help distinguish consequences. Do not tune field names or offsets to Blue
Prince, and do not count raw field/card totals as progress toward useful controls.

## Validation and desktop preview

All 200 workspace tests passed before the journal repair. After repair, all 41
application tests passed (40 existing plus the new maximum-size request/document
integrity regression); the other crates were unchanged. Strict workspace Clippy,
formatting and the release build passed. The frozen v2 37-tool smoke passed native
practice writes/restores, shared session/MCP, authentication, stale restoration
and restart invalidation (`test-results/smoke-f59105e1/report.json`).

Preview `0.1.0-20261021-160254` is running on the user's desktop, PID 89776.
All four published product binaries match the evaluated copies; the model files
passed the publisher's checks. All 29 history entries and `qwen3.8:latest` survived
the update. See `test-results/runtime-product-v2/desktop.json`. The shortcut
continues to launch the current preview.
