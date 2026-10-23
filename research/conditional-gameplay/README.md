# Conditional gameplay baseline

The frozen [protocol](PROTOCOL.md) tested whether the current Sable scene supplied
usable activity for investigating the two historical ground-incline and swimming
questions. It stopped before test-design inference or any field edit. The questions
remain unverified; neither effect was tested.

Artifacts are local in `test-results/conditional-gameplay-v1/`. The published
engine/MCP binaries, three offline examples, sources, model identity and game
storage were retained before launch. The analysis driver was separately recorded
before inference, after capture. No measurement image was manually viewed.

One engine observation job collected fifteen images with its own keyboard samples.
Four accepted D/A/D/A holds lasted 1000.4001–1000.8251 ms including the surrounding
input-call envelope. All four episodes were separated by quiet intervals. Their
image differences were 38.0001, 24.0362, 26.4641 and 26.6093, compared with a quiet
median of 0.1161. These are image contrasts, not actor displacement or key meanings.

The existing activity selector chose two D episodes. One local image-model call
retained an uncertain interpretation with `consistent_visible_change: false`;
neither image pair had matched feature cells. No reliable physical movement,
slope condition or swimming transition was established. The activity receipt
keeps the interpretation separate from measured image contrast.

The unchanged planner rejected the recorded sequence. Keyboard sampling estimated
holds of 1004, 1003, 988 and 1007 ms. Three exceeded its existing 100–1000 ms range,
so it could not retain four eligible responses. The protocol had unfortunately
placed its nominal duration exactly at that upper bound. The original samples,
failure and thresholds remain unchanged. A future protocol should leave timing
margin; this result does not justify clamping measurements or claiming success.

No test-design request ran, no live path was reacquired, no runtime helper loaded
and no gameplay field was written. `activity-question-plan` is a read-only example
that combines historical questions with separately validated new activity, while
explicitly withholding live binding and execution authority. It was built and
checked with strict Clippy. The analysis driver now writes a terminal summary on
a failed stage; the original frozen driver and its failure receipt are preserved.

The independent audit verifies frozen hashes, frame identities, all fourteen
image differences, input timing, terminal receipts and the retained storage
backup. It passes. All fifteen capture-clock checks have no error. Only player
logs and `SaveData/SaveManager` changed; the main save, save image, settings and
cloud metadata retain their pre-launch hashes. No storage was restored or removed.

The owned game and headless editor exited, the Anode lease was released, and the
user's desktop editor remained open and responsive. No new desktop build was
published for this research-only addition. Full-goal progress stays **36/100**.

Run the offline audit without input, inference or game access:

```powershell
python research/conditional-gameplay/audit.py
```
