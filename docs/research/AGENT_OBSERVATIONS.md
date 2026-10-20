# Image-grounded agent observations — 2026-10-20

## Result and scope

Preview `0.1.0-20261020-210429` lets an MCP client inspect an actual captured
image and submit a typed interpretation to the shared desktop investigation.
The desktop shows its source, capture, explanation, uncertainty and history.
This is agent-assisted observation, not a built-in automatic vision model or
autonomous gameplay understanding. No game names, offsets, counter rules or
expected gameplay values were added to production code.

Blue Prince exposed why this matters: the existing OCR path misread an icon
beside its numeric counter. Its earlier failures remain recorded in
[observation evaluation](OBSERVATION_EVALUATION.md). External interpretations
are separate evidence and do not replace those readings or improve that OCR
benchmark by relabeling it.

## Real-game and MCP evidence

The engine and game ran in Anode's separate Windows session. The game's two
existing profiles were preserved; normal movement used the third test profile.
No game memory was written.

Three successive images were returned through real MCP stdio image content,
decoded, checksum-checked and individually inspected before submission. The
visible counter read **38, 37, 36**. The agent submitted those three numbers,
the visible room label, a provisional open-doorway observation, and an uncertain
movement observation with no value. A still image was not used to claim speed.

The same agent operated the game and read the images. This is a functioning
image-to-agent-to-engine integration test, not a blinded accuracy benchmark or
evidence of unattended game discovery. Three numeric samples are insufficient
for the matcher, and no memory control was established.

After rebuilding with archive persistence, a new connection captured the
counter at 36. Four observations were submitted again against that new image:
number, text, boolean and uncertainty. Anode UI inspection verified the visible
source, explanation, capture number and unverified status. The underlying OCR
reading remained separately visible and incorrect.

## Integrity and persistence checks

Every capture has an investigation identity, frame identity and SHA-256. Reads
and new submissions recheck the saved image. Exact replay does nothing;
conflicting replacements require a separate series. The engine records the
transport source; an observer/model name is a caller declaration.

Nine malformed or incompatible submissions were rejected without changing the
four accepted series: checksum, frame, investigation, bounds, conflicting
value, changed label, changed type, unknown field and missing provisional value.
A request claiming a desktop source still retained the agent transport source.
An old investigation was rejected after reconnecting.

The archive retained all four series and frame metadata. Restart preserved its
exact bytes and all 26 history events while invalidating the active connection.
Archive SHA-256:
`753aa40d124ea09f3be20adb07ba86c60f2a8b101b6f845bae1a29a3f88bfd2b`.
Archives are historical evidence; the application does not automatically resume
them or reuse their addresses.

The workspace's 13 tests, strict Clippy and formatting checks passed. Added
tests cover source separation, uncertainty exclusion, limits, altered images,
geometry changes, closed targets, out-of-order backfill, and failure to persist
before publication. A fixture verifies that provisional numeric observations
can reach the existing matcher; its supplied labels are not game inference.
The release practice/MCP smoke suite passed with 16 tools, including existing
edit/restore, authentication and restart checks.

## Local artifacts and reproduction

Raw evidence is local and ignored by Git:

- `test-results/visual-agent-v1/read-01`, `read-02`, `read-03`: actual MCP image
  responses and decoded PNGs, with corresponding `submit-*.json` requests.
- `integration-before-restart.json`: the three-reading investigation.
- `read-final`, `final-*.json`, `final-replay`: final build image and submissions.
- `verification.json`: nine rejections, replay, transport and archive checks.
- `archive-restart.json`: unchanged archive, retained history, invalidated live
  state after restarting the final binary.
- `test-results/smoke-cc3df47f/report.json`: release regression report.

`scripts/mcp-evidence.py <data-dir> <request.json> <output-dir>` runs a real MCP
call, saves text and decodes native image content without embedding base64 in
logs. It checks the image digest against the returned metadata. Use a fresh
frame's IDs and checksum; saved requests intentionally become stale across
connections. See [the MCP contract](../MCP.md) for the observation schema.

Final executable SHA-256:
`b0d32c5d18ec065faf93cae55db3e234e268799001162f85f883ce935fcdd513`.

## Remaining work

There is no automatic model invocation, multi-frame behavior observer, scene-cut
identity recovery, historical archive browser or durable investigation job yet.
Only provisional numbers enter matching; text, booleans and vectors are retained
as descriptive evidence. Uncertain observations are excluded. Matching remains
limited by the sampled memory and does not authorize external-game editing.
Independent, blinded evaluation and actual gameplay effect validation remain
required for generalization credit.
