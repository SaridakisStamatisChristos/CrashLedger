# CrashLedger phased implementation

Source of truth: CrashLedger_Architecture_MVP_PR_Roadmap_v1.md supplied as the implementation handoff.

| PR | Objective | Acceptance |
| --- | --- | --- |
| 01 | Cargo workspace, typed run/verdict model, YAML validation, CLI validate, CI | Good v1 parses; unknown/unsafe/oversized configurations reject; formatter, lint and test gates |
| 02 | SQLite broken/fixed reference order service | Same key twice yields two/one orders; fixed key/payload conflict HTTP 409; concurrent fixed key stable |
| 03 | Bounded HTTP/1.1 driver and one-shot response-suppression proxy | First 2xx fully buffered, zero downstream response bytes, transport error, exactly one fire, retry |
| 04 | Independent read-only SQLite oracle, evaluator integration, demo | Broken FAIL/count=2/exit=2; fixed PASS/count=1/exit=0; missed fault INCONCLUSIVE |
| 05 | JSONL evidence, hashes and report CLI | External reader reconstructs verdict; corrupted bundle rejected; sensitive data redacted |
| 06 | User-authored standalone scenario runner and adversarial tests | Run against separate local fixture; 100 isolated cycles; cancellation and unavailable oracle |
| 07 | Cross-platform CI, docs, threat model and ADRs | CI and documented commands exercise contract; no unverified claims |
| 08 | v0.1 release candidate | Clean checkout build, broken/fixed demos and evidence readback |

**PR-01 is merged. PR-02 is scoped strictly to the independent reference fixture. Neither PR-01 nor PR-02 supplies PR-03/04 transport fault or end-to-end evidence.** No universal exactly-once guarantee is claimed. License selection remains owner-controlled.
