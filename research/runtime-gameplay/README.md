# Gameplay state is missing from the menu observation

This exploratory study found concrete movement hypotheses in retained runtime
metadata, but could not enter gameplay through Anode. No gameplay control was
edited or verified. Weighted progress remains **31%**.

The previous frozen Blue Prince report includes a first-person controller type
with four-byte floating-point fields named `m_WalkSpeed`, `m_RunSpeed`,
`m_JumpSpeed` and `m_GravityMultiplier`. Its static `Active` reference was read
successfully as null. The bounded graph contains no exact-type nodes or scalar
reads for those selected fields. This is a stronger explanation for their
absence from the cards than assuming the runtime adapter cannot inspect them.
It remains a hypothesis about lifecycle: the report does not prove that no such
object existed elsewhere in the process, or that a future Active target will
have this exact type.

The next useful observation is a fresh catalogue during actual gameplay, then
targeted reads across repeatable behaviour. Do that before broadening heap
traversal or inventing game-specific layouts. Field names do not establish
ownership, actual effect or an editable range. Runtime writes/restoration remain
unimplemented; a successful read would still require a separate causal test.

## Retained metadata analysis

[inspect_metadata.py](inspect_metadata.py) accepts a saved report, an ordinary
text query and a new output file. It contains no game/type/field-name filters.
It searches qualified type, image and field names, and reports exact-type node
occurrences, successful scalar-read occurrences and roots declared on each type.
It never reads live memory or game source. Node occurrences are not distinct
objects; zero observations are bounded-coverage statements. A root declared on a
type need not refer to an instance of that type.

Eight exploratory queries used the previous `runtime-context-v1` menu report:
speed (49 matches), movement (4), jump (9), player (265, output limited to 100),
health/damage/ammo/stamina (zero each). These counts describe only the metadata
retained in that report. They do not establish the presence or absence of those
gameplay mechanics. The controller type was noticed during exploratory inspection
before these queries; this is not held-out selection or automatic semantic
discovery. The post-run audit checks selected facts directly against raw fields,
root readings and graph observations.

## Two bounded setup attempts

[v1 protocol](PROTOCOL.md), [v2 protocol](PROTOCOL-v2.md),
[audit](audit_postrun.py), [results](results.json).
Raw evidence and a pre-input storage backup remain under
`test-results/runtime-gameplay-v1` and `runtime-gameplay-v2`.

The first owned process, PID 39652 in session 2, reached the main menu. Anode
reported its foreground GameInput helper (PID 30848) as blocking normal focus and
input. An isolated Xbox controller was attached, with isolation confirmed from
the parent desktop. One A pulse did not visibly advance the menu. Three manually
inspected images cover startup and the menu before/after that pulse. Accessible
inspection exposed only the game pane, with no actionable controls. No input
delivery to the game is claimed. The controller was detached and the game exited
normally before releasing the lease.

The second owned process, PID 90744, tested one different mechanism: Anode's
explicit window-focus operation. The tool refused it with the same GameInput
blocker. No controller action or screenshot followed. The game remained responsive
and exited normally; the lease was released. Neither attempt loaded a runtime
helper, wrote gameplay memory, selected a save slot, repaired Windows services or
moved Steam. Both gameplay blueprint files, Settings.dat and the cloud metadata
file match the pre-input backup. RewiredSaveData.dat changed during launch/controller
connection; no all-files-unchanged or full game rollback claim is made.

The user was offered the option to start gameplay on their own desktop and connect
Reversebolt, or leave Windows services alone and continue independent testing.
No response is assumed. The shared engine remains usable while the separate Anode
gameplay-input path is unavailable. This is a blocker for this experiment, not a
claim that the whole project has no available work.

The application binaries were unchanged. Desktop preview
`0.1.0-20261021-163051` remains the current tested build. No new gameplay control,
automatic semantic label or generalization credit was earned.
