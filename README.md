# CrashLedger

CrashLedger is a **local-first correctness experiment runner** for controlled failures and independently checked postconditions, implemented in Rust.

## Current scope — PR-02

PR-01 provides the workspace, strict scenario validator and pure verdict evaluator. PR-02 adds a **standalone, independently tested SQLite-backed HTTP order fixture** with intentionally broken and transactionally fixed idempotency modes. CrashLedger still does **not** inject a response fault, orchestrate the complete experiment or produce a runtime verdict; those milestones begin in PR-03/04.

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

## Standalone fixture — PR-02

Start a local HTTP order server with a fresh DB. The server prints READY plus its OS-selected loopback origin after schema initialization and listener binding:

~~~bash
cargo run --locked -p crashledger-demo -- --mode broken --db-path ./run-data/orders-broken.sqlite
cargo run --locked -p crashledger-demo -- --mode fixed --db-path ./run-data/orders-fixed.sqlite
~~~

Send POST /orders with Content-Type: application/json, Idempotency-Key: demo-order-0001, and JSON body {"sku":"widget-42","quantity":1}. Broken returns HTTP 201 on every identical request. Fixed returns HTTP 201 for first creation and HTTP 200 with the same order_id on identical replay. Same key plus a changed body yields HTTP 409. GET /ready tests that SQLite tables can be queried; it is **not** an application correctness oracle.

For actual verification, workspace tests spawn each mode as a real process and check committed order counts through a **separate SQLite read-only connection**. Run cargo test --locked --workspace.

## End-to-end objective (not implemented yet)

A bounded HTTP/1.1 proxy will suppress **one** successful, fully received order response without sending downstream response bytes. The driver retries the identical request. A separate **read-only** SQLite oracle then checks order rows. Against the deliberately broken fixture the experiment must yield FAIL and two rows; against the transactional idempotent fixture it must yield PASS and one row. A missed fault or unavailable observer must never be classified PASS.

## Licensing

No license is granted by this repository at present. A project license requires the owner's explicit selection.
