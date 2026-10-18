# Scenario schema v1

The checked-in example at scenarios/lost-ack.yaml is the canonical YAML contract. All mapping types reject unknown keys. Required fields cannot be omitted. Parsing/validation has **no target I/O**.

- Only the literal method POST and HTTP origin with an explicitly specified loopback IP and port are admitted. Numeric 127/8 and ::1 are allowed. DNS hostnames (including localhost) are rejected to avoid name rebinding. Credentials, HTTPS, URL paths, redirects and query/fragment components in base_url are not accepted.
- The request is a bounded JSON object, has Content-Type application/json and a nonempty stable ASCII Idempotency-Key. User-provided connection/framing headers are forbidden. Paths must be absolute, bounded, without traversal or query.
- v1 supports exactly two attempts, only retry_on transport_error, up to 1 second backoff, and one fault firing after complete 2xx upstream response. Fault path mismatch is valid and must later yield INCONCLUSIVE, not PASS.
- The SQLite scalar oracle must have a relative database path without lexical parent traversal and a single bounded SELECT query with at most 16 scalar bound parameters. These lexical checks are NOT a filesystem sandbox or SQL security boundary. At execution time, read-only database access and statement checks are enforced independently of schema validation. For an authored standalone run, the observer database must be explicitly selected with `--oracle-db`; scenario YAML does not grant access to a database.
- Max request and response bytes are 1..65536; total timeout is 1..120000 ms, with upstream timeout strictly smaller.
- Invalid schema or unsupported version exits 4 in the CLI. Validation must not equate accepted scenario configuration with an executed test.

## Scope limitations

`validate` checks only the scenario schema; it does not inject a fault or establish a correctness verdict. To execute an experiment, use `demo` for the isolated reference service or `run` with a separately managed loopback target and an explicit observer database. The engine must record the fault and retry, obtain an independent observation, and evaluate the resulting evidence. Unknown fields, unbounded runs, remote addresses and unsupported fault behavior are rejected.
