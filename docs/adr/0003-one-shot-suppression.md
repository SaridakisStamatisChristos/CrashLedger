# ADR-0003: One-shot HTTP acknowledgement suppression

**Decision:** Buffer one successful HTTP/1.1 upstream response completely; use an atomic one-shot selector and close the downstream client connection with zero bytes. Retry identical request bytes only after zero-byte transport failure.

**Rejected:** Returning synthetic HTTP 500, injecting faults before a committed response, or claiming that dropped response implies a target-level crash.

**Trade-offs:** Strict Content-Length only; no chunked transport, TLS/HTTP2, remote endpoints or broad fault menu in v0.1.
