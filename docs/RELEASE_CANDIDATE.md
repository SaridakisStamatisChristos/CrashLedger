# CrashLedger 0.1.0 — internal release candidate

**Status: candidate preflight, not a published release.** A GitHub tag or Release has deliberately **not** been created. The owner has not selected a license, so public distribution/licensing is unresolved.

## Target user and test

The first deliverable is a local correctness experiment tool for engineers who need to reproduce a lost post-commit HTTP response and observe whether a retry duplicates an SQLite order. It is not an arbitrary crash-fault verifier.

The clean-checkout script accepts a new output directory:

~~~bash
bash scripts/rc_preflight.sh ./rc-output
~~~

It runs cargo build --release --locked --workspace and validates:
- crashledger --version is 0.1.0 and schema_version 2 is rejected with exit 4;
- canonical broken demo: suppressed successful first response, retry, SQLite count 2, FAIL, exit 2;
- canonical fixed demo: identical retry, SQLite count 1, PASS, exit 0;
- both offline Rust JSON reports and external Python SHA-256/causal verifier agree;
- packaged Linux x86_64 tarball contains crashledger and crashledger-demo, and SHA256SUMS checks;
- locked metadata-based dependency license screen has no prohibited declarations; registry metadata may be fetched if the fresh runner lacks uncached platform packages.

Generated outputs under rc-output: scenario logs, broken/fixed console transcripts, reports, version, dependency-audit.json, unique sample artifact bundles, Linux tarball and SHA256SUMS. Output is **private/local to the execution environment**. No implicit upload, tag creation, external deploy or release publication.

## Scope limitations

Evidence is redacted and SHA-256 checked against accidental corruption, not cryptographically signed. Scenario snapshot is non-executable. Runtime verdicts are limited to the stated invariant/fault/observer on one local run. Windows/macOS build/test gates do not imply Linux packaging guarantees extend to those platforms, and do not execute the 100-pair stress suite.

## Public release blockers

1. Owner must explicitly select and document project licensing.
2. Third-party declarations and notice/license obligations require manual review; metadata screen is not a legal compliance sign-off.
3. If binaries are publicly distributed, add the appropriate license and third-party notices and verify platform packages, checksums and provenance.
4. Optional: signed artifacts and a documented release workflow; neither is required for this *internal candidate* milestone.

After these decisions, a separate owner-authorized action may publish a GitHub tag/release. The PR-08 merge itself does not publish anything.
