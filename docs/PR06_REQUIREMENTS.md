# PR-06 — standalone author-supplied v1 scenarios and falsification

## Contract
- **R06-1:** `crashledger run FILE --oracle-db ABSOLUTE_PATH --out DIR` does not spawn the fixture. It executes a validated, local-only authored scenario with bounded proxy and retry driver.
- **R06-2:** The SQLite file is a separate, explicit CLI selection. YAML may not cause unapproved disk reads. The independent observer remains read-only.
- **R06-3:** Each run creates an isolated `r-...` evidence directory. Both FAIL and INCONCLUSIVE are recorded and reportable; read/write failures never produce PASS.
- **R06-4:** Non-2xx upstream responses do not trigger injection and yield INCONCLUSIVE, not target FAIL; oracle-unavailable similarly yields INCONCLUSIVE; unreachable upstream is an infrastructure ERROR.
- **R06-5:** Two hundred full end-to-end runs (100 broken + 100 fixed) must pass independently and retain their individually reportable artifacts.
- **R06-6:** A fixed fixture resists concurrent same-key requests; malformed, unsupported, oversize or nonlocal scenarios reject.
- **R06-7:** Cancellation/shutdown follows existing PR-03 proxy JoinSet and PR-04 subprocess owner cleanup.

## Acceptance
`cargo test --locked --workspace` executes `crashledger-engine/tests/stress_100.rs` and the CLI black-box integration tests. Separately, CI runs `python3 scripts/test_standalone.py` against a service launched outside the demo lifecycle. The isolated 200-run Rust stress test uses a new fixture database and run directory every cycle, reads the independent SQLite count, and reads back the SHA-256-verified report. Failure must fail CI; not accept 99/100 as success. Tests exercise external service, observer absent, HTTP 500, unreachable upstream, and explicit DB path requirement.

## Limits
No remote target, arbitrary process kill, TLS MITM, production credentials, or general exactly-once claims. The external database must be explicitly selected on the CLI. A newly-created temporary run directory is kept when successful; harness artifacts are not uploaded by default.
