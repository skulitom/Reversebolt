# Memory persistence around trial results

The original Tetris trial wrote a byte but observed no score response; it did not
record whether the edited interpretation remained changed around the image.
Reversebolt now collects that missing evidence for new post-edit/restoration
observations. It cannot retrospectively classify the old Tetris trial.

Each result capture is bracketed by full-interpretation reads, with allocation
checks and monotonic intervals. A normalized one-byte patch is not mistaken for
the entire value. At most 200 interpretations of at most eight bytes are read
twice: 3,200 bytes per observation. The existing eight-observation and 128-change
limits remain. Writes, review, refusal, restoration and quarantine rules are
unchanged; these additional reads never issue a write.

The shared journal/MCP records distinguish a matching visible result, a stable
memory edit whose visible number differs, replaced bytes, unavailable memory,
uncertain timing and unreadable visual results. Conditional probe comparisons
become unavailable when the edit did not persist or its comparison evidence is
missing. They cannot accidentally vote for another candidate under that failure.
The desktop card shows a short status, with the explanation under Inspect.

Equal endpoints do not prove continuous persistence: ABA changes, display lag,
allocation reuse and hidden/delayed effects remain possible. A matching result
does not identify gameplay authority; a mismatch does not permanently eliminate
a candidate. Existing observations without witnesses remain legacy evidence.

## Fixed native method

The [protocol](PROTOCOL.md), [configuration](protocol-v1.json), unchanged owned
WinForms fixture, runner and exact changed source snapshots were frozen before
the first case. Protocol SHA-256:
`79e8917eaa3a4fef5b47ff536d52f5ffa5fb7974232ed92c077b0191695e0ef4`.
Product preview: `0.1.0-20261021-090018`.

Three fresh Anode cases use two matching fields at randomized locations. The
evaluator changes the fixture between observations; the product receives no
oracle addresses or source layout. Candidate zero is selected in advance, with
no replacement or rerun. All operations use MCP; no manual screenshot review,
gameplay or desktop inputs are required.

Case zero uses the lower field as the display source; case one uses the upper
field, leaving the selected lower field an unused copy. Case two starts like
case one, then the evaluator sends an ordinary fixture update to 47 after the
confirmed edit. This controlled external overwrite tests the persistence
distinction. It is not a discovered gameplay rule or an atomic-game model.

The [post-run audit](verify.py) independently recomputes the recorded outcomes
from whole-value expected bytes, read intervals and visual evidence, verifies
the frozen hashes, limits and single-application/restoration command counts,
and retains failures. Raw artifacts remain locally under
`test-results/intervention-persistence-v1/`. This is controlled integration
evidence, not independent-game acceptance.

## Results and limits

| Case | Recorded result | Restoration | Full case |
|---|---|---|---|
| 0: selected source | Memory held at 53; visible value 53; `matched` | Bytes and first visual check returned to 50 | Failed: the additional visual reading was unavailable |
| 1: unused copy | Memory held at 53; visible value 50; `did_not_follow` | Bytes and visible value returned to 50 | Passed |
| 2: externally replaced copy | Both fields became 47; `memory_changed`; probe comparisons unavailable | Refused before native entry; newer 47 values retained | Passed |

Case zero's last capture retained expected full bytes at both endpoints but its
selected OCR value was uncertain. The new outcome was `visual_unavailable`, not
a falsely successful response. The strict runner assertion failed and the case
remains failed; it was not retried or silently rescored. Its earlier successful
edit and initial restoration check remain separate observations. This exposes
continued visual-reading unreliability, not an independently usable control.
The evaluator's oracle confirmed the actual field values; it never supplied
observations to the product.

All three jobs are terminal: `j_5826e3fd08ca4e8186c8b8b8161e0a85` exit 1,
`j_11422128842840399ed74b81745c7f59` exit 0 and
`j_e5b98d4a39ea4fa98ee38ac9460ca05b` exit 0. All nine owned process IDs were absent
at cleanup and the Anode lease was released. No independent-game run was added
in this change. [Compact evidence](evidence.json) retains the failed case and
complete memory witnesses without duplicating discovery state or input history.

All **154 workspace tests**, strict Clippy and release build pass. Controlled
tests cover source/copy outcomes, changes during capture including unchanged
patch bytes, missing/partial reads, geometry errors, clock/order mismatches,
legacy evidence, persistence across journal reopening, and no extra native
writes. The packaged smoke check passes with 31 MCP tools; evidence is
`test-results/smoke-56ee6f4f/report.json`. Logs are
`test-results/intervention-{tests,clippy,build,smoke,audit}.log`.

Desktop preview `0.1.0-20261021-090018` is running in the user's session, with all
eight history entries and `qwen3.8:latest` preserved. The executable SHA-256 is
`998eedf02ba177cf9da71cf86810a05cc217d9b5948e9cfce481221bb16f0e43`; the delivery
receipt is `test-results/intervention-desktop-delivery.json`. The new card text
compiled and was delivered; this run did not separately exercise it with UI
automation. Progress remains **27%** against the unchanged full objective.
