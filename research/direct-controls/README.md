# Prominent direct controls

Live findings now use a large number field and a contrasting slider, or an On/Off
checkbox. Slider drags apply once on release; text applies on Enter or focus loss;
checkboxes and accessible SetValue requests apply directly. Undo remains beside
the short status. The small details button contains reading history, refresh/watch
and advanced inspection. No review/apply menu is needed for a direct edit.

`set_runtime_value` and `set_observed_value` share these actions with MCP. Each
requires a fresh canonical request UUID and exact investigation context. A saved
identical request returns its retained result without another attempt; conflicting
arguments are refused. Fresh preparation, durable intent, native guards, unknown
outcome holds and guarded restoration remain behind the direct gesture. Numeric
runtime adjustments still have the existing limit of 20. Runtime booleans require
canonical 0/1 bytes. OCR edits still require eligible, agreeing mappings and a
fresh screen check. Metadata names and successful writes do not prove meaning.

The initial native run found that egui's text field advertised a UIA value
provider but ignored string SetValue actions. Explicit handling now preserves
exact integer text, consumes each action once, and respects disabled controls.
The failed v1 evidence is retained. The v2 run passed; visual review then led to
a brighter slider rail and larger checkbox. The final v3 run repeated the full
native workflow with those visual changes.

Final evidence at source revision `348d5c3d32039daedfb34989a9293af1fc82f7d1`:

- 15 workflow checks and six native accessibility actions passed: number edit,
  slider adjustment, checkbox toggle, and Undo after each.
- Five trials were restored, including two initiated through MCP. Repeated IDs,
  collisions, noncanonical IDs, restart retention and no replay were checked.
- All 16 helpers completed and unloaded; 35 fixture pauses were balanced, with
  zero helper Rust heap operations while paused and a maximum pause of 67 µs.
- Six independent host byte readings agreed. Source/binary/report hashes and
  owned-process exits were audited. The 44-tool shared-session smoke passed.
- The 231-test workspace run passed before the accessibility fix; all three
  final UI regression tests, updated OCR tests, strict Clippy, formatting and
  release builds passed. Pointer-event testing confirms one commit at release.
- One UI layout image was inspected in each successful run. No game screenshots
  or gameplay input were used. The evaluation target was an owned ABI fixture.

The fixture's boolean extension is behind the `direct-controls` research feature;
production receives no fixture layout knowledge. See [protocol](PROTOCOL.md),
[retained results](results.json) and local `test-results/direct-controls-v3`.
This establishes UI and byte-edit mechanics, not real-game understanding,
useful effects, complete rollback, persistent identity or generalization.
Full-vision progress remains 31%.

Preview `0.1.0-20261021-183718` is running on the user desktop. All four product
binaries match the evaluated hashes; all 29 prior history entries and the local
observer model were preserved. The previous preview was already closed at update
time; the launcher opened the new build without terminating a user process.
