# Repeated-contrast matcher: fresh follow-up

Use the exact V1 process-selection, source, input, observation, freezing and
scoring method in PROTOCOL.md. Run one fresh 2048 session, then one fresh Tetris
session. No retries, memory writes, adaptive inputs, game-specific product logic
or manual image selection. Preserve both failures and successes.

The sole product change is the generic numeric matcher's evidence requirement:
both temporal halves, and the observations each mapping actually explains,
must contain two low and two high readings separated by more than 2.2. A rare
one-off event may now abstain until further observations. Freeze the candidate
binary and changed source hashes before capture.

run-v2.mjs differs from run.mjs only in its protocol filename. The existing
score.py retains its V1 timing/geometry rules, which are identical here. Compare
fresh results descriptively: randomness and elapsed gameplay make these unpaired
runs, not a controlled estimate of an improvement rate. Replay of the retained V1
FPS trajectory can establish its observation-level ineligibility, but cannot
reconstruct its discarded training bytes. The Rust regression uses synthetic
bytes with that exact trajectory and is reported separately.

Keep overall progress at 26%. Useful hypotheses, even with fresh agreement,
do not establish controlling state or verified edits in either game.
