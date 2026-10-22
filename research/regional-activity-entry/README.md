# Small local responses can enter discovery and planning

The trial baseline could already accept small regional motion, but discovery
still discarded it earlier: `play_context::select` required whole-image change
of at least 5 and five times the quiet median. Regional measurement therefore
never ran for a small subject on a mostly static image. The planner also required
four large whole-image responses, leaving the later regional baseline unreachable.

Whole-image activity retains priority and its existing serialized source shape.
When none qualifies, the shared saved-image reader tries at most four prospective
sampled-key sources. Each uses the existing stable-region selector and unchanged
matcher, with four regions/twenty template checks. The total fallback budget is
eighty checks. All attempts, including failed reads and unsupported measurements,
are retained in `regional_activity_search`; the first supported source is selected.

The new source marks itself `regional_candidate_v1`. This marker alone cannot
produce interpreter input or a valid activity context. A source-bound summary
must establish a matched root, quiet displacement at most one normalized pixel,
and displacement at least two pixels in both selected intervals. Source-image
coordinates are converted to the matcher's normalized scale for these checks.
The actual whole-image differences stay unchanged. Geometry, scene continuity,
hashes, sampled-input references and monitor checks remain in force.

Automatic preparation uses the selected source from the shared reader. A regional
context can retain an ambiguous interpretation; it does not change that verdict
or assign a gameplay meaning. Planning rechecks the current source and summary,
then requires the two measured same-key episodes to have bracketed, settled,
consistent sampled holds. It proposes their repetition as four actions and
explicitly records `episode_order: [0,1,0,1]`. There are two observed episodes,
not four. Existing whole-image planning keeps its four-episode rule. Selection
and planning do not send input, edit fields or grant authority.

## Evidence

All **379 workspace test executions**, strict workspace/all-target Clippy and
format checks pass. The new generated-image integration test covers actual
whole-image differences below the old threshold, the shared reader, archived
interpreter input, an ambiguous interpretation, context persistence and action
planning. Missing support, a nonmoving second pair, changed input, static images
and unavailable image reads reject the proposed route. The target backend panics
on native reads or capture. No model is invoked; the interpretation is synthetic.

The first focused test failed the unchanged timing guard because its generated
window claimed a 20,000 µs largest gap when the actual gap was 19,900 µs. Correcting
the fixture's metadata allowed the test to pass; the production guard was not
weakened. This validates software integration, not gameplay meaning or accuracy.

The [frozen replay protocol](PROTOCOL.md) and [create-only runner](run.py) check
two previously exposed game recordings through the actual shared reader:

| Saved recording | Source | Full regional report and summary | Base evidence and observations | New model calls |
| --- | --- | --- | --- | --- |
| Hyperbolica | Exactly preserved | Exactly preserved | Exactly preserved | 0 |
| Manifold Garden | Exactly preserved | Exactly preserved | Exactly preserved | 0 |

Both stay on their existing whole-image path. These are compatibility checks,
not examples of the regional fallback succeeding in a game. All 204 frozen
source/input/protocol/executable files pass the post-run hash audit. Full output
is under `test-results/regional-activity-entry-v1`; [compact results](results.json)
retain each comparison. No game was launched, no new screenshot captured, and
no input or gameplay write was performed by the replay.

Overall progress remains **35%**. Real-game small-subject discovery, causal
controls and withheld generalization remain unverified. Local region motion can
still be background, camera, animation or coincident movement. The desktop
preview remains unavailable after quarantine; executable and review packet stay
local as requested.
