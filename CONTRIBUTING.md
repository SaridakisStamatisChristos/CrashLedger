# Contributing to CrashLedger

Thank you for improving the experiment. This is a narrow, local-first correctness demonstrator, so prefer small PRs that strengthen invariants, reproducibility, or independent verification over speculative features.

## Getting started

1. Open a focused issue or PR describing the failure mode and the expected independent observation. For security reports, use [SECURITY.md](SECURITY.md) instead of public issues.
2. Work on a branch based on `main`. Keep unrelated refactors out of correctness-sensitive changes.
3. Run `bash scripts/check.sh` and `bash scripts/demo.sh` on Linux/macOS before submitting; CI also runs macOS and Windows portable checks.
4. Add both a positive case and a falsifier, particularly around fault timing, idempotency, persistence, race conditions, redaction, and verdict gates.

The Python-based licensing and documentation gates below require Python **3.11 or newer** (`tomllib` is part of its standard library).

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
python3 scripts/test_license_policy.py
python3 scripts/check_product_hygiene.py
```

Tests must preserve the golden negative control (broken: two orders, FAIL, exit 2) and fixed control (one order, PASS, exit 0). Never convert missing evidence into PASS, weaken assertions to force green CI, or claim a general exactly-once proof.

Schema v1, evidence v1, HTTP transport and exit-code changes require explicit compatibility documentation. Keep the project's local-only trust boundaries. Do not commit SQLite run state, secrets, logs, build output or local evidence bundles.

## Contributions and licensing

**First-party code is Apache-2.0 licensed.** By submitting a contribution, you represent that you have the right to license that contribution under Apache-2.0 and agree that accepted contributions are licensed under those terms. Retain third-party attribution and do not add code under incompatible terms. Document new dependencies, license declarations and any notice obligations. See [LICENSE](LICENSE), [NOTICE](NOTICE), [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md), and [docs/DEPENDENCY_POLICY.md](docs/DEPENDENCY_POLICY.md).

Contributions are reviewed by the maintainers; a pull request does not guarantee acceptance.
