# Scenario schema v1 — PR-01

The checked-in example at scenarios/lost-ack.yaml is the canonical YAML contract. All mapping types reject unknown keys. Required fields cannot be omitted. Parsing/validation has **no target I/O**.

- Only the literal method POST and HTTP origin with an explicitly specified loopback IP and port are admitted. Numeric 127/8 and ::1 are allowed. DNS hostnames (including localhost) are rejected to avoid name rebinding. Credentials, HTTPS, URL paths, redirects and query/fragment components in base_url are not accepted.
- The request is a bounded JSON object, has Content-Type application/json and a nonempty stable ASCII Idempotency-Key. User-provided connection/framing headers are forbidden. Paths must be absolute, bounded, without traversal or query.
- v1 supports exactly two attempts, only retry_on transport_error, up to 1 second backoff, and one fault firing after complete 2xx upstream response. Fault path mismatch is valid and must later yield INCONCLUSIVE, not PASS.
- The SQLite scalar oracle must have a relative database path without lexical parent traversal and a single bounded SELECT query with at most 16 scalar bound parameters. These lexical checks are NOT a filesystem sandbox or SQL security boundary. Runtime read-only flags, statement-readonly checks and pinned working-directory handling remain required in PR-04.
- Max request and response bytes are 1..65536; total timeout is 1..120000 ms, with upstream timeout strictly smaller.
- Invalid schema or unsupported version exits 4 in the CLI. Validation must not equate accepted scenario configuration with an executed test.

## Scope limitations

In PR-01 the evaluator is pure and its inputs are only synthetic unit-test vectors. Only the later engine, independent observer, fault proxy and recorded workload may supply real evidence. Unknown fields, missing oracle, unbounded runs, remote addresses and unsupported fault behavior are errors at parse/validate time.
