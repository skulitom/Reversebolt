# Explicit focus before one controller activation

The v1 setup ended without gameplay: three inspected images showed startup then
the same menu before/after one controller A tap. UI Automation offered no actions.
Anode reported its GameInput helper as the foreground/input blocker. The owned
game exited normally; no runtime helper or gameplay write ran. The isolated pad
was detached and the lease released. Preserve this failed setup.

One distinct setup check is available: Anode's `seat_window` focus operation,
which v1 did not call. Launch a fresh owned process and ask that tool to focus its
observed window. If focus is not confirmed, stop this game setup without further
input. If confirmed, inspect the menu once and try a single appropriate controller
activation. Inspect its outcome once. No administrative repair or service changes.
All original save-preservation, runtime-binary and gameplay-effect limitations
remain. This is a new setup version, not a replacement for the v1 outcome.
