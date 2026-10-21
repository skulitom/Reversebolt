"""Audit a frozen run's bookkeeping and sources, then save a compact record.

Run from the repository root. This is not another gameplay evaluator; fixture
effect/private-state audits occur after inference in the Rust executable.
"""
from pathlib import Path
from collections import Counter, defaultdict
import hashlib
import json
import sys


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


root = Path("research/action-equivalence")
raw = Path(sys.argv[1] if len(sys.argv) > 1 else "test-results/action-equivalence/frozen-v1.json")
freeze = json.loads((root / "evidence/freeze-v1.json").read_text(encoding="utf-8"))
for path, expected in freeze["sources"].items():
    assert digest(path) == expected, f"frozen source changed: {path}"
assert digest(freeze["executable"]) == freeze["executable_sha256"]
provenance = json.loads((root / "provenance.json").read_text(encoding="utf-8"))
for entry in provenance:
    assert digest(root / entry["copy"]) == entry["copy_sha256"]
    # The original user-owned experiment need not exist in a fresh checkout.
    if Path(entry["original"]).exists():
        assert digest(entry["original"]) == entry["original_sha256"]
    if not entry["modified"]:
        assert entry["original_sha256"] == entry["copy_sha256"]

data = json.loads(raw.read_text(encoding="utf-8"))
assert data["seeds_per_case"] == freeze["seeds_per_case"]
assert data["start"] == freeze["seed_start"]
assert data["requested_deltas"] == [3, -4]
assert data["action_replays"] == 2 and data["audit_ticks"] == 32
cases = data["cases"]
assert len(cases) == freeze["expected_searches"]
assert sum(len(r["actions"]) for r in cases) == freeze["expected_requests"]
index = {}
counts = defaultdict(Counter)
reasons = Counter()
for run in cases:
    key = (run["suite"], run["scenario"], run["seed"], run["channel"])
    assert key not in index, f"duplicate search: {key}"
    index[key] = run
    assert freeze["seed_start"] <= run["seed"] < freeze["seed_start"] + freeze["seeds_per_case"]
    prior = run["prior"]
    assert len(set(prior["survivors"])) == len(prior["survivors"])
    assert all(0 <= i < len(run["candidates"]) for i in prior["survivors"])
    canonical_correct = False
    if prior["decision"] == "IdentifiedWithinScope":
        selected = run["candidates"][prior["selected"]]
        canonical_correct = [selected["offset"], selected["kind"]] == run["truth_after_answer"]
    for group in (run["suite"], run["suite"] + "/" + run["scenario"]):
        c = counts[group]
        c["searches"] += 1
        c["canonical_identified"] += prior["decision"] == "IdentifiedWithinScope"
        c["canonical_correct"] += canonical_correct
        c["searches_with_at_least_one_scoped_action"] += any(a["outcome"] == "ObservedWithinScope" for a in run["actions"])
        c["prior_interventions"] += prior["interventions"]
        c["prior_rollback_failures"] += prior["rollback_failures"]
        c["prior_disposal_failures"] += prior["disposal_failures"]
        c["prior_private_effects_remaining"] += run["hidden_prior"]
        c["action_private_effects_remaining"] += run["hidden_actions"]
    earlier_failed = False
    assert len(run["audits"]) == len(run["actions"])
    for action, audit in zip(run["actions"], run["audits"]):
        status = action["outcome"]
        plan = action["plan"]
        observed = status in ("ObservedWithinScope", "ObservedWithUnknownEffects")
        if earlier_failed:
            assert status == "NotTested" and not action["attempts"]
        earlier_failed |= status == "Failed"
        if not plan["error"]:
            members = [i for g in plan["groups"] for i in g["interpretations"]]
            members += [b["interpretation"] for b in plan["blocked"]]
            assert sorted(members) == list(range(len(prior["survivors"])))
        if action["attempts"]:
            assert not plan["error"] and not plan["blocked"] and len(plan["groups"]) == 1
            patch = plan["groups"][0]["patch"]
            assert patch and len(patch["before"]) == len(patch["after"])
            assert 0 < len(patch["after"]) <= 8
            assert patch["before"][0] != patch["after"][0]
            assert patch["before"][-1] != patch["after"][-1]
            assert prior["rollback_failures"] == 0 and prior["disposal_failures"] == 0
            assert len(prior["trials"]) >= 2
            assert all(t["error"] is None and t["disposed"] and t["recovery"] in ("Verified", "AlreadyOriginal", "NoWrite") for t in prior["trials"])
        if observed:
            assert len(action["attempts"]) == 2
            assert (status == "ObservedWithinScope") == run["scope"]["closed_effect_scope"]
            for attempt in action["attempts"]:
                assert attempt["error"] is None and attempt["wrote"] and attempt["disposed"]
                assert attempt["recovery"] in ("Verified", "AlreadyOriginal")
                assert len(attempt["observations"]) == 3
                assert all(abs(f[run["channel"]] - action["desired"]) <= run["scope"]["tolerances"][run["channel"]] for f in attempt["observations"])
            assert audit is not None
        else:
            assert audit is None
            reasons[action["reason"]] += 1
        bad_audit = bool(audit and (not audit["effects_ok"] or not audit["restored"] or audit["hidden_effect"]))
        for group in (run["suite"], run["suite"] + "/" + run["scenario"]):
            c = counts[group]
            c["requested_actions"] += 1
            c[status] += 1
            c["action_attempts"] += len(action["attempts"])
            c["action_rollback_failures"] += sum(a["recovery"] not in ("Verified", "AlreadyOriginal", "NoWrite") for a in action["attempts"])
            c["action_disposal_failures"] += sum(not a["disposed"] for a in action["attempts"])
            c["audited_actions"] += observed
            c["failed_extended_audits"] += bad_audit
            c["failed_scoped_action_audits"] += bad_audit and status == "ObservedWithinScope"
            c["scoped_actions_after_canonical_abstention"] += status == "ObservedWithinScope" and prior["decision"] == "Abstain"

# Same available evidence must not magically reveal the private source type or
# hidden state. Compare complete blind answers, never their truth/audit fields.
for seed in range(freeze["seed_start"], freeze["seed_start"] + freeze["seeds_per_case"]):
    def get(name):
        return index[("previously_seen_withheld", name, seed, 0)]
    for names in [("NarrowFloat", "NarrowBits"), ("HiddenClean", "HiddenLatent", "HiddenImmediate")]:
        for name in names[1:]:
            for field in ("candidates", "prior", "actions"):
                assert get(names[0])[field] == get(name)[field], (names, field)
    assert get("NarrowFloat")["truth_after_answer"] != get("NarrowBits")["truth_after_answer"]

record = {
    "schema": 1,
    "raw_record": str(raw), "raw_sha256": digest(raw), "raw_bytes": raw.stat().st_size,
    "freeze": "freeze-v1.json", "freeze_sha256": digest(root / "evidence/freeze-v1.json"),
    "verifier": str(Path(__file__).resolve()), "verifier_sha256": digest(__file__),
    "searches": len(cases), "counts": {k:dict(v) for k,v in sorted(counts.items())},
    "reasons": dict(reasons), "identical_evidence_twin_checks": "passed for every seed",
    "source_and_record_checks": "passed", "independent_game_evidence": False,
}
(root / "evidence/results-v1.json").write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
for suite in data["summary"]:
    print(suite, dict(counts[suite]))
print("All source, scope, recovery, action-accounting, and indistinguishability checks passed.")
