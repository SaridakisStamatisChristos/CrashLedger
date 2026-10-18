# CrashLedger

CrashLedger is a **local-first correctness experiment runner** for controlled failures and independently checked postconditions, implemented in Rust.

## Current scope — PR-05

PR-01 provides the workspace, strict scenario validator and pure verdict evaluator. PR-02 adds a **standalone, independently tested SQLite-backed HTTP order fixture** with intentionally broken and transactionally fixed idempotency modes. PR-03 adds a bounded loopback HTTP/1.1 one-shot suppression proxy and an identical-byte retry driver. PR-04 adds the experiment engine and a direct read-only SQLite observer. The full lost-response demonstration is now available from the CLI; PR-05 persists versioned, manifest-verified evidence and adds an offline report command that reconstructs the verdict.

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

## HTTP/1.1 transport fault and retry — PR-03

The `crashledger-http` library exposes `Proxy::start`, `retry_post` and typed transport events for orchestration by the future engine. The proxy binds an OS-selected localhost port and forwards bounded POST requests to a configured loopback upstream. It fully buffers a successful 2xx response and, for the **first** exact path match only, closes the downstream socket **without writing response bytes**. After this transport failure, the driver makes one retry with **identical body and key bytes**. Ordinary 4xx/5xx replies are forwarded and do not trigger retries. A missed fault does not imply application correctness.

It is deliberately HTTP/1.1 only: Content-Length framing is required, transfer encoding/chunked streams and protocol upgrades are rejected, and no remote upstream is accepted. All worker tasks are joined or cancelled on `shutdown`. The library records events but **does not decide PASS/FAIL**. Run `cargo test --locked --workspace` to exercise the proxy against a controlled TCP upstream.

## End-to-end demo (PR-04)

The CLI owns an isolated SQLite fixture subprocess and a loopback fault proxy. It runs the **same** canonical lost-ack scenario against the intentionally broken and transactionally idempotent order modes. The client observes a transport failure after the first upstream success, then retries identical request bytes. Once proxy tasks have quiesced, a **separate read-only SQLite connection** executes a bound scalar query. The independent observed row count decides the verdict only after fault and attempt preconditions are met.

~~~bash
cargo run --locked -p crashledger-cli -- demo --mode broken
# Expected FAIL, two orders, exit 2 (the deliberate negative control)
cargo run --locked -p crashledger-cli -- demo --mode fixed
# Expected PASS, one order, exit 0
~~~

To use a different output root, pass `--artifacts-dir ./path` to `demo`. Each run creates a unique subdirectory containing the fresh isolated fixture SQLite database. PR-05 adds versioned, bounded JSONL events, metadata, redacted snapshot, separate oracle observation and verdict, SHA-256 manifest, and offline verification.

Expected broken and fixed verdicts are scoped to the tested request, failure model and independent observer. They are **not** proof of universal exactly-once processing, arbitrary target commit durability, or general crash safety.


A bounded HTTP/1.1 proxy will suppress **one** successful, fully received order response without sending downstream response bytes. The driver retries the identical request. A separate **read-only** SQLite oracle then checks order rows. Against the deliberately broken fixture the experiment must yield FAIL and two rows; against the transactional idempotent fixture it must yield PASS and one row. A missed fault or unavailable observer must never be classified PASS.

## Persistent evidence and standalone reporting — PR-05

Each successful experiment execution, including expected FAIL and available INCONCLUSIVE cases, creates six files under its isolated artifacts/r-... directory: scenario.snapshot.yaml (redacted and explicitly non-replayable), run.json, events.jsonl, observation.json, verdict.json, and manifest.sha256. The writer creates the manifest last and refuses to overwrite files; persistence failure results in exit 4. Event records contain causal and HTTP status metadata, never raw credentials, response bodies, request bodies, or SQL parameter values.

Inspect a prior run **without starting a target or rerunning**:

~~~bash
cargo run --locked -p crashledger-cli -- report ./artifacts/r-EXAMPLE --format text
cargo run --locked -p crashledger-cli -- report ./artifacts/r-EXAMPLE --format json
python3 scripts/verify_bundle.py ./artifacts/r-EXAMPLE
~~~

Replace r-EXAMPLE with the actual run directory printed by demo. The Rust reporter rejects corrupted/missing/invalid artifacts, checks the SHA-256 manifest, and recomputes the core verdict from the recorded facts. A trustworthy FAIL report exits **0** because reporting succeeded; broken demo itself still exits **2**. A corrupt bundle or report infrastructure failure exits **4**.

**Evidence scope:** SHA-256 detects accidental corruption, not a malicious author who can regenerate a whole bundle. The redacted snapshot is not replayable. The independent Python verifier uses only the standard library.

## Licensing

No license is granted by this repository at present. A project license requires the owner's explicit selection.
