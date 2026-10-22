# Runtime appearance integration, frozen before inference

Question: can the existing image observer supply original-image regions to the
same automatic appearance reader used by runtime discovery?

The product change schedules one existing observation pass after capture 2 of
the 15-capture runtime play prefix, in the background. Runtime inspection settles
that job's worker before archiving play evidence. No second visual pass is added.
At most one proposal and six crop reviews run; the existing pass checks its
90-second budget between calls, with each model request separately limited to
90 seconds. This is not a hard 90-second overall timeout.

Missing/invalid preparation inputs produce a durable unavailable receipt with
zero model calls. A resize before publication discards the obsolete appearance
with a warning. Model rejection/unavailability remains optional; failed evidence
storage, replaced investigation and ownership violations remain hard failures.
No field write or edit authority is added.

Evaluation, fixed in advance:

1. Run automated app tests: model/no-model job planning, early publication before
   the native precondition, original capture-2 binding, regional archive, missing
   timing, resize, cancellation, target replacement and persistence failures.
2. Freeze sources, binary, manifests and input hashes before model use.
3. Run exactly one pass on each of the original first two images from the
   retained Hyperbolica and Manifold Garden discovery sequences. Use installed
   `qwen3.8:latest`, digest
   `22130167c4c20e20c7b71454612966ca8e8171e9b3cc8ab6ce8aa6cbfec79643`.
   Use unchanged prompts and thresholds. No retries or image selection after
   viewing outcomes. Game names and source metadata are not sent to the model.
4. Replay publication after all 15 original frames are already available. Use the
   production worker/publication and play-evidence reader with a file-only target
   that panics on native reads, captures or writes. Original frame metadata and
   hashes are preserved; replay process identity is explicitly artificial.
5. Compare with the same reader before adding observations. Base play evidence
   must remain identical. Record all model receipts, rejected/no-item outcomes,
   selected regions and appearance stops; verify source/input hashes afterward.

These exposed saved cases establish integration behavior only. They cannot prove
new-game generalization, localization accuracy, persistent identity, action
meaning, useful gameplay controls or live performance. No native game run is
attempted while desktop publication remains unavailable.
