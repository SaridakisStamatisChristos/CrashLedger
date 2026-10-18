# CrashLedger v0.1 architecture

CrashLedger is a local-first Rust experiment runner. It exercises a single, bounded post-commit HTTP response loss and checks an independently observed SQLite scalar. It is not a general distributed consistency checker.

~~~mermaid
flowchart LR
  CLI["CrashLedger CLI"] --> Engine["Engine / run lifecycle"]
  Engine --> Proxy["Loopback HTTP/1.1 proxy"]
  Proxy --> Target["Independently launched target"]
  Target --> SQLite[("Target SQLite")]
  Engine --> Oracle["Read-only SQLite oracle"]
  Oracle --> SQLite
  Engine --> Core["Pure verdict evaluator"]
  Proxy --> Core
  Oracle --> Core
  Engine --> Evidence["Evidence writer"]
  Core --> Evidence
  Evidence --> Files[("Versioned evidence + SHA-256 manifest")]
  Files --> Reporter["Offline Rust reporter"]
  Files --> Independent["Independent Python checker"]
~~~

## Boundaries

- CLI: validate schema, launch demo with owned fixture, or run a user-authored scenario against a separately managed loopback service.
- scenario: closed YAML v1 contract, numeric loopback origin with explicit port, byte and time bounds, exact matcher, bound observer SQL.
- http: Content-Length HTTP/1.1, no chunked/upgrade/TLS. Buffers first completed 2xx response and drops downstream without a single byte. Atomically fires one fault; driver retries identical request after transport EOF/reset only.
- engine: owns fixture only in demo mode, enforces quiescence and evaluates typed facts; standalone run requires an explicit absolute oracle database path, not authority inferred from YAML.
- oracle: uses a separate SQLite READ_ONLY connection, no fixture-supplied validity flag; exactly one integer result.
- core: deterministic four-valued verdict evaluator (PASS / FAIL / INCONCLUSIVE / ERROR). Missing precondition or observation never silently produces PASS.
- evidence: persists redacted snapshot, metadata, causal JSONL, independent scalar, saved verdict and last-written SHA-256 manifest; offline readers verify and recompute.

## State machine

~~~mermaid
stateDiagram-v2
  [*] --> Loaded
  Loaded --> Validated
  Validated --> Ready
  Ready --> Executing
  Executing --> Quiescing
  Quiescing --> Observing
  Observing --> Evaluating
  Evaluating --> Complete
  Evaluating --> Inconclusive
  Loaded --> Error
  Validated --> Error
  Ready --> Error
  Executing --> Error
  Quiescing --> Error
  Observing --> Error
  Evaluating --> Error
~~~

The diagram is conceptual; the engine's typed Phase transitions reject invalid transitions. Fatal setup and transport errors return exit 4 rather than attributing an application failure.

## CLI / fixture dependency separation

Neither core nor scenario depends on HTTP transport or the order fixture. The separate fixture process owns its writable SQLite connection. The read-only oracle is a different crate and connection. The evaluator receives a scalar and transport facts, never a fixture-provided PASS signal.

## Portability scope

Localhost HTTP/1.1 and bundled SQLite work without cloud accounts, Docker, Node, root privileges or external target access. Cross-platform claims must refer to CI actually executed on Linux/macOS/Windows; Linux is the reference platform for the 200-run repeated stress and authored-standalone negative cases.
