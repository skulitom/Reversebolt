# Measuring nearby runtime state

The shared Rust core can now select sibling fields on an exact rooted object
path and analyze repeated associations with observed input. This adds a way to
investigate active runtime objects without relying entirely on names or visible
HUD changes. It is an internal research API and two local commands; it does not
add unnamed values to the desktop or promote fields into editable controls.

Progress remains **36%**. We have verified selection coverage, analyzer rules and
native clock bounds. We have **not yet measured input-associated sibling changes
in a game**. The analyzer's positive cases are synthetic.

## Results

| Retained native discovery | Rooted roles | Selected fields | Omitted fields | Batches |
| --- | ---: | ---: | ---: | --- |
| Sable | 10 | 117 | 0 | 32, 32, 32, 21 |
| Viewfinder | 5 | 128 | 33 | 32, 32, 32, 32 |

The seeds are all accepted primary and alternative paths from the existing
metadata receipts. Within each rooted role, selection follows report traversal
order and alternates between roles until the 128-field budget is exhausted.
Field names and scalar values do not rank siblings. Eligibility still excludes
unsupported scalars, noncanonical booleans and nonfinite discovery floats. The
initial seed hypotheses remain influenced by the earlier metadata model.

The reader now records QPC bounds around every refresh pause, with both clock
calls outside the managed pause. An unchanged owned ABI fixture completed one
discovery and three four-path refreshes. All four helpers unloaded, eleven
pauses balanced, and there were zero helper Rust heap operations while paused.
Every sampling interval fell inside the independently recorded loader interval
at the same 10 MHz clock frequency. The longest refresh stop-to-resume interval
was 11 microseconds **in this fixture**, not a real-game performance claim.

Five new Rust tests cover role/budget accounting, state pulses versus persistent
steps, idle drift, constants, missing reads, changed or invalid addresses,
contradictory scalar bytes, wrong process scope, input/clock gaps and modified
plans. All **468 workspace tests**, strict all-target Clippy, formatting and the
post-run artifact audit pass.

No game was launched, no input sent, no screenshot viewed and no model invoked
in this experiment. The owned fixture exited normally. The main desktop editor
remained responsive, and the tested `0.1.0-20261023-012028` preview remains the
next-launch version. Security settings were unchanged; current protections
remain enabled and no new Reversebolt detection was observed. Nothing was
uploaded.

## Interpretation and next measurement

Fourteen samples bracket four actions with quiet samples before and after.
An association requires complete reads and stable addresses, plus either a
repeatable temporary change or a repeatable retained step. Exact byte equality
is conservative: it will miss jitter, delayed changes and responses that occur
between samples. A successful association would still not prove gameplay
meaning, causality or actor identity. The next native experiment must measure
these intervals with fresh process-bound paths and actual foreground/key polls,
retaining failures rather than lengthening input after seeing results.

The new APIs are `runtime_neighbors::plan` and `runtime_response::analyze`.
`reversebolt-runtime-neighbors` takes a discovery report, its naming receipt and
a new output directory. `reversebolt-runtime-response` takes a plan, batch index,
fourteen retained observations and a new output file. Neither command contacts a
target or requests a model; native collection remains separate.

See the [predeclared protocol](PROTOCOL.md), [selection/freeze runner](offline.py),
[native clock check](native.py), [post-run audit](audit.py) and
[compact results](results.json). Exact 221-source snapshots, binaries, inputs,
stdout/stderr and results are retained locally under
`test-results/runtime-siblings-v1`. The snapshot begins at commit `d3f4de9` plus
the recorded implementation changes; hashes identify the tested sources.
