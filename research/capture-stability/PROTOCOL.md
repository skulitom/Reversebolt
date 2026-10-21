# Fixed-window native capture sampling — frozen v1

Freeze this method, fixtures, runners, relevant product sources and packaged
binary hashes before the following three cases. Run each once in Anode, preserve
every failure, and never substitute a better-looking capture. No manual image
inspection, game control, local model calls or game-specific discovery inputs.

Cases 0 and 1 copy the learned-references fixture and runner byte for byte. These
are new cases against a changed product, not replacements for that earlier
protocol's failed case. Both attach normally and automatically capture the same
sixteen values, learn a bounded reference, relocate the allocation, then recheck
four fresh values. Case 0 edits the moved field from 50 to 53, observes the effect
and restores 50. Case 1 moves it again after preparation and requires refusal
without changing either the old or current field. Keep the original strict
assertions, including every expected OCR reading. No manual capture, correlate
or freeze calls in these two cases; the evaluator's oracle is never sent to the
product. The old protocol and its failed run remain untouched.

Case 2 attaches a separate, persistently blank dark WinForms window with automatic
observation disabled. Four explicit full captures test capture/OCR plumbing.
Require a WGC sampling receipt on each: policy last_completed_in_fixed_window_v1,
80 ms after the first completed frame, at most eight frames, selected index last,
monotonic completion times strictly before the window deadline. Require the
selected image to match its recorded RGBA digest, low detail (<0.001), a warning,
no OCR words or numeric tracks, and no edits. This is a genuine blank retained as
blank; there is no retry, cached replacement, inferred value or blank rejection.

For all available captures in all cases, audit sampling receipts and saved-image
hashes after execution. Report first versus selected low-detail counts, source
timestamps, frame counts and maximum completion span. Image content never
influences selection: use the last frame completed before a fixed deadline or
the eighth completed frame. Intermediate images are not persisted. Measurements
and warnings are diagnostic only. A static source can legitimately yield one
frame. Raw compositor timestamps do not establish simultaneous game state or
remove the existing ABA, compositor-lag and non-atomic capture limitations.

These three cases establish bounded native workflow behavior, not statistical
reliability, cross-game generalization, semantic identification, gameplay control,
restart recovery or proof that every transient blank is avoided. Do not grant
independent-game progress from these owned fixtures.
