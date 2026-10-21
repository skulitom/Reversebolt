# Manifold Garden: first reversible real-game movement contrast

The unchanged product at `348d5c3` automatically reached player-controller state
in actual gameplay. Manual metadata search selected a walking-speed hypothesis;
the product's guarded direct edit changed 8.2 to 0, then restored the original
bytes. This is exploratory selection, not automatic semantic identification or
held-out generalization.

The protocol was saved before any movement input or gameplay-field write.
Twelve 200 ms lateral inputs were measured using 18 automatically processed
screenshots, with no per-frame model review. Median image differences were
6.419 at baseline, 0.258 with zero walking speed, and 6.399 after restoration.
Stationary differences stayed around 0.258. All frozen contrast criteria passed.
This supports a reversible effect on the rendered response to walking input;
it does not measure world-space velocity or establish scope across scene loads.

Four helpers completed and unloaded. Their 114 pauses, including discarded
warmups, had no helper allocations/deallocations while paused; maximum pause was
5.178 ms. Four setup screenshots were reviewed. Existing save slots 1 and 2 and
their thumbnails are byte-identical to backup. Empty slot 3 was used; its new
save/thumbnail, settings and log were retained. Owned game and headless editor
processes exited, and Anode was released. Anode's authorized restart resolved the
previous input-helper issue without service changes or moving desktop Steam.

See [protocol](PROTOCOL.md), [frozen movement test](movement-protocol.json), and
[results](results.json). Raw local evidence lives under
`test-results/manifold-runtime-v1`, including frozen binaries, target/catalogue,
guarded write/restoration receipts, frames, measurements and storage backup.
Run `python research/manifold-runtime/measure.py test-results/manifold-runtime-v1`
to recompute the screenshot contrast. No game-specific logic was added to the
product. Automatic useful naming/ranking, broader effects, and revalidation
remain unresolved; overall progress remains 31% pending the broader capability
gate.
