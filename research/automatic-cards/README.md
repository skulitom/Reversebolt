# Automatic cards from reviewed discovery

Completed local and agent reviews now materialize exact accepted primary paths
from the full retained discovery graph. Findings outside the initial 64-value
preview no longer require a manual search and Show card action. The shared
runtime list receives the original scalar representation and discovery timestamp;
the desktop displays the reviewed name/explanation as a possible effect. No new
read, capture, model call or write is performed by card publication.

The list stays bounded to 64. Explicit Add controls selections, refreshed cards
(including unavailable readings), and existing edit/freeze/comparison cards take
priority. Unused preview entries can be displaced. Protected capacity shortages
are returned as exact omitted path IDs. A new card list advances its revision and
generation; repeated identical inclusion preserves both. Agent receipt replay
returns its saved publication without overwriting current cards or reviews.
Local publication is separately archived before the staged view is installed.

This preserves shared card records, not a permanent semantic identity. The UI
still requires a current explanatory review to display a runtime card; a later
review can hide a retained card. Unfreeze all remains reachable. Names, bounds,
controlling state and effects are not verified by this change. Discovery values
can be old; direct edits still perform their existing fresh checks.

The frozen [protocol](PROTOCOL.md) replays two unchanged independent-game
discoveries with the shared Rust implementation:

| Retained discovery | Accepted primary paths | Initially in preview | Added automatically | Omitted |
| --- | ---: | ---: | ---: | ---: |
| Hyperbolica | 2 | 0 | 2 | 0 |
| Manifold Garden | 0 | 0 | 0 | 0 |

Hyperbolica's prior hypotheses were walking speed and shake strength. Both keep
their original tentative explanations; neither effect was tested here. The
Garden empty result remains empty. Both second inclusions leave the complete
runtime view unchanged. Original saved inputs and reports were not modified.

All 358 executions in `cargo test --workspace` passed, including new core coverage
for paths outside the preview, inherited source bindings, preserved failed/fresh
readings and selections, full capacity, invalid evidence and revision exhaustion.
The existing app submission/archive/restart test now verifies automatic inclusion
and exact retry without any target I/O. Strict workspace/all-target Clippy and
formatting pass. These checks establish publication mechanics, not generalization.

Archive: `test-results/automatic-cards-v1`. All 155 frozen source files, four input
files and the example binary pass the post-run audit. Manifest SHA-256:
`9f004e96af8e26d2e366826233c61926372afd2157716035f2f40fe90dc82136`.
See [summary results](results.json). Desktop publication is still unavailable
after the prior Windows quarantine; no replacement was built or launched.
