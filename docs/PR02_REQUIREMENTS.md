# PR-02 — standalone SQLite fixture correctness contract

## Hard requirements

- F01: HTTP/1.1 POST /orders with JSON sku and positive integer quantity plus Idempotency-Key. Both modes use the same API.
- F02: Broken mode deliberately commits a **new** order for every identical request. Two requests must yield two committed SQLite rows.
- F03: Fixed mode uses a SQLite BEGIN IMMEDIATE transaction for key lookup, order insert, and ledger insert. Identical retry returns the committed order_id without inserting another row.
- F04: Conflicting key reuse with a changed raw JSON payload returns 409 and never creates another order.
- F05: Fixed mode concurrent identical-key requests produce exactly one order and one ledger record.
- F06: HTTP success is emitted only after the database transaction commits, and the independent read-only SQLite observer verifies committed state rather than accepting the service's own response as an oracle.
- F07: The fixture listens on loopback only, rejects malformed JSON and keys, and caps requests at 65536 bytes.
- F08: The fixture has no dependency on CrashLedger core/runner/proxy, and each integration test uses a fresh temporary DB and owned child process.
- F09: The workspace passes rustfmt, clippy, locked cargo test, and reproducible Cargo.lock CI.

## Implementation

The standalone binary is crashledger-demo, in fixtures/order-service. Migration 001_init.sql intentionally does not make orders.idempotency_key unique; instead the fixed path uses the unique idempotency_requests table with a foreign key to orders. The key's request fingerprint uses SHA-256 of method, route and the exact JSON body bytes, not semantic normalization. SQLite busy_timeout permits concurrent key contenders to serialize under BEGIN IMMEDIATE.

The GET /ready endpoint confirms schema availability but is not a business correctness claim. PR-02's independent read-only connection is a **test observer**; the orchestrated oracle belongs to PR-04. There is no process-kill or response-suppression fault in PR-02.

## Adversarial verification

Black-box integration tests launch the actual process and use raw HTTP and a separate read-only SQLite connection for:
- broken duplicate negative control;
- fixed create/replay identity and count;
- changed-payload conflict preserving count;
- twelve concurrent identical-key HTTP calls;
- missing key, invalid JSON, invalid sku/quantity and extra fields;
- oversized request and nonlocal listener rejection.

F01–F09 are VERIFIED only when the associated tests pass on the current head. Tests assert both HTTP semantics and independent row counts. No universal exactly-once guarantees or crash-durability claims are made.

## Applicability and limitations

No auth, cloud, dashboard, distributed agents, user accounts or network adversary: N/A to this local test fixture. User-provided database path is deliberately explicit; integration tests always allocate fresh isolated DBs and stop owned processes. No release/license change is part of this PR.
