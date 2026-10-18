# Correctness and verdict contract

## What can be concluded

The canonical experiment observes **one lost HTTP acknowledgement** after the reference order service has committed its transaction. An identical retry reaches the reference service. A separate read-only SQLite query counts orders for the same key.

- Broken negative control: two committed orders, expected one => FAIL, exit 2.
- Fixed positive control: one committed order, expected one => PASS, exit 0.
- Missing fault or required retry, missing independent SQLite scalar, or no final successful retry => INCONCLUSIVE, exit 3 (if execution facts are available).
- Infrastructure failure (e.g. unreachable local target, proxy fault, evidence I/O failure) => ERROR, exit 4.

A PASS means the **specified invariant held in this one observation**. It is not a proof of arbitrary exactly-once processing, general crash safety, process restart durability, or correctness under all failures.

## Acceptance preconditions

The one-shot fault is eligible only after a complete 2xx HTTP/1.1 Content-Length response has been buffered by the proxy, with **zero downstream bytes** sent to the client. An actual transport failure causes exactly one byte-identical retry. The evaluator demands one matching fault, two attempts, successful final response, and an independent integer observation.

A first response of HTTP 500 is not a successful post-commit trigger; no fault is fired and the run is INCONCLUSIVE. Retry is never triggered by ordinary application HTTP errors. A blocked or unavailable observer never becomes PASS.

## Negative control matters

The broken service intentionally lacks a unique order constraint, and its two rows are verified through SQLite. The fixed service uses BEGIN IMMEDIATE and a unique idempotency ledger; concurrent duplicate submissions return the same order ID. Both travel through the same driver/proxy/oracle/evaluator path.

## Reproducibility

The golden broken/fixed demos pass CI on Linux and are also tested in 100 fresh pairs. The in-memory event ordering records observed facts, not verified global happens-before across arbitrary processes. See EVIDENCE_SCHEMA.md and THREAT_MODEL.md.
