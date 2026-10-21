# Independent browser-game evaluation v1

Freeze before the first capture: two fresh sessions each of original 2048 and
Jake Gordon's Javascript Tetris, alternating titles. The exact original source
archives and file hashes are in the local upstream manifest. Both projects are
MIT licensed: https://github.com/gabrielecirulli/2048 and
https://github.com/jakesgordon/javascript-tetris . Source files remain unchanged.
Downloaded sources/profiles/raw captures remain under ignored `test-results/`.

Run the published Reversebolt binary unchanged. Select the owned Chromium game
window through the real MCP inventory and use the normal automatic attachment
loop (16 full captures, comparisons from 12, 16 MiB memory budget). Use the
already installed local vision model selected in the desktop. No product
process-selection adjustment, game profile, field address, source layout, label,
oracle reading, DOM bounds or manually supplied visual observation is allowed.
Product hashes and the model digest are pinned in `protocol-v1.json`.

The evaluator hosts original files on loopback and blocks all other page
requests. Each run uses a new owned headed Chromium profile inside Anode.
Native key pulses follow fixed schedules, independent of oracle values. The
driver guards the exact foreground HWND/PID and child session, releases every
key and stops on focus loss. Read-only document event listeners record delivered
browser keys; these show browser receipt, not proof of game-accepted meaning.

A separate evaluation reader samples DOM-visible counters and (for Tetris)
piece position, direction, color, shape, actual versus displayed score. These
readings are never submitted to Reversebolt. Original source bytes are intact,
but the added event listeners and transient CDP results introduce observer
overhead and may allocate copies. A matching memory value is therefore not proof
of authoritative game state. No memory edits are performed in this evaluation.

Numeric scoring is fixed before results: score/best in 2048 and score/rows in
Tetris. DOM rectangles are mapped to the captured client area using device pixel
ratio and the generic difference between client and viewport dimensions. The
scorer requires an oracle sample on each side of the image capture interval,
no gap above 200 ms, identical region/value throughout, and positive compatible
geometry. Any violation is unscorable, never a correct reading. Product number
tracks are selected spatially (their centers inside the oracle rectangle), not
by searching for the expected number. A single distinct readable value is
correct or wrong; no readable value is abstention; different readable values
are conflicting. Stable zero counters are reported separately from changing
counters. Retain all captures and all attempts, including harness failures.

Report input receipt, capture count/timing/coverage, score precision/abstention,
memory matches, action patterns, conditional checks, and automatically produced
visual/temporal observations. Record nonnumeric oracle diversity without
counting the oracle as visual understanding. Compare selected window PID with
Chromium's own browser/renderer inventory; this is evaluation evidence only.

This is a first exposure evaluation of independently authored browser games,
not a withheld final acceptance suite and not evidence of 3D-game compatibility.
It cannot satisfy the full goal's hidden-behavior, causal editing, recovery or
independent-game release requirements. Keep progress at 25% unless evidence
justifies a specific capability change. A failed result changes the next action;
it must not be repaired by hardcoding a game answer.
