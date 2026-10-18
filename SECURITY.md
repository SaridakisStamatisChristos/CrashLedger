# Security policy

CrashLedger v0.1.0 is a **local-only correctness experiment runner**, not a hardened production fault-injection platform. There is currently no published stable release; the `main` branch receives fixes, subject to validation.

## Reporting a vulnerability

**Do not publish working exploits, API tokens, private SQLite data, or sensitive incident evidence in a public issue.** If GitHub private vulnerability reporting is enabled for this repository, use its **Report a vulnerability** option under Security. Otherwise, contact the maintainer privately through a contact method they provide on their public GitHub profile. No private security-response SLA is promised.

For nonsensitive defects, use a public [bug report](.github/ISSUE_TEMPLATE/bug_report.yml) with a minimal, sanitized reproducer.

## Scope and known limits

- Only numeric loopback HTTP/1.1 with bounded Content-Length framing is supported; TLS/HTTP2, remote endpoints and streaming are out of scope.
- A user-authored scenario cannot choose its own SQLite observation file: the CLI requires `--oracle-db` explicitly.
- Evidence manifests detect ordinary corruption but are not digital signatures; a filesystem writer can regenerate all hashes.
- Source scenarios may contain sensitive values in memory; the saved snapshot and events redact payloads and non-Content-Type header values but provide no general-purpose secret manager.
- A PASS proves only the specified invariant for the observed experiment, not generalized exactly-once delivery.

Please supply version/commit, operating system, triggering input and a minimal test case **without private data**.
