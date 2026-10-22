# Memory/input associations

Implemented 2026-10-22. Full-goal progress remains **35%**. Memory-only proposals can
now carry descriptive input evidence without requiring an onscreen number. This
is a step toward deciding what to investigate, not a verified gameplay meaning.

## Collection

Image input windows end at the image-capture operation. Memory samples are read
afterward, so those windows cannot be silently reused for memory comparisons.
Full/Visual captures now query the existing job-owned sampler buffer after memory
reads and before optional recognition. The query begins at the preceding memory
sample's first read and ends at the current sample's last read. The first sample
begins at its own first read. Play-only captures add no memory context; batch
transitions clear memory history. Each comparison uses its individual chunk's
receipt, not the broad scan's end as a substitute for read timing.

The live capture summary stores `memory_input_context`; the local raw export keeps
the optional context in each `Sample.input_context`. Existing image windows and
their clocks are unchanged. No new sampler, memory read, image, key event, OCR or
model call is added. The context uses the existing limit of 256 polls per window;
metadata stays inside the existing export/manifest budgets. Absence, sampling
omissions and gaps remain visible.

## Analysis and agent access

The normal final follow-up review derives `memory_input_associations_v1` alongside
its pinned trajectory checks. For each candidate it retains a class for every
adjacent pair of memory captures, plus increased/decreased/unchanged counts for
no-key-detected intervals and at most eight distinct keys. Additional keys are
listed as omitted from aggregates; their interval classes remain. Selection uses
neither game names, field layouts nor image labels. No new UI cards or report UI
are created. `read_memory_followup` exposes these results in its existing pages.

Input coverage must include both uncertain read operations, with matching clocks
and sampler generations, selected-window focus, at least three polls, no omitted
polls and no sampling gap/poll duration above 100 ms. Read operations must be ordered
and within a four-second span. One key needs at least two pressed samples; several
keys are ambiguous. Gaps are recomputed from polls rather than trusting a summary.
Missing/invalid memory endpoints remain unknown and never become a zero or hold.

Unlike the earlier image-response classifier, this policy can describe a held key
during memory reads: it describes exposure across the entire uncertain interval,
not an effect after a press. A key sample does not prove game delivery or a
continuous hold. These comparisons are derived after capture, not prospective
input hypotheses. Repeated intervals are not independent trials. Quiet drift,
other input, asynchronous updates, address reuse and edits can explain the data.
Naming and causal authority require further evidence.

The result keeps the existing 256 KiB file budget. Legacy archives with no memory
input windows produce no new input association; image context is never substituted.
The frozen previous follow-up is replayed to verify this compatibility.

## Verification

Generated target/sampler fixtures exercise real shared preparation, capture,
automatic final review, export, paged read and offline replay. Twelve source and
twelve fresh captures make exactly 24 target-read attempts and 48 buffer-window
queries (one image and one memory window per capture). Preparation/analysis adds
no queries. A missing memory sample excludes both adjacent pairs.

The fixture's float increases with a sampled key **and** with no key detected:
both counts are retained. This counterexample prevents a positive key association
from being presented as a specific gameplay response. Tests also cover wrong
clocks, restarts, focus loss, mixed/single key samples, dropped polls, falsified
gap summaries, malformed chronology and an image-only window ending before memory.
Large reports with 96 members, twenty samples, unknown classes and omitted key
aggregates stay within the unchanged storage cap. Saved results reproduce exactly
offline, without target or model access.

See `results.json` for exact counts, hashes and logs. These are generated tests,
not independent-game performance. The Windows security block still prevents new
native validation; no quarantined build was relaunched or replaced. Everything
stays local, and progress remains 35% until the missing capabilities are verified.
