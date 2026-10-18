# CrashLedger

[![Rust CI](https://github.com/SaridakisStamatisChristos/CrashLedger/actions/workflows/ci.yml/badge.svg)](https://github.com/SaridakisStamatisChristos/CrashLedger/actions/workflows/ci.yml)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

**Reproduce a lost HTTP acknowledgment. Verify the real database effect. Keep inspectable evidence.**

CrashLedger is a local-first Rust CLI for a narrow but important reliability experiment: **what happens when an order commits successfully, its HTTP response disappears, and the client retries the same request?**

It does not guess whether an HTTP request succeeded. A controlled fault proxy discards one complete successful response, the client retries with identical bytes, and a separate read-only SQLite connection checks what actually committed.

> **Scope:** v0.1.0 is an engineering demonstrator for loopback HTTP/1.1, an SQLite scalar observer, and a single one-shot fault model—not a universal exactly-once guarantee, production fault-injection service, or general crash verifier. No GitHub Release has been published.

## In two commands

Requires [Rust](https://rustup.rs/) (stable toolchain), Python 3 and Bash on Linux or macOS. The additional license/documentation checks require **Python 3.11+**. Clone the repository, then run:

```bash
bash scripts/check.sh
bash scripts/demo.sh
```

The demo creates fresh broken and fixed reference services. Both experiments run through **the same proxy, retry driver, independent observer and evaluator**:

| Reference mode | Committed orders for one key | Verdict | CLI exit |
| --- | ---: | --- | ---: |
| **Broken** — writes a new order on each retry | 2 | **FAIL** (expected negative control) | 2 |
| **Fixed** — transactional idempotency/replay | 1 | **PASS** | 0 |

The demo script treats the deliberate broken-service exit code 2 as an *expected test result* and only exits 0 when both cases and their evidence checks succeed. Run directories are printed and retained in a temporary output root.

## CLI

Build both executables from committed dependencies:

```bash
cargo build --locked --release -p crashledger-cli -p crashledger-demo
```

| Operation | Example |
| --- | --- |
| Validate a scenario without contacting the target | `cargo run --locked -p crashledger-cli -- validate scenarios/lost-ack.yaml` |
| Run the intentionally broken demo | `cargo run --locked -p crashledger-cli -- demo --mode broken` |
| Run the idempotent demo | `cargo run --locked -p crashledger-cli -- demo --mode fixed` |
| Verify a saved experiment without replaying it | `cargo run --locked -p crashledger-cli -- report artifacts/r-REAL-ID --format json` |

A `report` on a valid **FAIL** evidence bundle exits 0 because inspection succeeded; the underlying `demo` exits 2. A missing or corrupt report exits 4.

### Against your own local test service

Start a compatible, **separately managed loopback** HTTP service. Copy `scenarios/lost-ack.yaml` and change `target.base_url` to its numeric `127.0.0.1:<port>` URL. Supply the observed SQLite file **separately**, using an explicitly chosen absolute path:

```bash
cargo run --locked -p crashledger-cli -- run scenarios/my-local.yaml \
  --oracle-db /absolute/path/to/observed.sqlite \
  --out artifacts
```

The standalone runner does not start or stop that service, and the YAML file cannot grant itself database access. HTTP response loss is injected only after a complete matching successful upstream response. Unsupported, remote, oversized, or malformed scenarios are rejected. See [scenario v1](docs/SCENARIO_V1.md) and [the correctness contract](docs/CORRECTNESS_CONTRACT.md).

## How a verdict is earned

```text
  Client ─POST──> Loopback fault proxy ──POST──> SQLite-backed order service
  Client <──EOF── One completed HTTP 2xx is suppressed with zero reply bytes
  Client ─RETRY─> Loopback fault proxy ──POST──> Same service / same request bytes

  After quiescence: Independent READ_ONLY SQLite query → integer row count
                    Fault + retry facts + independent scalar → verdict
                    Evidence writer → redacted JSONL, JSON, SHA-256 manifest
                    Offline Rust / Python readers → integrity + verdict checks
```

| Verdict | Meaning | Exit |
| --- | --- | ---: |
| `PASS` | Required fault and retry were observed; specified invariant held | 0 |
| `FAIL` | Required fault and retry were observed; invariant violated | 2 |
| `INCONCLUSIVE` | A required fault, retry, final success, or observation is missing | 3 |
| `ERROR` | Experiment, transport infrastructure, or evidence persistence failed | 4 |

The evidence bundle contains a redacted, **non-executable** scenario snapshot, run metadata, ordered events, independent scalar observation, stored verdict and a SHA-256 manifest written last. Offline readers validate hashes and recompute the verdict; SHA-256 is **not an authenticated signature**.

## Architecture and evidence

CrashLedger is a small Rust workspace with independent crates for scenarios, HTTP fault transport, SQLite observations, run orchestration, pure verdict evaluation and evidence/reporting.

- [Architecture diagrams and module boundaries](docs/ARCHITECTURE.md)
- [Correctness / verdict contract](docs/CORRECTNESS_CONTRACT.md)
- [Threat model](docs/THREAT_MODEL.md)
- [Evidence v1 format](docs/EVIDENCE_SCHEMA.md)
- [Compatibility evidence](docs/COMPATIBILITY.md)
- [Roadmap](docs/ROADMAP.md)

CI checks Rust formatting, Clippy, the workspace tests and 100 fresh broken/fixed pairs on Linux; macOS and Windows build and run their portable test matrix. The clean-checkout Linux release-candidate gate validates demos, reports and a local binary archive. See [release-candidate notes](docs/RELEASE_CANDIDATE.md).

## Troubleshooting

The first build downloads locked Cargo dependencies. If `--locked` reports a dependency resolution error, check your Rust toolchain and network access rather than deleting `Cargo.lock`. The reference demo requires available local loopback ports and writes evidence to the temporary directory printed by the script. A deliberate `FAIL`/exit 2 for the broken service is expected; the wrapper script succeeds only when both controls and evidence checks pass.

## Contributing, security and support

Bug reports and small reproducible improvements are welcome. Read [CONTRIBUTING](CONTRIBUTING.md), [security reporting](SECURITY.md), [support](SUPPORT.md), and [the code of conduct](CODE_OF_CONDUCT.md). Do **not** submit live secrets, private databases, or unredacted incident logs.

## License and dependencies

CrashLedger's first-party source code is licensed under the **[Apache License, Version 2.0](LICENSE)**. Copyright 2026 Stamatis-Christos Saridakis. See [NOTICE](NOTICE).

Third-party dependencies retain their own licenses. [Third-party notices and review status](THIRD_PARTY_NOTICES.md) and the [dependency policy](docs/DEPENDENCY_POLICY.md) distinguish an automated metadata screen from a completed legal/notices review. The code is open source; publishing official binary releases remains a separate release decision.
