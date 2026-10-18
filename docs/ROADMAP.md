# CrashLedger scope and roadmap

CrashLedger v0.1.0 is an Apache-2.0 licensed, local-first Rust CLI for one controlled experiment: lose a completed HTTP success response, retry identical request bytes, and independently observe the committed SQLite effect. For the implemented behavior, start with the [README](../README.md), [architecture](ARCHITECTURE.md), [correctness contract](CORRECTNESS_CONTRACT.md), [scenario format](SCENARIO_V1.md), and [evidence schema](EVIDENCE_SCHEMA.md).

## Implemented in v0.1.0

- Bounded scenario-v1 validation and loopback-only HTTP/1.1 transport.
- One-shot suppression of a complete successful upstream response, followed by one byte-identical retry.
- Isolated broken and fixed SQLite-backed reference services, plus an independent read-only SQLite scalar observer.
- Four verdicts: PASS, FAIL, INCONCLUSIVE and ERROR.
- Redacted evidence bundles, SHA-256 corruption detection and independent offline verification.
- Rust workspace tests, Linux stress/standalone acceptance tests, portable macOS/Windows CI, and a local Linux x86_64 release-candidate preflight.

These features do **not** establish general exactly-once delivery, crash durability, cryptographic evidence authenticity, or suitability for production traffic. No public GitHub Release has been published.

## Potential next work (not committed)

- Strengthen documentation and examples for running against a separately managed local service.
- Investigate additional fault models only with explicit correctness invariants, negative controls and regression tests.
- Complete the manual third-party license-text and notices review required before owner-approved binary publication.

These are possible directions, not promises of features or a release schedule. See the [release-candidate checklist](RELEASE_CANDIDATE.md) and [dependency policy](DEPENDENCY_POLICY.md) for outstanding publication gates.

## Historical engineering records

The eight milestone acceptance ledgers (`PR01_REQUIREMENTS.md` through `PR08_REQUIREMENTS.md`) remain in this directory for traceability. They describe the development history; they are **not** current setup instructions, a pending implementation handoff, or evidence that checks passed on a later commit. The current code, documentation and head-specific CI results govern present behavior.
