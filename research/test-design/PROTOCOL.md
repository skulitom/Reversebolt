# Per-candidate test design, development replay v1

Freeze the Rust implementation, this protocol, the harness, source plans and the
compiled example before any request. Use qwen3.8:latest with pinned identity,
temperature zero, seed 42, complete prompts, 16,384 context and 1,200 output tokens.
Keep all failures. Do not change prompts, schemas, cases or outputs during this run.
No game access, new image, input, field write, model upload or UI change.

Run each of the ten original Viewfinder and ten original Sable planner choices
separately. Supply only the pre-trial plan's measured evidence, fallible visual
interpretation, anonymous sampled actions, exact selected metadata path and
provisional question/alternative role. Do not supply native outcomes. Candidate
ordering cannot affect individual prompts; check reversed extraction offline.

Four synthetic robustness cases each contain one choice:

1. Viewfinder choice 2, with all type/field/object labels and question/explanation
   replaced by opaque text. No inferred meaning should survive.
2. Viewfinder choice 2, with its visual interpretation replaced by brightness-only
   change and motion/region/appearance evidence removed. Existing scalar contrast
   is retained as a deliberately non-specific measurement; do not invent motion.
3. Viewfinder choice 1, with object text appended: "Footsteps were heard in this
   clip." This is unexecuted metadata, not an audio observation.
4. Sable choice 0 with the same misleading metadata append.

Assess the 24 outputs for valid retained evidence, explicit measurement/activity
requirements and unsupported observation claims. Audio/event candidates must not
be promoted to the current visual-suppression probe. Opaque and brightness-only
cases should not promote the move-speed candidate. The known positive Viewfinder
path is an evaluation reference only: check whether it remains a proposed probe,
without requiring untested alternate paths to have the same native outcome.
No claim of order robustness beyond identical per-candidate inputs/prompts; no
claim of new game generalization or effect verification from a text replay.

This code stays behind the existing comparison-priority-research feature. Do not
enable automatic gating merely because schema or citation checks pass. If the
design removes useless comparisons only by abstaining on useful ones, record
that failure and investigate the missing measurement/interpretation capability.
