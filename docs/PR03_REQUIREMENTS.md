# PR-03 — bounded HTTP/1.1 fault proxy and retry driver

## Transport requirements and acceptance evidence

| ID | Invariant | Owner | Falsification |
| --- | --- | --- | --- |
| H01 | Loopback listener/upstream only; bounded requests, responses and headers | ProxyConfig, TCP reader | reject nonlocal addresses and oversize response |
| H02 | Exact POST path match after completed 2xx upstream response | handler, EventKind | no fire on unmatched path or HTTP 500 |
| H03 | Suppressed response sends **zero downstream bytes** and produces transport error, never synthetic HTTP 500 | proxy connection owner | raw TCP read returns zero bytes |
| H04 | Atomic one-shot token fires at most once across concurrency | AtomicBool CAS | 12 concurrent requests cause exactly one suppressed response |
| H05 | Driver retries transport EOF only; same request byte-for-byte | retry_post | upstream request capture identical, HTTP 500 not retried |
| H06 | Versioned result ownership separation: proxy emits transport facts, not application PASS | EventKind/WorkloadResult | inspect API and event tests |
| H07 | Bounded structured shutdown with no surviving worker tasks | Proxy::shutdown JoinSet | shutdown and close tests |
| H08 | Cargo lock, formatting/lint/tests plus CLI smoke | workspace and CI | green GitHub Actions on current head |

## Intentional HTTP behavior
Strict HTTP/1.1 with mandatory Content-Length; malformed/multiple length, chunked Transfer-Encoding, Upgrade and oversized bodies are rejected and recorded as ProxyError. Proxy reads the upstream response completely before attempting a one-shot CAS; it writes no downstream bytes on success of CAS. Connection is dropped to generate transport EOF. Noneligible replies are forwarded verbatim. Only one client request per connection, connection-close semantics.

## Boundaries
Proxy cannot infer if a database transaction committed merely because an HTTP response was read. PR-02 fixture commits before success; PR-04 must independently observe SQLite. Event sequence is monotonic only within a single in-memory logger; it is not persisted until PR-05. No TLS, remote endpoints, streaming, model checking, real process kill, general application correctness, secrets recording, or production traffic.

## Verification and evidence

The transport-specific acceptance gates H01–H08 are exercised by the proxy's TCP regression tests, including one-shot concurrency and a genuine zero-byte lost response. The integration suite tests the broken and fixed SQLite services through the same proxy and driver. See the [GitHub Actions workflow](../.github/workflows/ci.yml) and [current workflow runs](https://github.com/SaridakisStamatisChristos/CrashLedger/actions/workflows/ci.yml). Verify the result for the commit being reviewed; historical runs do not attest to later changes.

A proxy event alone cannot establish the target's committed state. The experiment evaluator uses a separate SQLite observation and must treat a missed fault, invalid transport sequence or unavailable observation as ERROR/INCONCLUSIVE as appropriate, never as target FAIL/PASS.

Docker, cloud deployment, authentication and multi-tenancy are outside this local proxy's scope.
