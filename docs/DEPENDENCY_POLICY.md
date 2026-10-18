# Apache-2.0 first-party and third-party dependency policy

**CrashLedger first-party source code is licensed under Apache-2.0.** See [LICENSE](../LICENSE) and [NOTICE](../NOTICE). The copyright owner has selected the project license; the earlier no-license restriction is superseded.

This does **not** relicense upstream crates, remove third-party obligations, confer certification, or automatically authorize publication of an official binary release.

## Automated gate

```bash
python3 scripts/test_license_policy.py
python3 scripts/audit_dependencies.py dependency-audit.json
```

The dependency inventory uses `cargo metadata --locked --offline` where possible, with locked-registry fallback when a fresh runner lacks target-specific metadata. It checks first-party packages for `Apache-2.0` metadata, lists the resolved third-party crates and SPDX declarations, and evaluates basic AND/OR/WITH alternatives. It fails when all detected complete license alternatives contain the currently excluded AGPL, GPL, LGPL, SSPL or EUPL terms. A permitted OR branch (such as `MIT OR LGPL-3.0`) can pass the metadata screen. Missing/unsupported expressions require manual review. This is a deliberately conservative project policy, not a statement that all copyleft dependencies are necessarily illegal to distribute.

The JSON result sets `compliance_certified: false` by design: a declaration-only screen cannot prove notice fulfillment, bundled-source obligations, redistribution rights, or full legal compliance. The compiler may bundle SQLite through rusqlite/libsqlite3-sys; its applicable notices and other native dependencies must be reviewed before any official binary publication.

## Official release checklist

1. Confirm first-party `LICENSE`, `NOTICE` and Apache-2.0 Cargo metadata.
2. Produce the locked, versioned dependency inventory; resolve every `needs_manual_review` entry.
3. Examine actual third-party license texts, bundled native code and required notices/attributions; update [THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md) accordingly.
4. Confirm artifacts include the Apache license, owner notice, third-party documentation and checksums.
5. Perform a release-specific compatibility and security review; publish only through a separately approved release operation.

The internal release-candidate CI exercises the metadata and packaging gates, but steps 2–5 are not automatically certified or treated as complete.
