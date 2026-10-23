# Saved-request context experiment

The unchanged native run completed type selection but its first proposal request
timed out. Its server log records a reload into a 32,768-token context and a small
801-token prompt, so prompt overflow is not established as this failure's cause.
The game has exited and no trial wrote a field. Do not alter the native record.

Reconstruct the first group's prompt and schema from the frozen Rust source and
saved selection. The failed HTTP request itself was not archived; explicitly
label the reconstruction. Preserve the request before sending it. Use the same
local model and fixed options, with contexts 32768, 32768, 16384, 16384 in that
declared order. Use the existing 90-second request bound and do not retry a
failed case. Save full responses/errors, elapsed time, model identity and loaded
model state before/after each request. No images, target access, game input,
memory writes or external submission.

This small diagnostic distinguishes request size from an unnecessarily large
context and records cold/warm effects. It does not reproduce game GPU contention,
prove the cause of the native timeout, or validate gameplay controls. Do not
select a result by whether its names are attractive. If small-context requests
complete, any product change must still preserve full prompt evidence and reject
possible truncation rather than silently shrink a large request.
