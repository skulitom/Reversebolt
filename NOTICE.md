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

Rust dependencies retain their own licenses. Cargo.lock records the exact
resolved versions. A complete redistributable dependency notice bundle and
license audit are part of the public-release gate; this preview is currently
delivered locally for development and testing.
