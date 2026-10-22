# Stable regions for a fresh pre-edit baseline

The shared test reader and final baseline observation now use the stable source
texture selector before spending their bounded template-matching budget. This
helps an agent choose a region for checking a later value change. It does not
add a report or an unidentified control to the desktop board.

The [fixed policy](PROTOCOL.md) keeps the existing first-action-minus-idle tile
gate, then checks source texture, two-direction corner strength and quiet
stability. It reuses the passive selector without new thresholds. For this use,
the source is **baseline frame 0** and the quiet comparison is frame 1. Discovery
coordinates are not reused. At most four candidates and 24 frame checks use the
unchanged first template; failed candidates are not replaced. Later conditions
cannot affect the proposal. The existing explicit lock and edit rules remain.

All **370 workspace test executions**, strict workspace/all-target Clippy and
format checks pass. A controlled local-motion fixture with a large flashing
distraction makes the old selection abstain; v2 recovers the known alternating
+16/-16 normalized-pixel motion. A small quiet brightness change verifies that
quality is measured on the actual first template. The application test now takes
the automatically recommended region through the real shared read/lock commands,
compares saved changed/recovered images and reopens the same history after
restart. Ownership, revision and late-lock refusal checks remain. These are
generated evidence tests, not real gameplay interventions.

Frozen replay uses exactly the three **OwnedPathFixture** records from the prior
baseline study. Each method runs twice using the same file-only executable:

| Baseline frames | Old selection | Stable selection | Recommendation |
| ---: | --- | --- | --- |
| 0 | Incomplete; no files read | Same | None |
| 6 | Four ambiguous template checks | All 647 tiles rejected for weak corners; no template checks | None |
| 2 | Incomplete; no files read | Same | None |

The complete striped baseline is still unsuitable; the earlier quality check
now explains its abstention before attempting a template search. Its new runs
took 0.062 seconds each, versus 0.156–0.172 seconds for the old method, on this
debug build. These timings are observations, not a performance benchmark.

Both methods repeat with exact JSON equality, and all three old outputs equal
their original archive completely. The complete case reads only six baseline
images and five baseline input windows. Incomplete cases read neither. All 163
current/frozen source files, 24 original/frozen inputs and the executable pass
audit. No model call, new capture, game input or target memory operation ran.

Manifest SHA-256:
`624cd5643018d470daa8eea902f28c351fbc3a7e688e87eaa1ddaa21055a854e`.
Executable SHA-256:
`6ac223bd04524f1095930f4acb1c105e7460cc20b9c558bbbe04aa80d06c4fd7`.
Archive: `test-results/stable-baseline-regions-v1`.
See [retained results](results.json).

Texture selection can omit useful low-texture subjects and can favor background
or camera motion. It does not establish identity, physical speed, field meaning
or causality. Real-game intervention evidence remains missing, so overall
progress stays at 35%. Desktop publication remains unavailable after the prior
Windows quarantine.
