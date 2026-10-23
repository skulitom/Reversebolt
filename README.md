# Reversebolt

An open-source Rust workbench for discovering and changing what controls a game.
Part of the Bolt family, with a desktop interface and an MCP interface sharing
one engine, session and history.

The goal is to connect an unfamiliar game and discover useful, named controls
such as health, movement or damage without shipping game-specific offsets or
handwritten trainers. Observation should include internal state, actions,
movement and appearance, including properties that never appear as HUD numbers.
See the [full vision](docs/VISION.md).

**Development preview — the end goal is not achieved.** This repository publishes
intermediate source, experiments and recorded failures. It does **not** reliably
turn an arbitrary game into a panel of health, ammo and damage controls. Current
progress is approximately **40/100** on our [capability rubric](docs/PROGRESS.md),
including **0/10 for the withheld-game generalization gate**. This is an internal
milestone estimate, not a compatibility rate or a release-readiness score.

## What works today

- A dark Windows desktop with an application picker. Connecting starts bounded
  discovery automatically; named findings appear as cards with short descriptions.
  Supported internal values have direct numeric entry, step buttons, toggles,
  guarded edits and a Freeze checkbox. Sliders require known small bounds.
  Unidentified readings stay off the main board.
- Native runtime inspection of supported IL2CPP applications, plus a fallback
  that compares captured screen numbers with sampled process memory. Discovery
  uses runtime evidence rather than a built-in list of game addresses.
- Optional local-model interpretation of runtime labels, screenshots and
  activity. Proposed names retain their evidence and uncertainty; a plausible
  name is not proof of a gameplay effect.
- A shared engine with durable jobs, edit history, restoration checks and human
  takeover. The stdio MCP server currently exposes 65 tools to the same session.
- Bounded before/change/restoration experiments and recovery of a previously
  tested control across a restart in a known development game. Agent workflows
  can request these experiments; connecting in the desktop does not start writes.

## Known gaps

| Area | What is still missing or unreliable |
| --- | --- |
| Generalization | No passed evaluation across multiple withheld, independently implemented games. Known development successes and practice fixtures do not satisfy this gate. Arbitrary engines, Linux/macOS, protected games and online play are not established targets. |
| Names and relevance | Runtime labels and model suggestions can be misleading, irrelevant or inconsistent. Citation checks do not establish actor identity, units, scope or actual gameplay meaning. Important values may be missed; a named card can still be unverified or unavailable for editing. |
| Hidden behavior and appearance | Motion and appearance measurements are partial. General inference of damage, collision, cooldowns, sound, animations and derived rules is unfinished. A screen change alone cannot prove those meanings. |
| Experiment selection | The first chosen field can be unrelated to the measured outcome. The Electrician trial selected an interaction flag for a movement-response test and found no suppression. Better selection and outcome measurements remain open work. |
| Capture timing | Recorded input is not always bounded by sufficiently consistent images. The latest native attempt stopped before editing because only three of four required response episodes qualified. Automatically collecting the missing evidence is unfinished. |
| Exploration | The bounded continuation policy passes saved-evidence checks, but continuing through multiple live comparisons after a negative result remains unverified. This workflow is currently exposed to agents and research runners, rather than being a complete autonomous desktop investigation. |
| Coverage and performance | Catalogues and memory samples are bounded and can be incomplete; paths may alias the same state. Model-assisted discovery can take several minutes, exhaust budgets or produce no useful controls. |
| Editing and restoration | Supported writes have fresh checks and narrow limits. Freeze is periodic, not a per-frame guarantee. Restoring selected bytes does not rewind the game world or all downstream effects. General natural-language edits and composed behavior changes are unfinished. |
| Recovery | One known control has been recovered and revalidated after a full restart. Reliable recovery across arbitrary scene loads, object replacements and game updates is not established. |
| Distribution and reproduction | This is a source preview, without a public binary release or installer. Earlier local builds triggered Windows Security detection; vendor clearance has not been established. Raw game captures, saves, memory archives, model weights and local binaries are excluded, so some historical audits cannot be rerun from this checkout alone. A complete binary dependency-notice audit remains part of the release work. |

## Evidence, including failures

These are recorded local development results, not a game compatibility list.
Research directories retain methods, machine-readable summaries and limitations.

| Experiment | Observed result | Limit |
| --- | --- | --- |
| [Hyperbolica: automatic comparison](research/hyperbolica-effects/README.md) | The planner chose a walking-speed hypothesis, changed 2.25 to 0, measured about 93% less rendered response, restored the bytes and observed response return. | Establishes an action-specific visual effect in a known development game; not physical units or generalization. |
| [Hyperbolica: restart recovery](research/hyperbolica-recovery/README.md) | Automatically recovered the learned route in fresh game/engine processes and repeated an effect/restoration comparison with no model calls. | One control and restart scenario; scene transitions and game updates remain untested. |
| [Electrician Simulator: first evaluation](research/independent-electrician/README.md) | Found seven provisional questions and completed one restored comparison. No suppression was observed. | A negative result, not a passed generalization test. No preferred replacement field was manually selected. |
| [Bounded exploration follow-up](research/bounded-exploration/README.md) | Thirty saved-native policy checks passed. A fresh game run stopped at the input/image timing gate with zero edits. | Live continuation after a negative comparison remains unverified. |

The latest local validation recorded **511 passing workspace tests**, one ignored
archive-dependent test, formatting and strict Clippy checks, a release build,
packaged headless startup and the 65-tool MCP catalogue check. Those checks
exercise implementation and protocol behavior; they do not measure universal
semantic accuracy. Historical notes retain earlier counts and failures.

## Build and try it on Windows

Use Windows x64, stable Rust with the MSVC toolchain, and Microsoft C++ Build
Tools with a Windows SDK. Build the **whole workspace** so the runtime loader,
runtime helper and MCP adapter are available beside the desktop executable.

```powershell
git clone https://github.com/skulitom/Reversebolt.git
cd Reversebolt
cargo build --workspace --locked
.\target\debug\Reversebolt.exe
```

Choose **Connect to application**, select a running window and connect. Play
normally while observation runs. A **Learning** or **Possible effect** card is an
investigation hint; only eligible values offer edits. Pause, Stop and Disconnect
remain available. **Practice** opens the disposable test fixture for trying the
basic workflow. Its supplied observations and arena make it easier than a real
game. See the [desktop preview guide](docs/PREVIEW_GUIDE.md) for detailed controls.

Optional local vision uses a model already installed in local Ollama; select it
in **Local vision**. The app does not download that model. The optional neural
text reader uses separately licensed, hash-pinned weights obtained explicitly
with `scripts/get-models.ps1`; see [model attribution](models/NOTICE.md). With no
neural text models, Windows OCR remains available where supported.

Session history is local under `%LOCALAPPDATA%\Reversebolt`.
`--data-dir <folder>` creates an isolated session and `--headless` runs the same
engine without the window. Native runtime inspection loads a helper into the
selected process. Use the preview with offline applications you can experiment
with, and retain your own saves.

## Agent interface

Start the desktop or headless engine, then configure an MCP client to launch
`target\debug\reversebolt-mcp.exe` using the absolute path on your machine.
The adapter connects to the engine's authenticated loopback API. Both processes
must use the same data directory; `--data-dir` is also accepted by the adapter.
See [MCP setup, tools and contracts](docs/MCP.md).

Agents share the desktop's observations, jobs, guarded changes and restoration
history. The person can stop a test or take control. An explicit MCP image-read
request sends that image to the requesting client; subsequent processing depends
on that client's configuration. Local processing does not imply that an external
agent client is local.

## Development

```powershell
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo build --workspace --locked
python scripts/smoke.py
```

Python is needed for the integration/research scripts, not the Rust desktop.
The smoke check launches an isolated engine and owned practice process.
Some research runners require a separately installed game, local model, Anode
or archived inputs, and may need local paths adjusted; read their protocols
before running them. Missing local
artifacts are not passing results. Do not commit endpoint credentials, saves,
screenshots, raw process memory, model weights or generated executables.

`scripts/publish-preview.ps1 -StageOnly` prepares a local versioned build.
Activating a preview updates the local desktop launcher; these scripts do not
publish a GitHub release. Existing connected sessions are not overwritten.

| Component | Responsibility |
| --- | --- |
| `reversebolt-core` | Evidence, matching, session state, comparison planning and guarded edit contracts. |
| `reversebolt-app` | Native process access, capture, engine service and egui desktop. |
| `reversebolt-runtime` | Bounded IL2CPP runtime inspection, loader and helper. |
| `reversebolt-mcp` | Stdio MCP adapter to the shared engine. |
| `reversebolt-jobs` | Durable jobs, interruption and human takeover (`reversebolt-session-jobs` package). |
| `reversebolt-vision` | Local text recognition, geometry and temporal regions. |
| `reversebolt-observer` | Local-model interpretation and experimental appearance/activity analysis. |

Useful contributions include reliable evidence collection, better experiment
selection, hidden-state and appearance measurements, portable access backends,
and reproducible evaluations that retain negative results. Keep reusable
inference separate from game-specific test setup. Adding a hardcoded trainer is
not evidence that discovery generalizes.

## License and provenance

Original Reversebolt code is released under the **[MIT License](LICENSE)**.
The independent MIT GameDiscoveryLab prototype contributed the original numeric
discovery and practice fixture; its notice is retained.

Third-party material keeps its own license: the PaddleOCR dictionary and optional
PP-OCRv6 weights are Apache-2.0; the vendored offline TAPIR research files are
Apache-2.0; the causal-validation research snapshot is MIT. Dependencies retain
their upstream licenses. Legacy Ocrs weights, if obtained separately, are
CC-BY-SA-4.0. See [source provenance](NOTICE.md), [model notices](models/NOTICE.md)
and the license files beside the included third-party material.

Squalr was studied as a reference. Its GPL-3.0 source is not included or linked
into this MIT implementation. The public repository contains source and compact
research summaries, not commercial game assets or the local evidence archives.
