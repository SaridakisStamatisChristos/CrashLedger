# CrashLedger v0.1.0 — internally verified release candidate

**Status: open-source source distribution, internal binary-release rehearsal. No public GitHub Release has been published.**

The owner selected **Apache License 2.0** for the first-party CrashLedger code. Source files and Rust package metadata carry that license, and the local release archive includes `LICENSE`, `NOTICE`, `THIRD_PARTY_NOTICES.md`, and `README.md`. The third-party notice inventory is **not yet certified complete**; this remains an explicit prerequisite before publishing binaries.

## Reproduce from a fresh Linux x86_64 checkout

```bash
bash scripts/rc_preflight.sh ./rc-output
```

The preflight builds the committed Cargo.lock workspace with `--release`, checks the CLI's 0.1.0 version, validates scenario v1 and rejects schema v2, executes broken and fixed reference services with the true zero-byte lost-response fault, reads the resulting isolated SQLite state independently, verifies offline Rust and Python reports, screens declared dependency licenses, and packs both binaries and licensing documents into a local tarball with SHA-256 digest.

The release-candidate job in [CI](../.github/workflows/ci.yml) runs only after successful Linux full acceptance and macOS/Windows portable verification. These jobs are *verification* activities, not a public binary distribution or an authenticated artifact provenance statement.

## Acceptance evidence and limits

| Gate | Expected |
| --- | --- |
| Broken demo | FAIL, two orders, exit 2 |
| Fixed demo | PASS, one order, exit 0 |
| Evidence readback | Rust and independent Python verifiers accept both |
| Corruption control | Modified evidence rejected |
| Dependency declarations | First-party Apache-2.0 and no excluded complete third-party alternative |
| Local archive | Two release executables, LICENSE, NOTICE, third-party notices document and README; checksum verifies |

Only the indicated loopback SQLite lost-ack experiment is covered. The redacted snapshot is not replayable and SHA-256 is not cryptographic authentication. Portability for macOS/Windows is scoped to their tested jobs; Linux is the validated candidate packaging target. Cargo.lock pins dependencies, but `rust-toolchain.toml` tracks stable rather than a frozen compiler, so the build is **not** claimed bit-for-bit reproducible.

## Remaining distribution steps

Manual third-party notices/license-text review, packaged executable review and owner approval of the actual GitHub Release remain open. The existing CI artifact stays local to the runner and is not attached to a public Release. No credentials, external network deployment, hosted service or general crash-durability claims are implied.
