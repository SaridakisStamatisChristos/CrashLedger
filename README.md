# CrashLedger

CrashLedger is a **local-first correctness experiment runner** for controlled failures and independently checked postconditions, implemented in Rust.

## Current scope — PR-01

This first milestone provides a Cargo workspace, a closed/versioned YAML scenario model, strict validation, a pure verdict evaluator, a lifecycle state machine, and a CLI validation command. **It does not yet inject HTTP faults, launch an order fixture, query SQLite, produce a real experiment verdict, or run an end-to-end demo.** Those milestones follow in PR-02 through PR-08. Nothing in PR-01 demonstrates exactly-once delivery or application crash safety.

Requirements and the incremental roadmap are in [docs/ROADMAP.md](docs/ROADMAP.md). The scenario source contract is in [docs/SCENARIO_V1.md](docs/SCENARIO_V1.md).

## Build and validate

Requires a working Rust toolchain; no Node.js, Docker, external service or cloud account is needed.

~~~bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo run --locked -p crashledger-cli -- validate scenarios/lost-ack.yaml
~~~

Expected success: VALID plus scenario ID and bound configuration, exit 0. An invalid or unreadable scenario exits 4 with an error. This CLI does not contact the target during validation.

## End-to-end objective (not implemented yet)

A bounded HTTP/1.1 proxy will suppress **one** successful, fully received order response without sending downstream response bytes. The driver retries the identical request. A separate **read-only** SQLite oracle then checks order rows. Against the deliberately broken fixture the experiment must yield FAIL and two rows; against the transactional idempotent fixture it must yield PASS and one row. A missed fault or unavailable observer must never be classified PASS.

## Licensing

No license is granted by this repository at present. A project license requires the owner's explicit selection.
