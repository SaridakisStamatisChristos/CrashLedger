# PR-01 — foundation requirements and verification

This ledger records the **PR-01 scope** and the repository's current acceptance evidence. PR numbers identify normal implementation milestones; the requirement IDs are retained for traceability. A green gate is based on executed CI, not merely the presence of source code.

| ID | Requirement | Implementation surface | Executable gate | Verification |
| --- | --- | --- | --- | --- |
| R01 | Valid, versioned, closed YAML scenario schema | `crashledger-scenario` | Parsing, roundtrip and unknown-field tests | Verified in workspace CI |
| R02 | Loopback-only target and bounded request | Scenario validation | Remote, invalid and size-bound rejection tests | Verified in workspace CI |
| R03 | Pure typed evaluator independent of fault injection | `crashledger-core` | Synthetic verdict tests | Verified in workspace CI |
| R04 | Distinct PASS / FAIL / INCONCLUSIVE / ERROR and stable exit codes | `crashledger-core` | Verdict and exit-code tests | Verified in workspace CI |
| R05 | Scenario validation CLI; invalid inputs exit 4 | `crashledger-cli` | CLI smoke and negative tests | Verified in workspace CI |
| R06 | Format, Clippy, locked workspace test gates | `.github/workflows/ci.yml` | GitHub Actions on Linux | Verified |
| R07 | Licensing must follow an explicit owner decision | `LICENSE`, `NOTICE`, workspace `Cargo.toml` | Apache-2.0 metadata and document validation | Implemented in the subsequent open-source productization change and verified in CI |
| R08 | Committed dependency lockfile and repeatable locked resolution | `Cargo.lock` | `cargo --locked` build, lint and tests | Verified in CI |

## Design decisions

Scenario v1 rejects DNS target names even when they currently resolve to loopback, preserving a fail-closed local networking boundary. The canonical lost-ack experiment uses exactly two attempts. Authentication, cloud services, distributed execution and production deployment are not part of the foundational CLI scope.

## Evidence and history

The original PR-01 requirements and tests were accepted when that milestone merged. The current repository independently rechecks those gates, as well as the end-to-end broken/fixed controls, in [Rust CI](../.github/workflows/ci.yml). See the [successful post-productization workflow](https://github.com/SaridakisStamatisChristos/CrashLedger/actions/runs/39117257580).

Apache-2.0 licensing was selected and merged **after** PR-01; the R07 row reflects the current repository state without implying that the license was part of the original foundation milestone. Requirements are preserved; historical pending-CI annotations are superseded by the executed verification above. A failing gate must be repaired and rerun, not made green by weakening its assertions.
