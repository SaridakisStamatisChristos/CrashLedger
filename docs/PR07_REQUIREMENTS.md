# PR-07 R7 requirements and acceptance

| ID | Hard scope | Evidence |
| --- | --- | --- |
| Q01 | Linux Rust format, clippy, locked workspace tests, broken/fixed real demos and offline negative-control verification | Ubuntu GitHub Actions |
| Q02 | Build and at least supported workspace unit/integration tests on macOS and Windows | portable CI matrix |
| Q03 | README commands, scripts/check.sh and scripts/demo.sh exercise actual CLI exit contracts | CI executes demo script |
| Q04 | Document architecture, separate trust boundaries, correctness claims, redaction, threat model and evidence | docs/ARCHITECTURE, CORRECTNESS_CONTRACT, THREAT_MODEL, EVIDENCE_SCHEMA |
| Q05 | Preserve ADRs for local-first, read-only observer, zero-byte fault, four verdicts | docs/adr/0001–0004 |
| Q06 | Developer workflow, roadmap and change history make project runnable by others | CONTRIBUTING, README, CHANGELOG |
| Q07 | No Docker/privileges/external network required for demo; no false production-ready or universal exactly-once claims | review and Linux CI |

Linux remains the platform for 100-pair stress and externally managed target tests. Portable macOS/Windows jobs exercise Cargo build/tests; no platform is claimed passing until its CI job is green. No license choice, release publication or deployment in PR-07.
