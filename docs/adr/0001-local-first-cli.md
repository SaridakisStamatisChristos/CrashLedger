# ADR-0001: Local-first Rust CLI

**Decision:** Keep CrashLedger a small Rust workspace CLI using loopback HTTP and a local observer. No SaaS, agent fleet, queue, database service, web dashboard or privileged runtime is required for v0.1.

**Why:** A narrowly controlled demonstration can be compiled, executed and independently validated with minimal external infrastructure. Cargo.lock is committed and used with --locked.

**Trade-offs:** No remote production testing, cluster scheduling, background reliability guarantees or authentication surfaces. Deferred until supported by user need and evidence.
