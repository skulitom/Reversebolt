# Experiment results

50 unseen layout seeds (1000–1049) × 12 synthetic scenarios × 2 numeric tracks = 1,200 target searches.
Every search uses 20 snapshots of a 256 KiB arena: 10 fit the mapping and 10 validate it.
These are random-layout repetitions of one synthetic generator, not 1,200 games.
The discovery function receives only raw bytes and numeric observations.

| Scenario | Targets | True field in exact matches | True field in learned matches | True field passes probes | Other interpretations pass probes | Unique and correct after probes |
|---|---:|---:|---:|---:|---:|---:|
| Plain | 100 | 100 | 100 | 100 | 0 | 100 |
| FloatNormalized | 100 | 0 | 100 | 100 | 1 | 99 |
| ScaledInteger | 100 | 0 | 100 | 100 | 0 | 100 |
| Inverted | 100 | 0 | 100 | 100 | 0 | 100 |
| DoubleOffset | 100 | 0 | 100 | 100 | 198 | 1 |
| NoisyObservation | 100 | 100 | 100 | 100 | 0 | 100 |
| Nonlinear | 100 | 0 | 0 | 0 | 0 | 0 |
| RotatingXor | 100 | 0 | 0 | 0 | 0 | 0 |
| Relocation | 100 | 0 | 0 | 0 | 0 | 0 |
| Constant | 100 | 0 | 0 | 0 | 0 | 0 |
| Unaligned | 100 | 0 | 0 | 0 | 100 | 0 |
| Coupled | 100 | 100 | 100 | 0 | 0 | 0 |

Of 299 additional interpretations that passed the small probes, 299 overlap the actual field's bytes.
Passing a behavior probe does not establish the correct field boundary or numeric type.
One true candidate among several is recall, not an automatically usable control.

The exact baseline compares observed numbers directly. The learned method fits y = a*x + b using robust pair hypotheses.
It tries i32, f32, and f64 at natural alignment; it does not try nonlinear transforms, bytewise alignment, or moving-address tracking.
Probe checks ask for +3 and −4 changes, wait two idle fixture ticks, and require the other numeric track to remain unchanged.
Probe checks do not establish long-term gameplay correctness or safety in another application.

Live Windows check: both numeric tracks resolved to exactly one correct field; positive/negative writes changed observations as expected; original values were restored.
The fixture supplied the whole arena address and ideal numeric observations. No variable offsets were given to discovery.

Squalr exact HUD scan candidate counts: 222 → 1 → 1 → 1.
Its sole remaining address was a display copy. A +3 write left the underlying observed state unchanged.
The separate marker test verified Squalr's write/read round trip and whole-process exact scan returned the expected marker address.
