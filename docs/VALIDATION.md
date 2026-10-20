# Preview 0.1.0 validation — 2026-10-20

## Automated evidence

`cargo test --workspace`: two discovery regression tests pass, covering robust
inverted scaling with observation noise and the inability of passive matching
to distinguish identical control/display trajectories.

`cargo clippy --workspace --all-targets -- -D warnings`: passes.

`scripts/smoke.py` passes against debug and release executables. It uses real
Windows process memory and a real MCP stdio lifecycle, not mocked tool results.

| Random layout seed | Health matches | Ammunition matches | Unique surviving controls |
|---|---:|---:|---:|
| 8675309 | 4 | 4 | 2 |
| 1001 | 3 | 4 | 2 |
| 20261020 | 3 | 4 | 2 |

Each seed passes edits and verified restoration on both tracks. Attempts to edit
before testing are rejected. Tests reject unauthenticated HTTP and browser-origin
requests; a subsequent state change makes restoration refuse to overwrite newer
bytes. Engine restart retains history and invalidates controls from the old
process. A genuine MCP client negotiates the protocol, lists nine tools, changes
the already-running engine's value, and observes that change through the shared
API. The history identifies the source as an agent.

Local raw release report:
`test-results/smoke-1542715d/report.json` (generated evidence, ignored by Git).

## Desktop evidence

Anode hidden Windows session, 1280×720 at 100% scaling:

- Open practice, Observe changes, Find matches, and Test effects were invoked
  through Windows UI Automation.
- A practice Health value changed from 38 to 120 and restored to 38 in the
  initial build, with visible verification messages.
- Review found low-contrast sidebar headings, an oversized initial window, and
  a restore action below the fold. All three were corrected.
- Release window opens at 1136×659 including borders, entirely within the screen.
  Both value controls and the toolbar's restore action are visible.
- Agent-issued API actions populate the visible desktop values and activity log.
- Release Health changed from 61 to 120 and restored to 61 through UIA.
- The text input initially ignored UIA SetValue. Explicit accessibility action
  handling was added; setting the release input to 120 through UIA then passed.
- Owned test windows and practice processes were closed; the Anode lease released.

## What this does not prove

The practice supplies observation labels, clean numbers, a bounded arena, and a
paused simulation. Its data model uses two scalar channels. This does not prove
screen recognition, appearance or behavior discovery, full-process performance,
real-game compatibility, safe rollback of a running game, arbitrary numeric
representation identification, or control of unobserved side effects.

The independent GameDiscoveryLab research includes difficult failures, notably
overlapping interpretations of floating-point values. Those findings remain
applicable; the successful practice seeds do not erase them.
