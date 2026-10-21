# Named direct controls and explicit Freeze

Use the release workspace binaries unchanged throughout the native run. Retain
their hashes, source revision, shared-engine/MCP responses and helper receipts.
Use the independently controlled native runtime fixture in Anode; do not spend
game input or model screenshots on these UI/worker checks.

Check numeric and boolean holds, independent host changes followed by re-holding,
unchanged reads without redundant writes, explicit stop, replay/conflict handling,
root replacement, and restart with an active hold. Read bytes from the owned host
independently of the editor. Exercise Freeze, direct number entry and +/- through
native accessibility, with one layout image. Preserve failed evaluations. Verify
all owned children exit and release Anode.

Freeze is periodic and serialized with other work. It pins the first fresh object
and address chain, checks again before every write and keeps existing typed delta
limits. It pauses while investigation jobs run or an agent has lost control.
An in-flight check can finish before a stop is acknowledged. Off leaves the last
value in place. Restarts do not resume holds; interrupted writes remain unknown
and fence further writes. Address reuse and downstream effects remain unresolved.
Only mapped runtime numbers/booleans currently support Freeze. Other observations
display an unavailable checkbox until this capability is supported.

Sliders require explicit known small domain bounds. Numeric values alone do not
establish bounds. The default is exact text entry and an increment/decrement pair.
Generic unidentified OCR placeholders and unusable names do not become cards.
Metadata names describe provisional settings; they do not earn gameplay validation.
