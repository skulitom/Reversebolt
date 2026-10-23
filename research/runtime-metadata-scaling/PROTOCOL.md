# Root-first metadata scaling

Keep the 200,000 retained-field, 50,000 loaded-class, 20-second metadata and
5 ms managed-pause limits. Inspect only cheap field headers when deciding whether
a declared class can own a static reference or embedded root. Bound that separate
stream to two million headers and 50,000 declared classes, with explicit counters.
Load complete layouts for eligible root owners, their parent/inline dependencies
and classes encountered in fresh bounded graph snapshots. Use no game names,
source-derived layouts, special field-name lists or cached object addresses.

Before another game run, use the owned runtime-values ABI fixture with an optional
large-metadata feature. It places a rootless class before the normal roots and
supplies configurable literal fields; an alternate mode makes that class a root
holder so retained-layout exhaustion is still tested. The ordinary fixture mode
must remain available without the feature. Its exported ABI asserts that metadata
calls occur only while the mock managed world is running.

Freeze sources, fixture and old/new readers before the native matrix. Compare
the old published reader and new reader on zero and 200,001 irrelevant fields.
Use the zero-noise run's measured header overhead to test exactly two million
headers and one more. Also test a required layout with 200,001 fields. Do not relax
limits in response to a failure. Retain errors and process exits.

For successful small and large cases, run the shared candidate parser and guarded
patch preparation. Check nested/array/enum values, change and exactly restore a
fixture value, refuse a readonly write, and reject changed layout/null references.
Compare field routes and values across old/new readers, ignoring process identity
and snapshot-local IDs. Check that root fingerprints/layout keys remain identical
where the fixture metadata is identical. Verify every mock stop has a matching
resume, and retained snapshot telemetry records no helper heap operations while
paused. Direct calls run only in owned subprocesses; they do not validate remote
loader unload or any game's behaviour.

Run workspace tests, strict Clippy and formatting after implementation. Build and
check the packaged desktop/MCP preview before activation. A native game evaluation
must be separately frozen and preserve the original Vampire Survivors failure.
Fixture scalability alone earns no independent-game generalization credit.
