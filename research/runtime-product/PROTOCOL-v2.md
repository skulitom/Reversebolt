# Product workflow v2 — bounded request documents

The original v1 run connected successfully and automatically published 64 cards
with zero image captures. The four-ID refresh then failed with `invalid plan
size`: its serialized step exceeded the job journal's 256-byte ceiling. The
fixture exited normally with five starts and five stops. No refresh helper ran.
Keep that failed run, its binaries and source revision unchanged.

Store each validated runtime task as a small immutable JSON document identified
by its SHA-256. The journal stores only that digest alongside the exact scope;
the engine checks the document's digest and schema before execution. Replay of
the same task uses the same document and journal payload. Keep the existing
256-byte journal ceiling and full four-value refresh interface. Add a regression
covering maximum-size cursors/identities, duplicate storage and modified documents.

Freeze v2 separately and rerun the unchanged product_case.py and fixture_host.py.
All original protocol assertions, selection rules and limitations still apply.
