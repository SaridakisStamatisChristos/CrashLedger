# CrashLedger v0.1.0 — candidate release notes

**Source is Apache-2.0 licensed; official binaries and GitHub Release not yet published.**

CrashLedger is a local-first Rust CLI for testing one specific idempotency failure: an order commits, its HTTP acknowledgement disappears, and a byte-identical retry is sent. The tool verifies the resulting order count independently through SQLite, giving a four-valued PASS/FAIL/INCONCLUSIVE/ERROR verdict.

**Highlights:** a strict scenario-v1 schema; independently hosted broken and fixed order services; bounded loopback HTTP/1.1 post-response suppression; transactional idempotency negative/positive controls; read-only SQLite scalar oracle; versioned, redacted JSONL/JSON evidence with SHA-256 manifest; offline Rust and Python verifiers; 100 fresh broken/fixed pairs on Linux; portable Cargo builds/tests on macOS and Windows; Apache-2.0 licensing and community contributions guidance.

**Canonical demo:** broken service → FAIL, two orders, CLI exit 2. Fixed service → PASS, one order, CLI exit 0.

**Known limits:** not a generic exactly-once, process-kill/crash-durability or distributed-fault proof. No remote targets, TLS/HTTP2 or streaming faults. Evidence hashes detect ordinary corruption but are not signatures. No hosted service, SLA or official binary release is claimed. Third-party notice/legal review remains required before distributing official compiled artifacts.

See [README](../README.md), [LICENSE](../LICENSE), [THIRD_PARTY_NOTICES](../THIRD_PARTY_NOTICES.md), and [release candidate](RELEASE_CANDIDATE.md).
