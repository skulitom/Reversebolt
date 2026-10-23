# Embedded sibling activity

The shared Rust sampler now retains the enclosing object and full member path of
scalar leaves inside structures. It walks back only through inline field storage;
references and array elements remain separate roles. Within each role it alternates
direct and embedded leaves, preserving discovery traversal order. Runtime labels,
values and observed changes do not rank the fields or grant editing authority.

The observer also exposes `runtime_type_selection` and the hypothesis CLI accepts
`--select-only`, so an investigation can measure activity before asking for complete
control names. This research path is not yet the desktop's automatic sampling loop.
The shared naming path does receive a catalog-size fix: a paged aggregate may use
up to 288,000 bytes, while each request stays within 12,000 bytes and the full
selection within 24 requests. An unaffordable round is rejected before requesting
its first page. The later full naming proposal still has its existing timeout.

## Native development result, 2026-10-23

Protocol: [initial measurement](PROTOCOL.md), [bootstrap continuation](RESUME-PROTOCOL.md).
Artifacts: `test-results/runtime-embedded-activity-v1`, `-v2` and `-v3`.
Sable is an already examined development game, not a withheld acceptance target.

* One native discovery covered 3,840 of 4,173 catalog roots. The remaining 333
  roots were outside this page; no completeness claim is made.
* Eighteen completed local-model requests selected eight types. No control-name
  proposal was requested. The model identity and every prompt/response are retained.
* The frozen plan selected 128 paths across 17 rooted roles, including 24 embedded
  paths. Another 518 eligible paths were outside that selection budget.
* The first eight of eleven batches measured 92 paths, including 16 embedded paths,
  at 78 distinct initial field addresses. The other 36 selected paths were not
  measured. Each batch used the same fourteen-sample `QQDQQAQQDQQAQQ` sequence.
* All 1,288 fresh scalar readings succeeded with stable observed locations. Six
  paths changed, 86 were constant. All sixteen measured embedded paths were constant.
  **Zero paths passed the unchanged four-repeat input-association rule.** Eighty-eight
  paths had stable quiet pairs for all four pulses; quiet stability alone is not
  a response to input. No semantic claim or useful control was established.
* One discovery plus 112 refresh calls completed, and every helper unloaded.
  The 490 observed pauses had zero helper Rust heap allocations/deallocations.
  Maximum stop-to-resume was 5,748 microseconds; the five-ms work budget is not
  a maximum scheduling latency guarantee. The saved evidence reanalysis passes.
* The measurement used 32 bounded A/D holds, no field writes and no viewed images.
  Setup used three viewed screenshots and two menu keys. Foreground/key/QPC
  coverage was verified; actual game consumption and visible movement were not
  independently measured. This is a limit on interpreting the negative result.

The initial preflight found that the expanded catalog exceeded the old single-
prompt aggregate cap (139,686 versus 110,000 bytes). No native measurement or
inference had started in v1. After the catalog fix, v2 completed discovery and
selection but stopped before its first sample/input because its window handle
had been queried from the parent desktop and was zero. Its terminal failure and
empty hold list are retained. V3 used the verified session-local handle and the
same saved plan, measurement functions and binaries. It did not repeat discovery
or model requests. Both failures remain visible rather than being replaced.

The owned game and test preview exited and the Anode lease was released. The main
save, save image, settings and cloud metadata are unchanged; logs and SaveManager
changed. The pre-launch folder is retained. The user's desktop editor remained
open and responsive.

## Delivery and limits

Preview `0.1.0-20261023-034437` passed an ordinary Anode startup/accessibility check.
Its rebuilt reader and loader also passed twelve fresh path reads in the same game,
then unloaded: three additional pauses, zero helper heap operations, maximum
724 microseconds. This separate publication check does not alter the frozen
experiment. No security settings or allowances were changed.

All 474 workspace tests with all features, strict all-target/all-feature Clippy,
formatting and the frozen evidence audit pass. The desktop shortcut selects this
preview for the next launch; it does not replace the running older editor.

The expanded context and catalog handling improve investigation infrastructure.
They do not establish automated understanding or cross-game generalization; the
full-goal estimate remains **36/100**. Many fields were omitted, paths can alias,
and exact quiet-pair equality is deliberately restrictive. A negative bounded
sample cannot establish that the game has no useful state or that a different
sampling design would fail. The next useful experiment should first verify that
the intended action actually produces a measurable game response, then distinguish
changing actor state from constant configuration and unrelated drift. Simply
repeating larger metadata samples is not supported by these results.

Reanalysis without game access:

```powershell
python research/runtime-embedded-activity/audit.py C:/DEV/Reversebolt/test-results/runtime-embedded-activity-v3
```

Fresh execution requires a new output directory, a pre-launch storage snapshot,
an owned process identity and a window handle obtained inside its desktop session.
The supplied resume scripts preserve this particular settled failed start; never
replay them against a started continuation or a new process. The frozen manifests
identify exact source, native binary, analysis binary, model and plan hashes.
