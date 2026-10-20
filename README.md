# Reversebolt

An open-source Rust workbench for discovering what controls an unfamiliar game.
Part of the Bolt family. Humans and agents use the same engine and session.

**Research preview:** this first desktop build works with a disposable practice
process. It does not yet recognize or edit arbitrary games. The destination is
documented in [the full vision](docs/VISION.md), with an explicit
[progress rubric](docs/PROGRESS.md).

## Try it on Windows

Requires stable Rust and the Microsoft C++ build tools.

```powershell
cargo run -p reversebolt-app --bin Reversebolt
```

Choose **Open practice → Observe changes → Find matches → Test effects**.
Each practice process has new random offsets and a numeric encoding. Discovery
receives synchronized bytes and observations, not the true field locations.
Testing separates controlling fields from misleading display/telemetry copies.
When a unique candidate passes, enter a value and choose **Apply**.
**Restore last edit** checks the bytes have not changed before restoring them.

The two labels and observations come from the practice adapter. Twenty random
state changes, a supplied arena boundary, and paused whole-arena rollback make
this an intentionally easier problem than a real game.

Session history is saved under `%LOCALAPPDATA%\Reversebolt`. Previous process
addresses are invalidated on restart. `--data-dir <folder>` runs an isolated
test session; `--headless` runs the same engine without the window.

## Agent interface

`reversebolt-mcp` is a stdio MCP server using the official Rust SDK. It connects
to the running desktop's authenticated loopback API. Agent edits and desktop
edits appear in the same history. See [MCP setup](docs/MCP.md).

## Develop and verify

```powershell
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
python scripts/smoke.py
```

`scripts/publish-preview.ps1` builds versioned preview binaries, updates the
desktop launcher, and can launch the preview. Existing running versions are not
overwritten. No global MCP client settings are changed.

## Architecture

- `reversebolt-core`: general numeric matching, evidence, shared commands,
  serialized session model, verified practice edits and restoration.
- `reversebolt-app`: Windows native process transport, engine worker, local API,
  egui desktop and isolated practice fixture.
- `reversebolt-mcp`: official MCP protocol adapter to the same engine.

The current practice adapter is deliberately separate from the discovery
algorithm. General screen, motion, appearance, object and rule inference,
live-game experiments, durable cancellable jobs, and restart revalidation are
future milestones, not advertised capabilities.

## Research and license

MIT. Independent code originated in the MIT GameDiscoveryLab research prototype.
Its measured failure cases and Squalr study are preserved under `docs/research/`.
Squalr was evaluated as a reference/backend candidate. It is GPL-3.0; no Squalr
source has been copied into this MIT implementation. Any future integration must
make its licensing implications explicit.
