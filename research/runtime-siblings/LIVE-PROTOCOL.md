# Sable sibling input measurement, development v1

Continue the sibling-selection/analyzer work at source 617678c, correcting the
batching defect found before native launch: v1's 60 KiB paths exceeded the
reader's separate 32K UTF-16 request limit. V2 also limits path JSON to 30K UTF-16
units, reserving the surrounding request and output-path space. Build and
stage the current default workspace, then freeze the four product binaries, two
analysis commands, this supplement, the runner and the current source before
launch. Back up the existing Sable save folder and record Steam build 10132013.
The existing desktop editor and launcher stay in place. Launch normally; no
security changes, renamed retries or vendor submission.

Use one fresh headless Reversebolt engine in Anode session 3 and installed Sable
with Continue. At most four manually reviewed setup images and zero manually
reviewed measurement images. Automatic attachment uses the existing fifteen
captures and four 200 ms D/A/D/A input probes, with qwen3.8:latest. Wait for the
one discovery/naming job to settle. Do not run a field edit or visual comparison.
This is a development target, not a new held-out game.

Pass the new discovery and its completed metadata receipt to the unchanged
sibling selector. Use all accepted primary/alternative paths, at most 128
eligible fields and the first eight batches in their generated order. Retain any
additional batch/field omissions. Do not select or discard fields by their names,
values, apparent relevance or prior outcomes.

For each batch collect the unchanged fourteen-sample quiet/action sequence.
Actions are D/A/D/A, each held for at most three seconds. Start the input poller
before the hold, wait 100 ms after key-down before starting the native read, and
release in a finally block after the fixed deadline regardless of reader state.
Each quiet sample has a 250 ms settling interval before its read. Keep polling
until at least 75 ms after the reader exits. Poll approximately every 5 ms with
QPC bounds, checking the selected foreground window/PID and all virtual-key high
bits. Abort further actions on focus changes or conflicting input. Do not extend
holds or retry a failed timing interval. No additional measurement screenshots.

Perform reads sequentially after engine jobs settle. Take the engine's existing
per-process lock, require a settled uncertainty fence and record a new fence
before each loader launch. Require matching process identity/helper hash and
confirmed helper absence in completion evidence before clearing the fence or
starting another read. Leave an uncertain loader/target intact for inspection;
never replay it. No gameplay-field writes or freeze operations.

Run the frozen Rust analyzer on every complete batch. Retain rejected batches
and exact errors; do not omit failed observations to make associations pass.
Counts of changing fields are not verified gameplay controls. An association is
only provisional evidence of activity along a particular rooted role. Report
unread paths, changing locations, quiet stability, per-action counts, omitted
coverage and the exact recorded values, including negative results.

After all work is settled, close owned engine/game processes, record save-folder
differences and protection state, and release Anode. Retain the pre-run backup;
do not overwrite the user's saves to hide changes. Review native results before
deciding whether to activate the staged build for the next desktop launch.
