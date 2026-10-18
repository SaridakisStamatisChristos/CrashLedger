# Changelog

## Unreleased

- Refresh product-facing scenario/roadmap documentation, fix stale acceptance references, clarify prerequisites, and prevent internal agent/handoff terminology from returning to current user docs.
## Earlier unreleased productization work (historical)

- Apply canonical Apache License 2.0 to first-party source and all Cargo packages.
- Add NOTICE, third-party notice-review documentation, community/security/support policies and standard issue/PR templates.
- Reorganize README for quick start, explicit exit codes, evidence usage and scoped guarantees.
- Include licensing documents in the internal binary candidate archive; strengthen license metadata and packaging gates.
- Retain the external third-party license-text/notice review as a manual official-release blocker.

## v0.1 release candidate preparation

- PR-01: Rust workspace, closed scenario contract and four verdicts.
- PR-02: SQLite broken/fixed reference fixture, transactional idempotency and black-box tests.
- PR-03: Bounded one-shot zero-byte HTTP response suppression and retry.
- PR-04: Independent read-only SQLite oracle and real broken/fixed end-to-end demonstrations.
- PR-05: Redacted versioned evidence, SHA-256 manifests and offline reports.
- PR-06: User-authored local scenario runs and 100 isolated broken/fixed test pairs.
- PR-07: Cross-platform CI, architecture decisions and quality/safety documentation.
- PR-08: Linux x86_64 clean-checkout v0.1 candidate rehearsal, offline dependency declaration audit, local binary packaging and SHA-256 digest.

These entries are implementation milestones, not an announcement of a published GitHub Release. First-party CrashLedger code is licensed under Apache-2.0.
