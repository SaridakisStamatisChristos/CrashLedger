use crashledger_http::{retry_post, Attempt, EventKind, Proxy, ProxyConfig};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::task::{JoinHandle, JoinSet};

struct Upstream {
    address: SocketAddr,
    requests: Arc<Mutex<Vec<Vec<u8>>>>,
    task: JoinHandle<()>,
}
impl Upstream {
    async fn start(status: u16, body: Vec<u8>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded = requests.clone();
        let task = tokio::spawn(async move {
            let mut tasks = JoinSet::new();
            while let Ok((mut socket, _)) = listener.accept().await {
                let recorded = recorded.clone();
                let body = body.clone();
                tasks.spawn(async move {
                    let mut head = Vec::new();
                    let mut byte = [0u8; 1];
                    loop {
                        if socket.read_exact(&mut byte).await.is_err() { return; }
                        head.push(byte[0]);
                        if head.ends_with(b"\r\n\r\n") { break; }
                        if head.len() > 16384 { return; }
                    }
                    let h = String::from_utf8(head.clone()).unwrap();
                    let n = h.lines()
                        .find_map(|l| l.strip_prefix("Content-Length: "))
                        .unwrap().parse::<usize>().unwrap();
                    let mut payload = vec![0; n];
                    if socket.read_exact(&mut payload).await.is_err() { return; }
                    head.extend_from_slice(&payload);
                    recorded.lock().await.push(head);
                    let reply = format!(
                        "HTTP/1.1 {status} reply\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    if socket.write_all(reply.as_bytes()).await.is_ok() {
                        let _ = socket.write_all(&body).await;
                    }
                });
            }
            tasks.abort_all();
            while tasks.join_next().await.is_some() {}
        });
        Self { address, requests, task }
    }
}
impl Drop for Upstream {
    fn drop(&mut self) { self.task.abort(); }
}
fn config(server: &Upstream) -> ProxyConfig {
    ProxyConfig {
        upstream: server.address,
        fault_path: "/orders".to_owned(),
        max_request_bytes: 65536,
        max_response_bytes: 65536,
        upstream_timeout: Duration::from_secs(4),
    }
}
const PAYLOAD: &[u8] = br#"{"sku":"widget-42","quantity":1}"#;
async fn post_bytes(addr: SocketAddr, path: &str) -> Result<Vec<u8>, std::io::Error> {
    let mut socket = TcpStream::connect(addr).await?;
    let msg = format!(
        "POST {path} HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nIdempotency-Key: demo-key\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        PAYLOAD.len()
    );
    socket.write_all(msg.as_bytes()).await?;
    socket.write_all(PAYLOAD).await?;
    let mut reply = Vec::new();
    match socket.read_to_end(&mut reply).await {
        Ok(_) => Ok(reply),
        Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset && reply.is_empty() => Ok(reply),
        Err(e) => Err(e),
    }
}

#[tokio::test]
async fn response_suppression_delivers_zero_bytes_and_only_fires_once() {
    let upstream = Upstream::start(201, br#"{"order_id":1}"#.to_vec()).await;
    let proxy = Proxy::start(config(&upstream)).await.unwrap();
    let first = post_bytes(proxy.address, "/orders").await.unwrap();
    assert!(first.is_empty(), "faulted response must have ZERO bytes");
    let second = post_bytes(proxy.address, "/orders").await.unwrap();
    assert!(second.starts_with(b"HTTP/1.1 201"));
    assert!(proxy.fault_fired());
    let events = proxy.shutdown().await;
    assert_eq!(events.iter().filter(|e| e.kind == EventKind::FaultFired).count(), 1);
    let fire_pos = events.iter().position(|e| e.kind == EventKind::FaultFired).unwrap();
    assert!(events[..fire_pos].iter().any(|e| e.kind == EventKind::UpstreamResponseComplete(201)));
    assert!(events.iter().enumerate().all(|(i, e)| e.seq == i + 1));
    let requests = upstream.requests.lock().await;
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0], requests[1], "retry bytes must be identical");
}

#[tokio::test]
async fn driver_retries_once_only_after_transport_error() {
    let upstream = Upstream::start(201, b"{}".to_vec()).await;
    let proxy = Proxy::start(config(&upstream)).await.unwrap();
    let result = retry_post(
        proxy.address, "/orders", "demo-key", PAYLOAD, 65536, 65536, Duration::from_secs(4),
    ).await.unwrap();
    assert_eq!(result.attempts.len(), 2);
    assert_eq!(result.attempts[0], Attempt::TransportError);
    assert!(matches!(result.attempts[1], Attempt::Http { status: 201, .. }));
    proxy.shutdown().await;
    let requests = upstream.requests.lock().await;
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0], requests[1]);
}

#[tokio::test]
async fn unmatched_path_is_forwarded_without_fault() {
    let upstream = Upstream::start(201, b"{}".to_vec()).await;
    let proxy = Proxy::start(config(&upstream)).await.unwrap();
    let result = retry_post(
        proxy.address, "/other", "demo-key", PAYLOAD, 65536, 65536, Duration::from_secs(4),
    ).await.unwrap();
    assert_eq!(result.attempts.len(), 1);
    assert!(matches!(result.attempts[0], Attempt::Http { status: 201, .. }));
    let events = proxy.shutdown().await;
    assert!(!events.iter().any(|e| e.kind == EventKind::FaultFired));
}

#[tokio::test]
async fn failed_upstream_response_is_not_suppressed_or_retried() {
    let upstream = Upstream::start(500, b"{}".to_vec()).await;
    let proxy = Proxy::start(config(&upstream)).await.unwrap();
    let result = retry_post(
        proxy.address, "/orders", "demo-key", PAYLOAD, 65536, 65536, Duration::from_secs(4),
    ).await.unwrap();
    assert_eq!(result.attempts.len(), 1);
    assert!(matches!(result.attempts[0], Attempt::Http { status: 500, .. }));
    let events = proxy.shutdown().await;
    assert_eq!(events.iter().filter(|e| e.kind == EventKind::FaultFired).count(), 0);
}

#[tokio::test]
async fn simultaneous_requests_can_claim_fault_only_once() {
    let upstream = Upstream::start(201, b"{}".to_vec()).await;
    let proxy = Proxy::start(config(&upstream)).await.unwrap();
    let mut tasks = Vec::new();
    for _ in 0..12 {
        let address = proxy.address;
        tasks.push(tokio::spawn(async move { post_bytes(address, "/orders").await.unwrap() }));
    }
    let mut lost = 0;
    let mut delivered = 0;
    for task in tasks {
        let response = task.await.unwrap();
        if response.is_empty() { lost += 1; }
        else if response.starts_with(b"HTTP/1.1 201") { delivered += 1; }
    }
    assert_eq!((lost, delivered), (1, 11));
    let events = proxy.shutdown().await;
    assert_eq!(events.iter().filter(|e| e.kind == EventKind::FaultFired).count(), 1);
    assert_eq!(upstream.requests.lock().await.len(), 12);
}

#[tokio::test]
async fn oversized_upstream_never_claims_fault() {
    let upstream = Upstream::start(201, vec![0; 100]).await;
    let mut cfg = config(&upstream);
    cfg.max_response_bytes = 10;
    let proxy = Proxy::start(cfg).await.unwrap();
    let response = post_bytes(proxy.address, "/orders").await.unwrap();
    assert!(response.is_empty());
    let events = proxy.shutdown().await;
    assert!(events.iter().any(|e| e.kind == EventKind::ProxyError));
    assert!(!events.iter().any(|e| e.kind == EventKind::FaultFired));
}

#[tokio::test]
async fn nonloopback_upstream_is_rejected() {
    let cfg = ProxyConfig {
        upstream: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)), 80),
        fault_path: "/orders".to_string(),
        max_request_bytes: 65536,
        max_response_bytes: 65536,
        upstream_timeout: Duration::from_secs(4),
    };
    assert!(Proxy::start(cfg).await.is_err());
}
