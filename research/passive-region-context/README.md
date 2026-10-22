# First passive-region context experiment

The new automatic region summary reached the interpreter, but sending the full
provenance-rich summary in its prompt exhausted the context window in one case.
This failed run is retained unchanged and motivates the separately frozen
[compact projection revision](../passive-region-context-v2/README.md).

Both numeric proposal reports remained exactly equal to the stable-region study.
The four local model calls used identical image bytes, source metadata, global
motion and existing appearance summaries within each pair of arms. Only the new
regions differed. No screenshot, game input or target memory operation ran.

| Case | Baseline | Full-summary arm |
| --- | --- | --- |
| Hyperbolica | Completed, positive flag | Truncated; no interpretation/context published |
| Manifold Garden | Completed, positive flag | Completed, positive flag; exact regions retained |

Hyperbolica's augmented response reported `prompt_eval_count=8079`,
`eval_count=113` and `done_reason=length` against `num_ctx=8192`. It ended partway
through the uncertainty string. The reader correctly rejected this unfinished
response. The 11,925-byte prompt included 3,738 bytes of full regional summary;
most provenance fields are useful for the archive but not for image interpretation.
Garden's 10,564-byte augmented prompt completed and preserved its exact summary
through `Receipt.context()`. These outcomes do not establish semantic accuracy.

All 369 workspace test executions and strict Clippy passed before this first run.
The source archive includes summary geometry/stop/source tests, original-pixel
conversion, legacy serialization and the automatic no-target-access handoff test.
At completion, all 163 sources, 18 original inputs, 21 generated inputs and both
executables passed their unchanged audit. [Results](results.json) retain every
observation/error; raw receipts and prompts are in
`test-results/passive-region-context-v1`.

- Manifest SHA-256: `2ea6583c086603448b551c3cd6c45b28697dc8d1d2ce5154908d4cf477f30413`
- Interpretation inputs SHA-256: `9e496afc6288d96b9d39c39989c90f6c2c4485241a23d28ed23598e5c834f45c`

No retry or source correction was made inside this study. The successor study
keeps the full summary in durable records and changes only its model-facing
projection. The prior source/binary archive remains the authority for this run.
