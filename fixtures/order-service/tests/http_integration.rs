//! Black-box tests launch the real fixture process, exercise real HTTP/1.1,
//! and verify state through a separate read-only SQLite connection.
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

struct Fixture {
    child: Child,
    _temp: tempfile::TempDir,
    db: PathBuf,
    address: String,
}
impl Fixture {
    fn start(mode: &str) -> Self {
        let temp = tempfile::tempdir().expect("isolated DB dir");
        let db = temp.path().join("orders.sqlite");
        let mut child = Command::new(env!("CARGO_BIN_EXE_crashledger-demo"))
            .args(["--mode", mode, "--db-path"])
            .arg(&db)
            .args(["--listen", "127.0.0.1:0"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn reference fixture");
        let mut line = String::new();
        BufReader::new(child.stdout.take().expect("fixture stdout"))
            .read_line(&mut line)
            .expect("read readiness line");
        assert!(
            line.starts_with("READY http://127.0.0.1:"),
            "fixture not ready: {line:?}"
        );
        let address = line
            .trim()
            .strip_prefix("READY http://")
            .unwrap()
            .to_string();
        let fixture = Self {
            child,
            _temp: temp,
            db,
            address,
        };
        assert_eq!(fixture.request("GET", "/ready", None, "").0, 200);
        fixture
    }
    fn request(&self, method: &str, path: &str, key: Option<&str>, body: &str) -> (u16, String) {
        send(&self.address, method, path, key, body)
    }
    fn count(&self, key: &str) -> i64 {
        let conn = Connection::open_with_flags(&self.db, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .expect("read-only independent SQLite observer");
        conn.query_row(
            "SELECT COUNT(*) FROM orders WHERE idempotency_key = ?1",
            [key],
            |row| row.get(0),
        )
        .expect("read committed count")
    }
    fn ledger_count(&self, key: &str) -> i64 {
        let conn = Connection::open_with_flags(&self.db, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .expect("independent SQLite observer");
        conn.query_row(
            "SELECT COUNT(*) FROM idempotency_requests WHERE idempotency_key = ?1",
            [key],
            |row| row.get(0),
        )
        .expect("read committed ledger")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
fn send(address: &str, method: &str, path: &str, key: Option<&str>, body: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(address).expect("connect fixture");
    stream
        .set_read_timeout(Some(Duration::from_secs(12)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(12)))
        .unwrap();
    let key_header = key
        .map(|k| format!("Idempotency-Key: {k}\r\n"))
        .unwrap_or_default();
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/json\r\n{key_header}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(request.as_bytes())
        .expect("send HTTP request");
    let mut bytes = Vec::new();
    stream
        .read_to_end(&mut bytes)
        .expect("receive HTTP response");
    let response = String::from_utf8(bytes).expect("HTTP fixture returns UTF-8");
    let status = response
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()
        .expect("HTTP status");
    (
        status,
        response.split_once("\r\n\r\n").unwrap().1.to_owned(),
    )
}
fn order_id(body: &str) -> i64 {
    let data: Value = serde_json::from_str(body).expect("JSON body");
    data["order_id"].as_i64().expect("integer order_id")
}
const BODY: &str = r#"{"sku":"widget-42","quantity":1}"#;

#[test]
fn broken_mode_duplicates_committed_orders() {
    let fixture = Fixture::start("broken");
    let a = fixture.request("POST", "/orders", Some("same-key"), BODY);
    let b = fixture.request("POST", "/orders", Some("same-key"), BODY);
    assert_eq!((a.0, b.0), (201, 201));
    assert_ne!(order_id(&a.1), order_id(&b.1));
    assert_eq!(fixture.count("same-key"), 2);
    assert_eq!(fixture.ledger_count("same-key"), 0);
}
#[test]
fn fixed_mode_replays_committed_order() {
    let fixture = Fixture::start("fixed");
    let a = fixture.request("POST", "/orders", Some("same-key"), BODY);
    let b = fixture.request("POST", "/orders", Some("same-key"), BODY);
    assert_eq!((a.0, b.0), (201, 200));
    assert_eq!(order_id(&a.1), order_id(&b.1));
    assert_eq!(fixture.count("same-key"), 1);
    assert_eq!(fixture.ledger_count("same-key"), 1);
}
#[test]
fn modified_payload_returns_conflict_without_second_order() {
    let fixture = Fixture::start("fixed");
    assert_eq!(
        fixture.request("POST", "/orders", Some("same-key"), BODY).0,
        201
    );
    let changed = r#"{"sku":"widget-42","quantity":2}"#;
    assert_eq!(
        fixture
            .request("POST", "/orders", Some("same-key"), changed)
            .0,
        409
    );
    assert_eq!(fixture.count("same-key"), 1);
    assert_eq!(fixture.ledger_count("same-key"), 1);
}
#[test]
fn parallel_same_key_creates_exactly_one_order() {
    let fixture = Fixture::start("fixed");
    let mut workers = Vec::new();
    for _ in 0..12 {
        let address = fixture.address.clone();
        workers.push(thread::spawn(move || {
            send(&address, "POST", "/orders", Some("race-key"), BODY)
        }));
    }
    let replies: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    assert_eq!(
        replies.iter().filter(|(status, _)| *status == 201).count(),
        1
    );
    assert_eq!(
        replies.iter().filter(|(status, _)| *status == 200).count(),
        11
    );
    let ids: Vec<_> = replies.iter().map(|(_, body)| order_id(body)).collect();
    assert!(ids.iter().all(|id| *id == ids[0]));
    assert_eq!(fixture.count("race-key"), 1);
    assert_eq!(fixture.ledger_count("race-key"), 1);
}
#[test]
fn malformed_requests_do_not_create_orders() {
    let fixture = Fixture::start("fixed");
    assert_eq!(fixture.request("POST", "/orders", None, BODY).0, 400);
    assert_eq!(
        fixture.request("POST", "/orders", Some("key"), "{bad").0,
        400
    );
    assert_eq!(
        fixture
            .request("POST", "/orders", Some("key"), r#"{"sku":"","quantity":1}"#)
            .0,
        400
    );
    assert_eq!(
        fixture
            .request(
                "POST",
                "/orders",
                Some("key"),
                r#"{"sku":"widget","quantity":0}"#
            )
            .0,
        400
    );
    assert_eq!(
        fixture
            .request(
                "POST",
                "/orders",
                Some("key"),
                r#"{"sku":"widget","quantity":1,"secret":"extra"}"#
            )
            .0,
        400
    );
    assert_eq!(fixture.count("key"), 0);
}
#[test]
fn oversize_request_is_rejected() {
    let fixture = Fixture::start("fixed");
    let body = format!(r#"{{"sku":"{}","quantity":1}}"#, "x".repeat(66_000));
    assert_eq!(
        fixture.request("POST", "/orders", Some("key"), &body).0,
        413
    );
    assert_eq!(fixture.count("key"), 0);
}
#[test]
fn nonlocal_bind_is_rejected() {
    let output = Command::new(env!("CARGO_BIN_EXE_crashledger-demo"))
        .args([
            "--mode",
            "fixed",
            "--db-path",
            "nonexistent-for-remote-bind.sqlite",
            "--listen",
            "0.0.0.0:0",
        ])
        .output()
        .expect("run denied config");
    assert!(!output.status.success());
}
