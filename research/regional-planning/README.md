# Regional comparison history in investigation plans

The shared application's `plan_runtime_comparisons` operation now includes the
regional intervention result for completed comparisons with a region saved
before editing. Previously it exposed only the older whole-screen assessment,
which can be inconclusive when a small region has a supported response.

The core planner still selects prior tests by exact path, proposed value, action
sequence, investigation, catalogue, discovery report and frame/input metadata.
The application checks the selected summary/path/value/actions against the
current retained test before deriving its regional outcome from saved evidence.
It returns the same compact assessment as `read_runtime_test`, including test,
trial and measurement-report digests. No caller-supplied measurement is accepted.

For those completed regional tests, `status` comes from the regional assessment
and `status_basis` is `regional_intervention`. The scalar assessment remains in
`whole_screen_status` and the unchanged `previous_test`. The retained experiment,
choice order and candidate authority are unchanged. Only `untried` choices can
become `next`; positive, negative, inconclusive and stopped tests are not retried
automatically in the same evidence context. Active and legacy tests keep their
existing behavior.

Policy `regional_comparison_history_v1` bounds recomputation to eight distinct
tests, at most 144 template checks. Failed analyses consume budget. Duplicate
test IDs reuse the result within the request; omitted tests are listed explicitly.
An unavailable analysis or exhausted budget is inconclusive with a distinct
`status_basis` and error. A fresh failed review removes any previous derived
assessment. The planner never substitutes a positive scalar outcome for missing
regional evidence. Valid but inconclusive assessments are separate from analysis
errors. The plan includes compact outcomes, not images or full matching reports.

Verification covers two layers:

- The existing generated PNG/input and synthetic edit-receipt integration test
  now checks the same history helper called by the planner. Its supported result,
  persisted restart, missing linked trial, corrupt image and unchanged-motion
  result agree exactly with the shared reader. An untried choice, original
  scalar assessment, retained test and unverified candidate remain unchanged.
- A scheduling test uses deliberately incomplete records and a private mocked
  evaluator to exercise the eight-test limit, duplicate reuse, errors consuming
  budget, omission, stale-result removal, wrong record digest, mismatched
  path/value/actions/summary, and skipped stopped/legacy records. These mocks
  test scheduling and binding only; they do not validate intervention evidence.

The whole live discovery/planner endpoint is not replayed by the new test: the
real regional-history helper is exercised directly, while existing core planner
tests cover prior-test context selection and ordering. Standalone examples that
call core `choices` directly do not gain the application's regional annotation.

All **377 workspace test executions**, strict all-target Clippy and format checks
pass. Logs are recorded under
`test-results/regional-planning-v1`, with source and log hashes. No game, native
write, screenshot, input action or model call is needed for these checks.

Overall progress remains **35%**. This closes a planning integration gap; it does
not establish semantic meaning or generalization to unseen games. A supported
regional pattern can still reflect camera/background motion or other changes.
Fresh real-game validation remains outstanding. Desktop publication remains
unavailable after quarantine. Artem explicitly requested that the executable and
security review packet stay local; nothing was submitted externally.
