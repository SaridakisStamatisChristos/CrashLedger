# CrashLedger v0.1.0 release notes — candidate draft

**Not published — draft for internal release candidate evaluation.**

CrashLedger is a local-first Rust CLI that demonstrates detection of a duplicate-order invariant violation after a single intentionally suppressed post-commit HTTP response. It runs a canonical broken and fixed order fixture, retries an identical request, observes SQLite state independently, and persists verifiable evidence bundles.

## Included

- Closed/versioned v1 scenario schema and strict loopback-only HTTP/1.1 transport.
- SQLite broken/fixed reference fixture with fixed transactional idempotency and an independent read-only scalar oracle.
- One-shot zero-byte response-suppression fault, typed transport evidence and four-valued PASS/FAIL/INCONCLUSIVE/ERROR verdict contract.
- Golden demo: broken FAIL/two rows/exit 2, fixed PASS/one row/exit 0.
- Authored separately-started local targets via run --oracle-db ABSOLUTE_PATH.
- Versioned redacted JSONL/JSON evidence with SHA-256 manifest and offline Rust/Python verifiers.
- Linux 100-pair stress, macOS and Windows portable build/integration CI, architecture/threat model/ADRs.

## Exclusions

Not a proof of generic exactly-once execution, process crash safety, distributed fault tolerance, TLS/HTTP2, remote testing, signed evidence, or production SaaS. No public license has been selected or granted. Do not redistribute candidate binaries without a license decision and third-party notice review.
