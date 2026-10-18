#!/usr/bin/env python3
"""Independent CrashLedger v1 bundle verifier (Python standard library only).

No Rust dependency and no target execution. SHA-256 is corruption detection, not
authentication: someone who edits all files can regenerate these hashes.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sys

NAMES = ("scenario.snapshot.yaml", "run.json", "events.jsonl",
         "observation.json", "verdict.json")


def require(ok, reason):
    if not ok:
        raise ValueError(reason)


def verify(directory, mode=None, verdict=None, count=None):
    require(directory.is_dir() and not directory.is_symlink(), "invalid bundle directory")
    raw = {}
    manifest = directory / "manifest.sha256"
    require(manifest.is_file() and not manifest.is_symlink(), "missing manifest")
    require(manifest.stat().st_size <= 4096, "oversized manifest")
    rows = manifest.read_text(encoding="utf-8").splitlines()
    require(len(rows) == len(NAMES), "wrong manifest length")
    for name, line in zip(NAMES, rows):
        require(line.endswith("  " + name), "manifest file/name/order mismatch")
        digest = line[:64]
        require(len(digest) == 64 and
                all(ch in "0123456789abcdef" for ch in digest),
                "bad SHA-256 digest")
        file = directory / name
        require(file.is_file() and not file.is_symlink(), "missing or symlink file: " + name)
        require(file.stat().st_size <= 131072, "oversized artifact: " + name)
        raw[name] = file.read_bytes()
        require(hashlib.sha256(raw[name]).hexdigest() == digest, "SHA-256 mismatch: " + name)

    run = json.loads(raw["run.json"])
    obs = json.loads(raw["observation.json"])
    recorded = json.loads(raw["verdict.json"])
    lines = raw["events.jsonl"].splitlines()
    events = [json.loads(line) for line in lines]
    require(events, "empty event log")
    require(run["schema_version"] == obs["schema_version"] == recorded["schema_version"] == 1,
            "schema mismatch")
    require(run["snapshot_redacted"] is True, "nonredacted snapshot")
    require(run["run_id"] == directory.name, "run ID mismatch")
    require(run["fixture_mode"] in ("broken", "fixed"), "unknown mode")
    require(obs["read_only"] is True and obs["kind"] == "sqlite_scalar", "oracle provenance invalid")
    require(all(e["seq"] == i and e["run_id"] == run["run_id"]
                for i, e in enumerate(events, 1)), "event ordering or run ID mismatch")
    require(events[-1]["kind"] in ("oracle_observation", "oracle_unavailable"),
            "missing final oracle event")
    if obs["observed"] is None:
        require(events[-1]["kind"] == "oracle_unavailable", "oracle event mismatch")
    else:
        require(events[-1]["kind"] == "oracle_observation"
                and events[-1]["value"] == obs["observed"], "observation event mismatch")

    fires = [i for i, e in enumerate(events) if e["kind"] == "fault_fired"]
    delivered = [e for e in events if e["kind"] == "client_response"]
    sent = [e for e in events if e["kind"] == "request_sent"]
    received = [e for e in events if e["kind"] == "request_received"]
    errors = [e for e in events if e["kind"] == "proxy_error"]
    require(len(fires) == run["matching_faults_fired"] <= 1, "fault count mismatch")
    require(len(sent) == len(received) == run["attempts"], "attempt count mismatch")
    require([e["attempt"] for e in sent] == list(range(1, run["attempts"] + 1)),
            "attempts out of order")
    if fires:
        fired = events[fires[0]]
        preceding = events[:fires[0]]
        require(any(e["kind"] == "upstream_response_complete"
                    and e.get("attempt") == fired["attempt"]
                    and 200 <= e.get("status", 0) <= 299 for e in preceding),
                "fault before 2xx upstream completion")
        require(not any(e["kind"] == "downstream_response"
                        and e.get("attempt") == fired["attempt"] for e in events),
                "faulted response delivered")
    transport_errors = [e for e in events if e["kind"] == "client_transport_error"]
    retried = (run["attempts"] == 2 and len(transport_errors) == 1
               and transport_errors[0]["attempt"] == 1)
    final_http = delivered[-1].get("status") if retried and delivered and delivered[-1]["attempt"] == 2 else None
    if errors:
        computed = "ERROR"
    elif len(fires) != 1:
        computed = "INCONCLUSIVE"
    elif run["attempts"] != 2:
        computed = "INCONCLUSIVE"
    elif not (retried and final_http is not None and 200 <= final_http <= 299):
        computed = "INCONCLUSIVE"
    elif obs["observed"] is None:
        computed = "INCONCLUSIVE"
    elif obs["observed"] == obs["expected"]:
        computed = "PASS"
    else:
        computed = "FAIL"
    require(recorded["verdict"] == computed, "recorded verdict contradicts independently computed verdict")
    if mode is not None:
        require(run["fixture_mode"] == mode, "unexpected fixture mode")
    if verdict is not None:
        require(computed == verdict, "unexpected verdict")
    if count is not None:
        require(obs["observed"] == count, "unexpected observed count")
    print(f"VERIFIED {directory} mode={run['fixture_mode']} fault={len(fires)} "
          f"attempts={len(sent)} expected={obs['expected']} "
          f"observed={obs['observed']} verdict={computed}")
    return computed


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("bundle", type=Path)
    ap.add_argument("--expect-mode", choices=("broken", "fixed"))
    ap.add_argument("--expect-verdict", choices=("PASS", "FAIL", "INCONCLUSIVE", "ERROR"))
    ap.add_argument("--expect-count", type=int)
    args = ap.parse_args()
    try:
        verify(args.bundle, args.expect_mode, args.expect_verdict, args.expect_count)
    except (OSError, ValueError, TypeError, KeyError, IndexError, UnicodeError) as e:
        print(f"REJECTED: {e}", file=sys.stderr)
        return 4
    return 0


if __name__ == "__main__":
    sys.exit(main())
