# V2 diagnostic attempt

V1 stopped on exact equality of recomputed play evidence before producing an
agent plan. The original inputs, executable identity and failure remain retained.
V2 changes only replay failure diagnostics: it writes actual and expected evidence
on mismatch. It keeps the exact comparison and the same input set, with no model
or target access. Determine the differing fields before changing any acceptance
criterion; do not count either failed attempt as a passed handoff replay.
