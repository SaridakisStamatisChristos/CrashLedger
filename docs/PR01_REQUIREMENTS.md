# R7 requirement and evidence ledger — PR-01

Status is conservative: *implemented* means code exists, not that it was executed.

| ID | Hard requirement | Implementation surface | Executable gate | Current evidence |
| --- | --- | --- | --- | --- |
| R01 | Valid, versioned, closed YAML schema | crashledger-scenario | roundtrip and unknown-field tests | Source authored; CI pending |
| R02 | Loopback-only default and bounded request | validation | remote/invalid/max tests | Source authored; CI pending |
| R03 | No injection for PR-01; pure typed evaluator | crashledger-core | synthetic verdict tests | Source authored; CI pending |
| R04 | Distinct PASS/FAIL/INCONCLUSIVE/ERROR, stable exits | crashledger-core | verdict/exit tests | Source authored; CI pending |
| R05 | CLI validate, invalid inputs exit 4 | crashledger-cli | smoke/negative tests | Source authored; CI pending |
| R06 | Cargo fmt, clippy, workspace tests | .github/workflows/ci.yml | GitHub Actions | UNVERIFIED until green |
| R07 | Owner-selected license only | README | policy review | No license invented |
| R08 | Lockfile checked in for reproducible build | Cargo.lock | cargo generate-lockfile and --locked CI | OPEN: requires Rust toolchain |

Assumptions: schema v1 rejects DNS names even if they currently resolve to loopback, preserving a fail-closed boundary; max_attempts is fixed at 2 as the canonical handoff allows. Credentials, cloud services, auth, distributed execution and production deployment are not applicable to PR-01.

Failure/invalidation rule: a failing test must be repaired and reverified rather than marked green by relaxing the test. Local Rust execution was unavailable at authoring time; check the actual CI result before claiming PR-01 verified.
