# Repair the offline batch-validation boundary

The separate partial-review continuation completed all 98 native reads and
confirmed unload, but every original analysis call rejected its batch because
it reused a UI task limited to four values. Preserve those seven failures.
The native reader accepts up to 32 paths; collection itself completed.

Extract native refresh validation into `runtime::validate_refresh`, shared by
the existing UI refresh handler and the response analyzer. Keep process/path,
scalar kind, byte/value, count and read-gap checks. Do not enlarge the UI task
limit. Add a 32-field regression covering a missing read and contradictory bytes
beyond the fourth field. Keep the input timing, address stability, fourteen
samples, quiet-pair equality and four-repetition association criteria unchanged.

After tests, freeze the repaired source and analyzer binary, then replay each
retained observation batch exactly once into a separate repair directory. Bind
the original plan and all observation documents by hash. Retain every accepted
or rejected result. No target, model, screenshot, new input or field write is
needed. This repairs analysis of existing data; it does not turn the original
attach/naming failure into success or establish meaningful gameplay controls.
