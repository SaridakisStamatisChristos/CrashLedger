# ADR-0002: Independent read-only SQLite observer

**Decision:** Observe committed state directly through a separate SQLite READ_ONLY connection after the client/proxy become quiescent. A parameterized SELECT must return exactly one integer cell.

**Rejected:** Judging correctness from the target's HTTP status, readiness endpoint, a fixture-supplied success flag, or assuming upstream 2xx alone proves the invariant.

**Trade-offs:** A user must explicitly designate external database file access; observer results depend on the authenticity and representativeness of the chosen DB. No cross-process durable-consistency proof is claimed.
