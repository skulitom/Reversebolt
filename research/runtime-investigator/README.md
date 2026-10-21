# Fresh-model investigation on Manifold Garden

The unchanged product discovered metadata and retained twelve grounded questions,
but this experiment **abstained before a comparison**. Every proposed alternative
was another path to the primary's declared field. None satisfied the precommitted
requirement for two distinct floating-point fields of one type. No gameplay test
input, measurement capture, edit, Freeze or Undo was performed.

This is a failed handoff from discovery to the evaluator's comparison policy,
not a successful real-game verification. The artifact audit passes because the
failure, evidence and cleanup are intact. Overall goal progress remains **34%**.

## Method and result

`PROTOCOL.md`, `run.py`, product sources and published binaries were hashed before
launch. The evaluator used Manifold Garden build 9427366 and existing test slot 3
in Anode session 3. Four screenshots were reviewed to navigate the title screen,
main menu, save selector and loaded scene. The user's slots 1 and 2 were preserved.
The launch reply was unavailable after a tool-output limit; the existing game PID
and session were recovered without repeating the launch.

The actual stdio MCP client attached to fresh gameplay and ran the normal automatic
runtime discovery and local metadata review. No previous proposal, remembered
field name, game source or offset was supplied. The installed model was
`qwen3.8:latest`, digest
`22130167c4c20e20c7b71454612966ca8e8171e9b3cc8ab6ce8aa6cbfec79643`,
served by Ollama 0.40.7. The two inference stages took 51.107 and 79.227 seconds.

The model selected eight observed types. The bounded evidence table contained
256 path samples and 195 distinct declared fields. Its twelve accepted questions
covered seven fields in `CarryRigidbody` and five in `LineDisplay`. Their
alternatives were all additional paths to the same declared fields, with no
fresh resolution to establish whether those paths alias or identify different
objects. The selected `RigidbodyController` contributed 85 distinct fields to
the evidence table but no accepted question. This is observed output coverage;
it does not establish why the model omitted those fields.

The protocol's first-eligible-pair selector therefore stopped. Its assertions
produce exit code 1 and an explicit `No eligible competing hypothesis; policy
abstained` record. The exact failed attempt remains in
`test-results/runtime-investigator-v1`; it was not retried or replaced with a
known movement field. There are zero response tests, zero measurement images,
zero gameplay holds and zero writes. Input-delivery and response-capture behavior
remain untested by this attempt.

The sole discovery helper completed and unloaded. It recorded 110 managed-world
pauses, with maximum stop-to-resume time 5.206 ms and zero helper allocations or
deallocations during those pauses. These are helper-reported timings, not an
independent game-thread balance oracle. The headless engine and MCP processes
were terminated by the evaluator after saving final state; their exit code 1
is recorded rather than described as a graceful service shutdown. The game was
closed normally through its observed window. All owned processes exited and
the Anode lease was released; the user's desktop preview was left running.

The before/after hashes match for both user saves and their screenshots. Only
test slot 3, its screenshot and the two game logs changed. No save rollback was
performed. All original source and binary hashes still match the frozen manifest.

## What this establishes

The metadata validator checks citations and name grounding. It does not certify
that a proposed question concerns the player, establish a competing property,
or match that question to an input or observable consequence. In particular,
an extra path is not automatically an independent alternative property. The
comparison policy correctly withheld edits, but cannot yet turn these questions
into useful autonomous investigations.

The planned D/A walking inputs came from an earlier exploratory test; they were
not learned in this run and would not answer many of the returned carry/display
questions. Future planning needs to connect observed actions and consequences
to a question, distinguish path/instance ambiguity from competing fields, and
explicitly retain unsupported plans as abstentions. Simply increasing the number
of returned names or removing the pair requirement would not verify that link.

This is an already inspected game and scene, so neither a success nor this
failure constitutes held-out generalization. The integrated comparison remains
verified in the owned fixture, with an earlier manually planned real-game
comparison retained separately. No new gameplay control was identified here.

## Reproduction and audit

Use a **new** output directory for any new run. `freeze.py OUTPUT` copies the
current published binaries and backs up LocalLow game storage before launch.
Load the evaluator save in Anode, then run `run.py OUTPUT GAME_PID` inside that
session. The runner refuses a previously started data directory and records
requests, responses, input receipts and unresolved failures. Its Anode session
check is evaluator setup, not a game-specific product rule.

After confirmed cleanup, retain `cleanup.json` and run `audit.py OUTPUT`.
The audit was authored after the protocol freeze and checks the original hashes,
retained review, absence of unresolved operations, helper receipts, process exit
and user-save integrity. No effect threshold was changed. Its initial assumption
that local and external-agent naming receipts shared the same wrapper was corrected
against the actual saved local receipt; no game action was repeated. The unused
comparison-outcome audit branches are not claimed exercised by this abstention.

`results.json` contains the compact audited outcome and coverage diagnosis. Raw
game metadata, saves and captures remain local under the ignored test-results
directory. No product source changed, so no replacement desktop build or repeat
of the previously passing 265-test suite was needed for this research-only change.
