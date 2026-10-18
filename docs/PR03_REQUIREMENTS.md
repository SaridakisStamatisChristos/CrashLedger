# PR-03 — bounded HTTP/1.1 fault proxy and retry driver

## Hard R7 requirements and acceptance evidence

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

## R7 evidence closure
H01-H08 start UNVERIFIED until final-head CI. No Docker/cloud/release, auth or multi-tenancy are applicable. PR-04 integration must treat timeout, ProxyError and missing fault as ERROR/INCONCLUSIVE, never target FAIL/PASS.
