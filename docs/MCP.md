# MCP and the shared desktop engine

Start Reversebolt first. Build the workspace, then configure an MCP-capable
client to launch:

```json
{
  "mcpServers": {
    "reversebolt": {
      "command": "C:\\DEV\\Reversebolt\\target\\debug\\reversebolt-mcp.exe"
    }
  }
}
```

This is an example configuration, not an automatic modification of any client.
For a versioned preview use the executable inside the current `dist` version.
The MCP server accepts `--data-dir <path>` to join an isolated desktop/test
session. Both processes must use the same data directory.

The official Rust SDK handles initialization, negotiation, tool schemas, and
stdio framing. The desktop owns a single serialized engine worker, so desktop
actions and agent actions cannot interleave memory experiments. The MCP server
never creates a private simulator or duplicate session.

## Available tools

| Tool | Behavior |
|---|---|
| `reversebolt_status` | Session, values, matches, evidence, edit history |
| `reversebolt_list_processes` | Application windows with title, PID, creation identity and window handle |
| `reversebolt_attach_process` | Connect an exact process/window for observation; default 16 MiB capture budget |
| `reversebolt_capture` | Window image, OCR and bounded memory snapshot from connected application |
| `reversebolt_correlate` | Screen-number/memory hypotheses after at least 12 moments; no causal validation |
| `reversebolt_detach_process` | Release the connection and snapshots; leave application running |
| `reversebolt_open_practice` | New randomized disposable target; optional reproducible seed |
| `reversebolt_observe` | Twenty observation/memory samples |
| `reversebolt_discover` | Numeric correspondence hypotheses with held-out validation |
| `reversebolt_test_candidates` | Two interventions per match, scope checks, practice rollback |
| `reversebolt_set_value` | Change a uniquely supported control and check the observation |
| `reversebolt_undo` | Restore last current-session edit if its bytes still match |
| `reversebolt_history` | Shared recorded activity and patches |
| `reversebolt_close_target` | Close owned target and invalidate controls |

Typical investigation: status → open practice → observe → discover → inspect
candidate evidence → test candidates → inspect remaining ambiguity → set value
→ undo. A protocol response may contain `isError: true`; do not treat a delivered
tool call as a successful edit.

Application investigation: list processes → attach using `pid`, `created` and
`window` returned by the list → capture varied moments → correlate → inspect
evidence → detach. The `live` state is separate from the practice state; practice
write tools never edit the connected external application. A process restart
requires a new selection; PID alone is not accepted as an identity.

Only windows on the engine's Windows desktop are listed. For Anode testing,
launch the engine in Anode with an isolated `--data-dir`. A main-desktop engine
lists main-desktop applications.

## Local transport and limits

The app binds an ephemeral port on `127.0.0.1`; endpoint and random bearer token
are stored in its user-local data directory. The adapter checks the loopback
host, disables redirects and proxy routing, and reloads endpoint information for
each call. HTTP requests require the token and reject browser Origin headers.
The token protects against accidental unauthenticated clients, not other
software with access to the same user's files.

A file lock prevents two engines from owning one data directory. History
persists; active addresses do not survive engine restart. Commands are currently
bounded synchronous operations, not durable cancellable MCP jobs. Interrupted
clients should inspect status before repeating a mutation.

The practice transport permits writes only within the owned child's advertised
arena. Whole-arena rollback is an experiment convenience for that paused
fixture, not a rollback strategy for a real game. External process handles
currently request query/read access only. Captures retain up to 20 moments and
512 MiB of raw memory; images remain in the data directory. The optional attach
budget is 1–32 MiB per moment. The desktop uses 16 MiB.

Capture reads screen and memory sequentially without pausing the target.
Unavailable pages are reported and excluded from correlation; coverage and
timing are visible. A window-rendering timeout preserves memory evidence.
