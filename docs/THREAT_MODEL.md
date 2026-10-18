# Local v0.1 threat model

## Trust boundaries

User YAML and CLI arguments are untrusted configuration. External loopback HTTP targets and their response bytes are untrusted. SQLite observer state is provided by an explicitly chosen local database; the oracle reads it independently but cannot prove the database represents a remote real-world effect. Local disk artifacts can be corrupted or rewritten by anyone with filesystem write access.

## Mitigations

- Network: numeric loopback origin with explicit port; no arbitrary DNS, external addresses, redirects, TLS interception or privileged target hooks. Bounded HTTP/1.1 Content-Length framing and deadlines.
- Parsing: closed YAML schema v1, no unknown fields, bounded JSON/headers/status, exactly one fault, two attempts. Unsupported framing (chunked, upgrade, duplicate Content-Length) is rejected.
- Filesystem: demo creates isolated new SQLite state; external run requires --oracle-db with a user-specified absolute path; observer opens read-only and parameterizes SQL rather than interpolating values.
- Evidence: redacted request body, non-Content-Type header values and observer bound parameters; no HTTP payload bytes in events. A manifest is written last and files are checked for name, size, ordering and hash. A writer I/O error exits ERROR.
- Lifetime: the engine stops/joins its proxy tasks, then observes SQLite and writes evidence. Demo owns and terminates the fixture child process.

## Remaining risks

1. SHA-256 is **not** an authenticated signature: an attacker with artifact write access can edit evidence and recompute every digest.
2. The redacted scenario snapshot is **not** executable input. A digest alone does not reproduce the original plaintext, workload timing, or scheduling.
3. An externally chosen SQLite file may be malicious, outdated, swapped or not representative of business state. v0.1 cannot attest database provenance.
4. Global power-loss durability, arbitrary process-kill cleanup, stronger cancellation handling and adversarial symlink races require further work; the local experiment is a bounded example, not a production agent.
5. Raw scenario files provided by users may contain credentials. The CLI handles them in process memory and does not promise secure memory erasure.
6. Dependency and platform supply-chain risk remains subject to an explicit owner licensing decision and later release checks.

No authentication, multi-tenant isolation, network service deployment, remote agents or SaaS billing exist in v0.1; those controls are not applicable to the local-only CLI.
