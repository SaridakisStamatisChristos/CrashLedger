#!/usr/bin/env python3
"""PR-06 black-box authored scenario: independently launched fixture, negative paths."""
import http.server
import json
from pathlib import Path
import socket
import sqlite3
import subprocess
import sys
import tempfile
import threading

REPO = Path(__file__).resolve().parent.parent
CLI = REPO / "target" / "debug" / ("crashledger.exe" if sys.platform == "win32" else "crashledger")
FIXTURE = REPO / "target" / "debug" / ("crashledger-demo.exe" if sys.platform == "win32" else "crashledger-demo")
GOLDEN = (REPO / "scenarios" / "lost-ack.yaml").read_text(encoding="utf-8")


def require(predicate, message):
    if not predicate:
        raise AssertionError(message)


def run_cli(scenario, db, artifacts, expected_code, expected_text):
    result = subprocess.run(
        [str(CLI), "run", str(scenario), "--oracle-db", str(db), "--out", str(artifacts)],
        text=True, capture_output=True, timeout=45, cwd=REPO
    )
    require(result.returncode == expected_code,
            f"code {result.returncode} != {expected_code}; stdout={result.stdout}; stderr={result.stderr}")
    require(expected_text in result.stdout, f"missing {expected_text}: {result.stdout}; {result.stderr}")
    if expected_code in (0, 2, 3):
        run_dirs = list(artifacts.glob("r-*"))
        require(len(run_dirs) == 1, "expected one isolated evidence directory")
        report = subprocess.run([str(CLI), "report", str(run_dirs[0]), "--format", "json"],
                                text=True, capture_output=True, timeout=20)
        require(report.returncode == 0, f"offline report error: {report.stderr}")
        parsed = json.loads(report.stdout)
        require(parsed["verdict"] == expected_text.split("Verdict: ")[-1],
                "reported verdict mismatch")
        require(parsed["fixture_mode"] == "external", "external run claimed to be a fixture")
        return parsed
    return None


def scenario_file(root, target, match="/orders"):
    document = GOLDEN.replace("base_url: http://127.0.0.1:4001", f"base_url: http://{target}")
    document = document.replace("id: lost-ack-order-retry", "id: authored-external-validation")
    document = document.replace("    path: /orders\n  when:", f"    path: {match}\n  when:")
    path = root / "authored.yaml"
    path.write_text(document, encoding="utf-8")
    return path


def fixture_cycle(mode, base):
    db = base / f"{mode}.sqlite"
    p = subprocess.Popen([str(FIXTURE), "--mode", mode, "--db-path", str(db),
                          "--listen", "127.0.0.1:0"],
                         stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    try:
        line = p.stdout.readline().strip()
        require(line.startswith("READY http://127.0.0.1:"), f"bad fixture READY: {line!r}")
        endpoint = line.removeprefix("READY http://")
        scenario = scenario_file(base, endpoint)
        correct = "FAIL" if mode == "broken" else "PASS"
        code = 2 if mode == "broken" else 0
        report = run_cli(scenario, db.resolve(), base / f"{mode}-evidence", code,
                         f"Verdict: {correct}")
        require(report["observed"] == (2 if mode == "broken" else 1),
                "incorrect independent DB count")
        if mode == "fixed":
            # Fault path doesn't match a successful upstream request.
            wrong = scenario_file(base, endpoint, "/somewhere-else")
            report = run_cli(wrong, db.resolve(), base / "wrong-path", 3,
                             "Verdict: INCONCLUSIVE")
            require(report["fault_fired"] == 0, "mismatched fault was fired")
            scenario_file(base, endpoint)
            # A fresh idempotency key avoids inheriting data; oracle path denied
            # via an explicitly chosen *missing* database must remain uncertain.
            missing = base / "not-existing.sqlite"
            run_cli(scenario, missing.resolve(), base / "oracle-missing", 3,
                    "Verdict: INCONCLUSIVE")
            require(not missing.exists(), "read-only oracle created DB file")
        return endpoint
    finally:
        p.terminate()
        try:
            p.communicate(timeout=5)
        except subprocess.TimeoutExpired:
            p.kill()
            p.communicate(timeout=5)


class FailService(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    def do_POST(self):
        length = int(self.headers.get("Content-Length", "0"))
        self.rfile.read(length)
        self.send_response(500)
        self.send_header("Content-Length", "2")
        self.send_header("Connection", "close")
        self.end_headers()
        self.wfile.write(b"{}")
    def log_message(self, *args):
        pass


def main():
    subprocess.run(["cargo", "build", "--locked", "-q", "-p", "crashledger-cli",
                    "-p", "crashledger-demo"], check=True, cwd=REPO, timeout=240)
    with tempfile.TemporaryDirectory(prefix="crashledger-pr06-") as d:
        base = Path(d)
        for mode in ("broken", "fixed"):
            sub = base / mode
            sub.mkdir()
            fixture_cycle(mode, sub)

        with http.server.HTTPServer(("127.0.0.1", 0), FailService) as server:
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()
            try:
                scenario = scenario_file(base, f"127.0.0.1:{server.server_port}")
                db = base / "separate.db"
                conn = sqlite3.connect(db)
                conn.execute("CREATE TABLE orders(id INTEGER, idempotency_key TEXT)")
                conn.commit()
                conn.close()
                report = run_cli(scenario, db.resolve(), base / "upstream-500", 3,
                                 "Verdict: INCONCLUSIVE")
                require(report["fault_fired"] == 0, "suppress_response fired on HTTP 500")
            finally:
                server.shutdown()
                thread.join(timeout=5)

        # No process listening: fail closed as infrastructure ERROR.
        with socket.socket() as sock:
            sock.bind(("127.0.0.1", 0))
            unused_port = sock.getsockname()[1]
        scenario = scenario_file(base, f"127.0.0.1:{unused_port}")
        db = base / "separate.db"
        run_cli(scenario, db.resolve(), base / "unreachable", 4, "")
        print("PR-06 verified: broken FAIL, fixed PASS, wrong-path and HTTP 500 "
              "INCONCLUSIVE, missing oracle INCONCLUSIVE, refused upstream ERROR")


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print(f"PR-06 FAILED: {error}", file=sys.stderr)
        sys.exit(1)
