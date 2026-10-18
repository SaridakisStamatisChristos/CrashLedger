# CrashLedger PR-04 — execution and independent oracle contract

Milestone: first end-to-end demo. NOT a general distributed correctness proof.

| ID | Hard requirement | Implementation owner | Acceptance gate |
| --- | --- | --- | --- |
| D01 | Both modes execute same canonical v1 scenario | engine + scenario | golden broken/fixed black-box |
| D02 | Isolated database, dynamic loopback fixture/proxy ports, owned fixture child | engine | two clean runs, unique directories |
| D03 | Complete upstream 2xx followed by zero-byte downstream loss and retry | existing PR-03 proxy + engine trace checks | TCP tests and golden demo |
| D04 | Client retries exactly once after transport error, identical body and idempotency key | PR-03 driver + engine | golden attempts and TCP identity |
| D05 | Read-only SQLite scalar oracle, separate connection and bound SQL parameters | oracle crate | independent SQL tests |
| D06 | Strict preconditions before checking observed target state | core + engine judge | synthetic no fault/no retry/invalid trace tests |
| D07 | Broken run FAIL, count=2, exit=2 | CLI + fixture + oracle | real subprocess integration/CI |
| D08 | Fixed run PASS, count=1, exit=0 | CLI + fixture + oracle | real subprocess integration/CI |
| D09 | Oracle missing gives INCONCLUSIVE, infrastructure failures give ERROR | core + engine + oracle | synthetic tests, CLI error tests |
| D10 | Build/format/lint/test using committed lockfile | GitHub Actions | current-head CI green |

Scenario target URL is replaced only with the dynamic bound address of the repository fixture. The idempotency key and JSON request are identical between modes. The SQLite path stays relative to the fresh run directory. No HTTP target endpoint supplies correctness evidence.

The fixture is a directly owned **subprocess**, launched from a sibling binary (or built via Cargo if not already present in a Rust source checkout). The fixture announces its OS-selected local port; an explicit GET /ready confirms SQLite schema availability. On cleanup, the child is killed and waited for. Proxy shutdown drains/cancels tasks before opening the observer.

The independent observer opens SQLite READ_ONLY and requires a read-only SQL statement returning exactly one integer cell (one row and one column), with matching bound parameter count; any unavailable or malformed observation is INCONCLUSIVE instead of PASS. It is not allowed to mutate the order service's SQLite database.

The outcome is deliberately scoped to this tested execution and specific invariant. PR-05 is responsible for writing complete versioned evidence bundles, manifests and reports. PR-04 retains the run directory with isolated SQLite data, but does not claim complete forensic evidence or replay.

### Falsification and applicability

Test broken/fixed isolation, missed fault, missing retry, ordering invalidity, unavailable observer, non-integer and malformed SQL result, corrupt HTTP response or proxy failure, and refused filesystem output. Distributed scheduling, HTTPS interception, remote targets, auth, cloud infrastructure, crash durability and signed evidence are outside PR-04. No release claim or license change.
