# General visual objects: first evaluation

Freeze the candidate binary, model digest, prompts and this method before
running the original Javascript Tetris through the ordinary MCP attach loop.
Use the pinned unchanged upstream archive from the independent-game baseline,
a fresh owned Chromium profile in Anode, 16 automatic captures and the standard
two local-model passes. Native pulses are fixed, 80 ms down every 1500 ms,
independent of game state. No manual screenshots or adaptive gameplay.

Reversebolt receives its normal window image, sampled input and bounded memory.
The test reader separately records the original game's current and upcoming
piece cells, colors, canvas geometry and pending redraw flags every 50 ms.
These source-derived details are for evaluation only; none enters the product
or its model requests. Existing source bytes remain unchanged. Instrumentation
can allocate copies, so this run cannot establish memory authority.

Score only object observations actually published by the product, in their
original source-frame slots. Do not relabel earlier observations as current.
The two predeclared targets are the currently falling piece and the upcoming
piece. Compute their visible cell-union rectangles using the original renderer's
recorded cell size and canvas-to-client transform. Require consistent oracle
geometry/cells/color throughout samples bracketing image acquisition, gaps and
request durations no greater than 200 ms, no pending redraw, and compatible
client/image geometry. In addition, all four cell-center pixels must match the
recorded fill color within two levels per channel. Otherwise mark the target
unscorable. This does not prove presentation timing or persistent identity.

At IoU thresholds 0.3 and 0.5, match published Object boxes to scorable targets
one-to-one, largest overlap first. Record misses, unscorable targets and unmatched
objects separately. This target list is not exhaustive for piles, frames or
other possible objects: do not call unmatched objects false positives or report
whole-scene precision. Report any Character/Held object classifications separately;
this game's original rendering contains neither characters nor held equipment.
Descriptions, physical rules and editability are not validated by a box match.

Run the current observer on the first annotated pair of dev_q_demo, dev_c_idle
and dev_c_move from the existing visual-challenge corpus. Use the existing
scorer to retain actor/held-component scores; newly proposed Object entities
remain unscored by that narrower annotation set. Retain every rejected response
and all attempts. Crop review is same-model checking, not independent validation.

The broadened object taxonomy is provisional. Neither an additional card nor a
successful box match establishes a gameplay control. Keep the full goal active
and the weighted progress denominator unchanged. These are exposed targets and
selected-object checks, not withheld release acceptance.
