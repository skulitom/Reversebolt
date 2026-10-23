# Embedded runtime state

Reversebolt now reads nested structures, closed generic value types, arrays of
structures/enums and static structures using runtime metadata. Enums use their
declared underlying numeric representation, retaining the enum type in the layout
fingerprint. No game-specific profile, field name, offset or source layout is used.
The extension is optional; providers without its three metadata exports retain
scalar/reference support. Discovery schema 5 distinguishes embedded storage from
object references; the shared core still accepts schema 4.

Inline fields use the runtime header size to adjust boxed field offsets and are
bounded against the runtime value size. Nodes distinguish storage kind and type
even when nested structures start at the same address. Fresh paths retain exact
type/layout guards without addresses in their locators. Read-only enclosing
structures prohibit scalar writes; following an object reference starts a new
container. Existing guarded edits/restoration work on supported scalar leaves.

## Evidence, 2026-10-23

Frozen artifacts: `test-results/runtime-values-v1`; protocol [here](PROTOCOL.md).

* Owned ABI test: 41 calls and 41 balanced pauses. Nested/array/static/enum edits
  restored in seven cases; two readonly cases refused the write. Null roots,
  shortened arrays, changed layouts, zero-offset nesting and out-of-bounds fields
  behaved as expected. The fixture aborts on metadata calls during pauses or on
  object APIs receiving unboxed data.
* Older provider compatibility: two calls, seven balanced pauses, four fresh reads.
* Independent installed Sable: two discovery pages cover 4,173 catalog roots.
  The retained snapshots contain 7,766 embedded nodes and 74,512 scalar reads.
  Node/read counts are observations and can alias or repeat. A deterministic
  32-path sample includes 23 paths through structures and nine enum paths;
  all 96 fresh readings succeeded. All eleven helpers unloaded, across 201 pauses.
* No helper Rust allocation/deallocation occurred during any of the 249 observed
  pauses. Maximum stop-to-resume was 5,669 microseconds in Sable; the five-ms work
  budget is not a guaranteed maximum scheduling/stop latency.
* Comparing retained runtime descriptors with the previous Sable launch finds
  975 of its 998 unsupported value-type fields: 582 now describe embedded values
  and 393 numeric enums. The other 23 were not encountered. Separate sessions
  and bounded graph samples make this a coverage diagnostic, not a recall score.
* The first discovery page builds a valid shared-core naming inventory: 555 types,
  2,656 fields and 63,820 reachable scalar paths. Meaningful labels and useful
  controls were not evaluated. More reachability also increases the naming load.

No gameplay field was changed. The game required two menu key presses and three
viewed setup images; measurement used no input, images or model calls. The main
save, image, settings and cloud metadata are unchanged; logs and SaveManager changed
and the pre-launch folder is retained. The owned game, reader and preview exited;
the user's existing desktop editor remained responsive and untouched.

Preview `0.1.0-20261023-030845` was launched normally and checked through Anode
accessibility. Its reader/loader hashes match the native-tested binaries. It is
selected for the next desktop launch; the running older editor was preserved.
No security allowance, exclusion or protection setting was changed.

The 462 workspace tests with default features, strict all-target/all-feature
Clippy, formatting and the frozen native artifact audit pass.

## Limits and next measurement

This is additional runtime access, not verified semantic control discovery. The
full-goal estimate remains **36/100**. Static primitive/enum roots, enum labels,
unsafe pointers/byrefs, invalid/recursive layouts and wider native-engine support
remain outside this extension. Existing read/node/depth/time budgets still omit
reachable state. Process-local role paths are not durable object identities.

Next, repeat the bounded sibling/input experiment with this expanded coverage,
preserving the enclosing field context of structured leaves and measuring state
before asking the model for complete semantic names. Establish a useful relation
or intervention before claiming another control or generalization credit.

Reproduction: build the workspace and `mock/Cargo.toml`, run `freeze.py` with a new
absolute output directory, then `native.py` against that directory. `live.py`
requires a separately prepared owned-process identity and frozen script protocol
inside Anode. `audit.py OUTPUT` audits the retained run without launching anything.
