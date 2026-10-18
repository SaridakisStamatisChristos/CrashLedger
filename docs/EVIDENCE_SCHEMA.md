# Evidence v1 on-disk schema

A completed local run creates an isolated directory with:

~~~text
r-<unique>/
  run-data/orders.sqlite       # demo only; not included in manifest
  scenario.snapshot.yaml       # redacted, NON-EXECUTABLE
  run.json                     # schema_version, run_id, scenario_id, mode, tool, platform, timeouts, scenario SHA-256
  events.jsonl                 # monotonic seq and typed observed transport / oracle facts
  observation.json             # READ_ONLY sqlite scalar or unavailable
  verdict.json                 # PASS | FAIL | INCONCLUSIVE | ERROR
  manifest.sha256              # five exact file hashes; written last
~~~

Manifest format: exactly five lines of lowercase hex SHA-256, two spaces, filename, in the listed order (excluding run-data and manifest itself). Missing, corrupted, reordered, symlinked, oversized or inconsistent files cause the Rust reporter to refuse validated status. The offline report recalculates the core evaluator result and compares it to verdict.json; a forged verdict plus a regenerated manifest is still rejected if inconsistent with facts.

Events use seq, run_id, kind, optional attempt/status/value. Kinds include request_sent, request_received, upstream_response_complete, fault_fired, client_transport_error, downstream_response, client_response, oracle_observation and oracle_unavailable. The first request_sent event reflects proxy receipt, not an independent network transmission probe. A final oracle event corresponds to the separately recorded observation.

~~~bash
cargo run --locked -p crashledger-cli -- report artifacts/r-REAL-ID --format text
cargo run --locked -p crashledger-cli -- report artifacts/r-REAL-ID --format json
python3 scripts/verify_bundle.py artifacts/r-REAL-ID --expect-verdict PASS
~~~

The report command exits 0 on successful inspection even for a correctly validated FAIL verdict; corruption exits 4. A SHA-256 manifest checks integrity for ordinary corruption, not malicious re-signing or authenticated provenance.
