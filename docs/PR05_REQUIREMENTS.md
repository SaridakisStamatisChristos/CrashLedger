# PR-05: evidence and offline reports

Source: CrashLedger architecture handoff section 9 and PR-05.

| ID | Hard requirement | Owner | Acceptance |
| --- | --- | --- | --- |
| E01 | Versioned six-file evidence bundle with final SHA-256 manifest | evidence/writer | broken/fixed real runs |
| E02 | Monotonic JSONL with fault/retry/oracle causal context | evidence/writer | Rust and Python reader |
| E03 | Run metadata: run ID, schema, tool/platform, timeouts, input digest, optional commit | run.json | schema validation |
| E04 | Redact non-Content-Type headers, body and bound oracle parameters | snapshot writer | canary secret tests |
| E05 | Persist independent observed scalar and verdict for FAIL and INCONCLUSIVE | observation.json/verdict.json | Rust roundtrip |
| E06 | Offline text/json reporter verifies hashes, chronology and recomputes verdict | evidence/report + CLI | real report subprocess |
| E07 | Missing, corrupted, oversize, reordered and forged verdict artifacts rejected | report | adversarial tests |
| E08 | Required writer failure produces ERROR / exit 4 | engine | create_new conflict test |
| E09 | Independent Python 3 standard-library verifier accepts both real demos | scripts/verify_bundle.py | CI |
| E10 | fmt, clippy, tests, demos and report pass with committed lockfile | GitHub CI | final-head green |

## Contract

Each artifacts/<run-id>/ contains scenario.snapshot.yaml, run.json, events.jsonl, observation.json, verdict.json and manifest.sha256 (written last). run-data/ contains the live fixture SQLite file but is not listed in the manifest. Event records have seq, run_id, kind, optional attempt, HTTP status and observed scalar. SHA-256 lines list exactly the five content files in fixed order. Reader enforces bounded sizes, no symlinks, hash equivalence, strict v1 records, monotonic sequences, causal prerequisites, and rederived verdict matching the saved claim.

The YAML scenario snapshot is deliberately **redacted and not executable**, to avoid leaking arbitrary header/body/parameter values. The SHA-256 digest of the unredacted validated scenario is recorded in run.json to allow later identification of an independently retained source. No full bodies, cookies, authorization headers, or SQL parameter literals appear in the JSONL events.

## Limits

An ordinary SHA-256 manifest detects corruption but is not a signature or tamper-proof attestation. The local read-only oracle is previously verified by PR-04; storing its scalar is evidence, not independent third-party witnessing. PR-06 owns user-authored run scenarios, PR-07 cross-platform hardening and PR-08 release candidate. The Python checker is a separate implementation of the bundle validation and canonical verdict contract, not a mathematical proof of program correctness.

## Falsifiers

Missing/corrupt artifact => report ERROR/exit 4. Manifest-rehashed forged verdict => reporter must reject. A secret-bearing scenario => no canary secret in any persisted evidence file. FAIL or INCONCLUSIVE => complete bundle/reporter exit 0. Writer I/O failure => demo ERROR/exit 4. Real broken/fixed demos => fault=1, attempts=2, expected=1, observed 2/1, verdict FAIL/PASS.
