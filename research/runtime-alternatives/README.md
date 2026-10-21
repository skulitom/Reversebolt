# Distinguishing two movement hypotheses by their effects

The saved model proposal contained both `moveSpeed` and `walkSpeed`. A new game
process was opened in evaluator save slot 3. The unchanged discovery pipeline
found fresh runtime state. The evaluator selected the first proposal with two or
three floating-point fields of one declaring type, matched exact runtime
type/layout and field descriptors, and checked all retained paths for each field
before choosing a path. Both fields had two paths to the same observed address
and a value of 8.2. No saved address, source layout, offset or game profile was used.

Each candidate received the same frozen baseline/change/recovery sequence through
the shared MCP engine and Anode. The only requested intervention was a guarded
change to zero, followed by the original trial's guarded restoration. Each
condition used a stationary image pair and four alternating 200 ms lateral input
pulses. Thirty-six measurement images were processed automatically; seven setup
screenshots were reviewed, including two loading transitions. No measurement
image was manually interpreted.

| Candidate | Baseline median | Changed median | Recovery median | Interpretation within this test |
| --- | ---: | ---: | ---: | --- |
| `moveSpeed` | 6.309 | 6.307 | 6.726 | The game returned the field to 8.2; the requested change did not suppress the measured response. |
| `walkSpeed` | 6.724 | 0.259 | 6.498 | Zero remained observed after input, response fell to stationary variation, and response returned after the guarded restore. |

Numbers are mean absolute grayscale image differences, summarized by the median
of four pulses. Stationary differences were about 0.256–0.259. The second field
passes the previously frozen suppression/recovery criteria; the first does not.
This distinguishes two plausible alternatives in the tested scene. It does not
prove that the first field is display-only, measure world-space speed, establish
persistent actor identity, or verify either field in other contexts.

Candidate selection is now automated from a retained model proposal and fresh
metadata, but the proposal was developed on inspected data, and the walking input
and metric came from the previous exploratory test. This is not a held-out
generalization result or an end-to-end in-app investigator. The desktop still
needs automatic hypothesis testing, evidence-backed promotion and recovery
across more contexts. No model labels from this research were published.

The game run used product `d4e2b0d`. Its 13 helper operations completed and
unloaded, with 123 recorded pauses, zero helper heap operations while paused, and
a maximum stop-to-resume duration of 5.185 ms. Three writes were confirmed (two
interventions and one restoration). The other restore was refused because the
original bytes were already present at the same guarded path. Both fields were
freshly observed at their original values before closing the game. Existing save
slots 1/2 and thumbnails remained byte-identical; evaluator slot 3 and logs were
updated and retained. Owned game/headless processes exited; Anode was released.

`PROTOCOL.md`, the model proposal, product binaries and metric were frozen before
launch. The client task-envelope correction was also made before launch; the
manifest retained original and corrected hashes under differing path separators.
The audit verifies the exact correction and both hashes. `capture.js` was recorded
after menu setup/discovery but before the first measurement input or gameplay
write; this timing departure is retained explicitly. The capture runner checked
foreground identity before every input/image; full window-list responses were not
saved in its action files. Raw results and the audit are in
`test-results/runtime-alternatives-v1`.

## Product fix prompted by the real-game result

Undo previously left a trial recorded as written when the game had independently
returned to the original value. The shared engine now recognizes the stronger
`already_original` outcome from the native refusal receipt. It requires the exact
inverse of the confirmed write, matching type/value/bytes, field address, entire
reference chain, process identity and patch identity, with a value-changed guard,
no attempted write and zero transferred bytes. The phase becomes restored and the
card uses the observed original value. It states that no restore write occurred;
downstream gameplay consequences can remain.

Other drift, replaced references, incomplete/contradictory evidence and uncertain
outcomes retain their existing behavior. Old refused records remain readable and
are not silently upgraded. Duplicate requests return the saved result; restart
preserves it without another native operation. The desktop and MCP share this
behavior, and the MCP description documents the new outcome.

Product `21fb8ca` passed 244 workspace tests, strict Clippy, formatting and release
builds. The frozen native fixture then verified numeric/boolean already-original
recovery, exact original byte reads, no restore writes, refusal of unrelated drift
and replaced references, replay and restart. Four trials used 13 helpers and 28
pauses, with zero helper heap operations during pauses and a 64 µs maximum. All
owned children exited. The unchanged 45-tool/practice smoke passed. Evidence is
in `test-results/runtime-alternatives-recovery-v1` and
`test-results/smoke-2f57b1c1`.

Preview `0.1.0-20261021-200143` is running on the user's desktop with matching
frozen binaries. All 44 history entries, the model, trials, changes and freeze
records were preserved. The full goal remains active. The next product step is
to make bounded hypothesis/contrast evidence a first-class shared investigation,
so useful explanations can be earned by measured effects in the ordinary
discovery loop rather than remaining an external research workflow.
