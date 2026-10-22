# Regional responses joined to recorded edits

The shared history reader now returns `regional_intervention_assessment` beside
the existing appearance comparison. The final recovery observation also returns
this compact assessment automatically when a region was saved before editing.
It joins the measured image pattern to the exact linked runtime trial, rather
than treating changed pixels alone as evidence of an intervention.

Policy `recorded_regional_intervention_v1` requires:

- A completed comparison and restored trial for the same scope, path and value.
- Original-value preparation and a region chosen before edit preparation.
- Valid, completed edit and restore **write** receipts. Finding the original
  value already present is insufficient evidence of a restore write.
- Ordered baseline selection, edit, changed images/readback, restoration and
  recovery images/readback. Reads must confirm the expected values at the same
  field address and complete object-reference chain.
- Matching sampled actions from one monitor, with comparable per-key hold times.
- The existing checksum-verified image/input comparison of the saved region.

With those records supported, the unchanged regional matcher can yield
`regional_change_and_return_observed`, `regional_change_not_observed`, or
`inconclusive`. Missing/inconsistent operation evidence also yields inconclusive,
with an `intervention_error`. Full test, trial and measurement-report digests bind
the result. A report from a different test state is rejected. No caller can
submit a measurement report through the application/MCP interface.

The old whole-screen assessment and candidate authority stay unchanged. A small
subject can have a supported regional pattern while its whole-screen assessment
is inconclusive. The completion response includes only the compact joined result;
history exposes the complete measurements. Unavailable images preserve readable
test metadata. No new capture, target access, input, write or model call occurs
while deriving the assessment.

Automated evidence uses generated PNGs, sampled inputs and **synthetic operation
receipts**. It is a software integration test, not a real-game intervention:

1. Measure the actual whole-screen metric and complete baseline capture through
   the core state machine. Select and save a supported region before the test's
   generated edit timestamp.
2. Complete changed and recovered conditions through capture completion, supply
   matching readbacks and finalize the linked comparison. The whole-screen
   result remains inconclusive; regional change-and-return is supported.
3. Compare completion and shared-reader results, reopen the saved trial/test and
   require identical history. The candidate remains unverified and unchanged.
4. Remove the trial, substitute an incorrect changed-value readback, corrupt an
   image, or replace the changed condition with unchanged motion. Missing
   evidence is inconclusive; unchanged motion is a negative result.
5. Reject a rebound report and 17 core countercases covering phase, cursor state,
   identities, readbacks, reference movement, receipt hashes, timing, input holds,
   monitor changes, missing selection and operation errors. A separate valid
   already-original restoration receipt is also rejected for this assessment.

All **376 workspace test executions**, strict workspace/all-target Clippy and
format checks pass. Logs and a source/log hash receipt are retained under
`test-results/regional-intervention-v1`. No native edit or independent gameplay
evaluation ran. This checks the evidence plumbing; matching background, camera
motion, external changes and downstream effects remain possible explanations.
The result does not establish gameplay meaning, persistent object identity,
physical speed, exclusive causality or complete world recovery.

Overall progress remains 35%. A fresh real-game comparison using this flow is
still needed. Desktop publication remains unavailable after Windows quarantine.
