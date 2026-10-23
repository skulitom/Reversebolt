# Source provenance

Reversebolt is independently implemented under MIT. The numeric discovery
algorithm and practice fixture originated in the MIT GameDiscoveryLab prototype
at commit `207e6cc`; both copyright notices are retained in LICENSE. New session,
desktop, transport, MCP, delivery and validation code was added for Reversebolt.

`research/action-equivalence/` includes an MIT snapshot of the independent
causal-validation research supplied in `experiments/causal-validation/`.
Its original notices are retained in the vendored LICENSE; `provenance.json`
records original and copied source hashes and identifies the adapted evaluator.
This standalone experiment is not linked into the desktop or MCP binaries.

The research notes evaluate Squalr at commit
`568aa7589b68b2fd4621cc66c6dde23fa14c7f50`. Squalr is GPL-3.0. Its code is not
included in this workspace or linked into this preview.

The unmodified TAPIR source files under `research/learned-tracking/vendor/`
retain their Apache-2.0 license. The PaddleOCR character dictionary retains its
Apache-2.0 attribution and license in `crates/reversebolt-vision/`; related
research material retains its notices under `research/ocr-v6/`. These licenses
are not replaced by the repository's MIT license. Model weights are not included.

Rust dependencies retain their own licenses. Cargo.lock records the exact
resolved versions. This public repository is an intermediate source release;
no compiled application, dependency binaries, game assets or local evidence
archives are distributed here. A complete redistributable dependency notice
bundle and license audit remain part of the binary-release gate.
