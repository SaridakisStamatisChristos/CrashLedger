# Open-source productization review — October 2026

**Historical engineering record.** This review describes the state and planned acceptance checks during the October 2026 licensing/documentation work. It is not a current-head CI attestation, a new release announcement, or a substitute for examining the present [workflow runs](https://github.com/SaridakisStamatisChristos/CrashLedger/actions/workflows/ci.yml).

The productization change selects Apache License 2.0 for **first-party source**, aligns all Cargo packages, and updates documentation and CI to prevent false "no license" claims. It does not create a public release, claim third-party legal compliance, or expand the v0.1 failure model.

## Implemented contract and evidence gates

| ID | Requirement | File/gate | Status condition |
| --- | --- | --- | --- |
| P01 | Unmodified Apache-2.0 license text | LICENSE, scripts/check_product_hygiene.py | CI passes text/length/Appendix check |
| P02 | Copyright notice and eight Rust packages declare Apache-2.0 | NOTICE, Cargo.toml manifests | workspace metadata gate passes |
| P03 | Accurate feature scope, quick start and CLI verdict semantics | README, scripts/demo.sh | README links and live demo |
| P04 | Contributors, support, security, conduct, issue/PR templates | root docs, .github | file/link hygiene gate |
| P05 | Locked Cargo dependency inventory and first-party license check | scripts/audit_dependencies.py, scripts/test_license_policy.py | CI preflight / policy tests |
| P06 | License, owner notice, THIRD_PARTY_NOTICES.md and README inside Linux RC archive | scripts/rc_preflight.sh | tar listing and checksum gate |
| P07 | No claim of certified third-party rights or release publication | docs/DEPENDENCY_POLICY.md, docs/RELEASE_CANDIDATE.md | documentation review |
| P08 | CI remains green on Linux/macOS/Windows and clean-checkout RC | .github/workflows/ci.yml | current-head workflow success |

## Explicit gaps

- **Third-party notices:** metadata declarations and SPDX parsing are a filter, not a substitute for collecting and reviewing applicable full license texts and required notices before official binary distribution.
- **GitHub About metadata:** at the time of audit, repository description and topics were empty. Editing GitHub repository settings is separate from committing files; suggested description: *Local-first Rust fault-injection experiments with independent SQLite correctness oracles and verifiable evidence.* Suggested topics: `rust`, `fault-injection`, `reliability`, `sqlite`, `idempotency`, `testing`.
- **Published artifacts:** v0.1 CI produces a local candidate tarball but does not publish a GitHub Release. Code is licensed; signed binaries, tags, release automation and support SLAs remain unclaimed.
- **Scope:** no remote targets or TLS/HTTP2, no generic exactly-once or power-loss durability proof, no user authentication or hosted control plane. Not applicable to current standalone local architecture.

No claim of SOTA performance, commercial readiness, legal certification or reproducible bit-for-bit binaries is made.
