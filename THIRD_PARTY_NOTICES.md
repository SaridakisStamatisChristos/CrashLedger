# Third-party dependency licensing and notices

**Status: inventory available; manual license-text and notices review is still required before an official redistributed binary release.** This document is not a certificate of license compliance, and it does not replace applicable third-party license texts.

CrashLedger first-party files use [Apache-2.0](LICENSE) and the corresponding [NOTICE](NOTICE). Cargo dependencies—including any bundled SQLite sources—remain copyright of their respective upstream authors and are **not** relicensed by the CrashLedger license.

Run the dependency declaration inventory from a checked-out workspace:

```bash
python3 scripts/audit_dependencies.py dependency-audit.json
```

The output records crate name, version, declared SPDX license, optional license_file, source, possible excluded copyleft and items needing human review. The release-candidate preflight writes its copy to `rc-output/dependency-audit.json`. The audit is locked to Cargo.lock, prefers offline metadata, and may retrieve missing locked registry metadata on fresh CI runners.

**Before distributing official compiled binaries:** inspect third-party license texts and attribution/notice requirements for every shipped dependency and bundled library, resolve ambiguous metadata, add any required notices/source offers or corresponding material, and review the resulting archive. This is a manual legal/compliance gate that a passing CI metadata scan does not discharge.

See [docs/DEPENDENCY_POLICY.md](docs/DEPENDENCY_POLICY.md). No commercial warranty, trademark license, or certification is granted by this document.
