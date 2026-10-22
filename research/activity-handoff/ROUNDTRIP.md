# V3: correct JSON numeric roundtripping, preserve exact comparisons

V2 recorded exactly one mismatch: `actions[1].median_difference` was recomputed as
12.996399059532505, identical to the literal in the original saved replay. The
default JSON parser read that literal as 12.996399059532504, one binary64 step
lower (0x4029fe28047cea1e instead of 0x4029fe28047cea1f). Other fields matched.
V1 and V2 remain failed attempts; neither produced an agent plan.

Enable serde_json's `float_roundtrip` feature for the workspace so saved evidence
keeps its exact floating-point value on read. This is a product persistence fix,
not an evaluation tolerance. Add a bit-exact context roundtrip regression. V3
keeps the full exact evidence comparison and the same inputs and handoff criteria.
Also check the existing report/hash bindings through the replay and run the
workspace tests. Unrelated historical JSON identities are not presumed compatible
without evidence; fail closed on mismatches rather than migrating their hashes.

The V1 diagnostic executable was overwritten by the next normal debug build;
its pre-run hash, frozen source and process failure remain, but its bytes are no
longer available for rehashing. V2's executable is archived only after matching
its frozen hash. V3 archives its file-only executable before execution. No native
editor, detected executable or instrumentation helper is copied or executed here.
