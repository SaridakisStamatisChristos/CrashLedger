//! Minimal bounded HTTP/1.1 proxy and retry workload for PR-03.
//!
//! No application-state or verdict decisions occur here. This crate records
//! transport facts; PR-04's independent oracle determines application correctness.
//! Only loopback HTTP/1.1 with Content-Length framing is supported.

use std::io;
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{oneshot, Mutex, Semaphore};
use tokio::task::{JoinHandle, JoinSet};

const MAX_HEADER_BYTES: usize = 16 * 1024;
const MAX_CONNECTIONS: usize = 32;

#[derive(Debug, Clone)]
pub struct ProxyConfig {
    pub upstream: SocketAddr,
    pub fault_path: String,
    pub max_request_bytes: usize,
    pub max_response_bytes: usize,
    pub upstream_timeout: Duration,
}

impl ProxyConfig {
    pub fn validate(&self) -> io::Result<()> {
        if !self.upstream.ip().is_loopback() {
            return Err(invalid("upstream must be loopback"));
        }
        if self.upstream.port() == 0 {
            return Err(invalid("upstream port must be nonzero"));
        }
        if !valid_path(&self.fault_path)
            || self.max_request_bytes == 0
            || self.max_request_bytes > 65_536
            || self.max_response_bytes == 0
            || self.max_response_bytes > 65_536
            || self.upstream_timeout.is_zero()
            || self.upstream_timeout > Duration::from_secs(120)
        {
            return Err(invalid("invalid fault path or resource bounds"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventKind {
    RequestReceived,
    UpstreamResponseComplete(u16),
    FaultFired,
    DownstreamResponse(u16),
    ProxyError,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub seq: usize,
    pub attempt: u32,
    pub kind: EventKind,
}

#[derive(Clone)]
struct Shared {
    config: ProxyConfig,
    fired: Arc<AtomicBool>,
    attempts: Arc<AtomicU32>,
    events: Arc<Mutex<Vec<Event>>>,
}

impl Shared {
    async fn log(&self, attempt: u32, kind: EventKind) {
        let mut events = self.events.lock().await;
        let seq = events.len() + 1;
        events.push(Event { seq, attempt, kind });
    }
}

pub struct Proxy {
    pub address: SocketAddr,
    shared: Shared,
    stop: Option<oneshot::Sender<()>>,
    server: JoinHandle<()>,
}

impl Proxy {
    pub async fn start(config: ProxyConfig) -> io::Result<Self> {
        config.validate()?;
        let bind = SocketAddr::new(IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), 0);
        let listener = TcpListener::bind(bind).await?;
        let address = listener.local_addr()?;
        let shared = Shared {
            config,
            fired: Arc::new(AtomicBool::new(false)),
            attempts: Arc::new(AtomicU32::new(0)),
            events: Arc::new(Mutex::new(Vec::new())),
        };
        let (stop, stopped) = oneshot::channel();
        let worker = shared.clone();
        let server = tokio::spawn(async move {
            serve(listener, worker, stopped).await;
        });
        Ok(Self { address, shared, stop: Some(stop), server })
    }

    pub fn fault_fired(&self) -> bool {
        self.shared.fired.load(Ordering::Acquire)
    }

    pub async fn events(&self) -> Vec<Event> {
        self.shared.events.lock().await.clone()
    }

    /// Stop accepting, drain worker tasks; overdue tasks are cancelled and joined.
    pub async fn shutdown(mut self) -> Vec<Event> {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        let _ = (&mut self.server).await;
        self.events().await
    }
}

impl Drop for Proxy {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        // shutdown() awaits clean completion. A dropped owner aborts the server.
        self.server.abort();
    }
}

async fn serve(listener: TcpListener, shared: Shared, mut stop: oneshot::Receiver<()>) {
    let permits = Arc::new(Semaphore::new(MAX_CONNECTIONS));
    let mut workers = JoinSet::new();
    loop {
        tokio::select! {
            biased;
            _ = &mut stop => break,
            permit = permits.clone().acquire_owned() => {
                let Ok(permit) = permit else { break };
                tokio::select! {
                    biased;
                    _ = &mut stop => break,
                    accepted = listener.accept() => {
                        if let Ok((client, remote)) = accepted {
                            if !remote.ip().is_loopback() {
                                continue;
                            }
                            let task_shared = shared.clone();
                            let attempt = shared.attempts.fetch_add(1, Ordering::AcqRel) + 1;
                            workers.spawn(async move {
                                let _permit = permit;
                                if handle_connection(client, task_shared.clone(), attempt).await.is_err() {
                                    task_shared.log(attempt, EventKind::ProxyError).await;
                                }
                            });
                        } else {
                            break;
                        }
                    }
                }
            }
        }
    }
    // Waiting for tasks to finish is bounded. No detached proxy tasks survive.
    if tokio::time::timeout(Duration::from_secs(5), async {
        while workers.join_next().await.is_some() {}
    }).await.is_err() {
        workers.abort_all();
        while workers.join_next().await.is_some() {}
    }
}

fn invalid(s: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, s)
}

fn invalid_data(s: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, s)
}

fn valid_path(path: &str) -> bool {
    path.starts_with('/')
        && !path.starts_with("//")
        && path.len() <= 2048
        && !path.contains(['?', '#', '\\', '\r', '\n', ' '])
        && !path.split('/').any(|segment| segment == ".." || segment == ".")
}

#[derive(Debug)]
struct Message {
    bytes: Vec<u8>,
    status: Option<u16>,
    method: Option<String>,
    path: Option<String>,
}

async fn read_message(stream: &mut TcpStream, max_body: usize, request: bool) -> io::Result<Message> {
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        if head.len() >= MAX_HEADER_BYTES {
            return Err(invalid_data("HTTP header exceeds 16 KiB"));
        }
        let read = stream.read(&mut byte).await?;
        if read == 0 {
            return if head.is_empty() {
                Err(io::Error::new(io::ErrorKind::ConnectionAborted, "no response bytes received"))
            } else {
                Err(io::Error::new(io::ErrorKind::UnexpectedEof, "partial HTTP header"))
            };
        }
        head.push(byte[0]);
        if head.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let header = std::str::from_utf8(&head).map_err(|_| invalid_data("non-UTF8 HTTP header"))?;
    let mut lines = header.split("\r\n");
    let first = lines.next().ok_or_else(|| invalid_data("missing HTTP start line"))?;
    let (status, method, path) = if request {
        let parts: Vec<_> = first.split(' ').collect();
        if parts.len() != 3 || parts[0] != "POST" || parts[2] != "HTTP/1.1" || !valid_path(parts[1]) {
            return Err(invalid_data("only POST with an absolute HTTP/1.1 path is supported"));
        }
        (None, Some(parts[0].to_owned()), Some(parts[1].to_owned()))
    } else {
        let mut parts = first.splitn(3, ' ');
        if parts.next() != Some("HTTP/1.1") {
            return Err(invalid_data("upstream must speak HTTP/1.1"));
        }
        let code = parts.next().ok_or_else(|| invalid_data("missing status"))?;
        let parsed: u16 = code.parse().map_err(|_| invalid_data("invalid HTTP status"))?;
        if !(200..=599).contains(&parsed) || code.len() != 3 {
            return Err(invalid_data("unsupported HTTP status"));
        }
        (Some(parsed), None, None)
    };
    let mut content_length = None;
    for line in lines {
        if line.is_empty() { continue; }
        if line.starts_with([' ', '\t']) {
            return Err(invalid_data("folded headers not supported"));
        }
        let (name, value) = line.split_once(':').ok_or_else(|| invalid_data("malformed HTTP header"))?;
        if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
            return Err(invalid_data("invalid header name"));
        }
        if value.bytes().any(|b| b == b'\r' || b == b'\n' || b == 0) {
            return Err(invalid_data("invalid header value"));
        }
        if name.eq_ignore_ascii_case("transfer-encoding") || name.eq_ignore_ascii_case("upgrade") {
            return Err(invalid_data("chunked/upgrade framing unsupported"));
        }
        if name.eq_ignore_ascii_case("content-length") {
            if content_length.is_some() {
                return Err(invalid_data("ambiguous duplicate content-length"));
            }
            let len = value.trim().parse::<usize>().map_err(|_| invalid_data("bad content-length"))?;
            content_length = Some(len);
        }
    }
    let length = content_length.ok_or_else(|| invalid_data("content-length required"))?;
    if length > max_body {
        return Err(invalid_data("HTTP payload exceeds configured bound"));
    }
    let mut body = vec![0u8; length];
    stream.read_exact(&mut body).await?;
    let mut bytes = head;
    bytes.extend(body);
    Ok(Message { bytes, status, method, path })
}

async fn handle_connection(mut client: TcpStream, shared: Shared, attempt: u32) -> io::Result<()> {
    let config = &shared.config;
    let request = tokio::time::timeout(
        config.upstream_timeout, read_message(&mut client, config.max_request_bytes, true),
    ).await.map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "request deadline"))??;
    shared.log(attempt, EventKind::RequestReceived).await;

    let mut upstream = tokio::time::timeout(config.upstream_timeout, TcpStream::connect(config.upstream))
        .await.map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "connect deadline"))??;
    tokio::time::timeout(config.upstream_timeout, upstream.write_all(&request.bytes))
        .await.map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "write deadline"))??;
    let response = tokio::time::timeout(
        config.upstream_timeout, read_message(&mut upstream, config.max_response_bytes, false),
    ).await.map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "response deadline"))??;
    let status = response.status.expect("response parser supplies status");
    shared.log(attempt, EventKind::UpstreamResponseComplete(status)).await;

    let eligible = request.method.as_deref() == Some("POST")
        && request.path.as_deref() == Some(config.fault_path.as_str())
        && (200..300).contains(&status);
    if eligible
        && shared.fired.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_ok()
    {
        // The entire upstream response was read, but *no downstream bytes have
        // ever been written*. Dropping this socket produces EOF/transport failure.
        shared.log(attempt, EventKind::FaultFired).await;
        drop(client);
        return Ok(());
    }
    client.write_all(&response.bytes).await?;
    client.flush().await?;
    shared.log(attempt, EventKind::DownstreamResponse(status)).await;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Attempt {
    TransportError,
    Http { status: u16, body: Vec<u8> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkloadResult {
    pub attempts: Vec<Attempt>,
}

/// Execute an identical two-attempt POST; ordinary HTTP responses never trigger
/// retries. A zero-byte transport EOF is retryable; malformed partial HTTP is not.
pub async fn retry_post(
    proxy_address: SocketAddr,
    path: &str,
    idempotency_key: &str,
    json: &[u8],
    max_request_bytes: usize,
    max_response_bytes: usize,
    deadline: Duration,
) -> io::Result<WorkloadResult> {
    if !proxy_address.ip().is_loopback() || !valid_path(path)
        || idempotency_key.is_empty() || idempotency_key.len() > 128
        || !idempotency_key.bytes().all(|b| (33..=126).contains(&b))
        || json.len() > max_request_bytes || max_request_bytes > 65_536
        || max_response_bytes == 0 || max_response_bytes > 65_536
        || deadline.is_zero()
    {
        return Err(invalid("invalid workload configuration"));
    }
    let header = format!(
        "POST {path} HTTP/1.1\r\nHost: {proxy_address}\r\nContent-Type: application/json\r\nIdempotency-Key: {idempotency_key}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        json.len()
    );
    let mut request = header.into_bytes();
    request.extend_from_slice(json);
    let mut attempts = Vec::new();
    for _ in 0..2 {
        let outcome = tokio::time::timeout(deadline, async {
            let mut socket = TcpStream::connect(proxy_address).await?;
            socket.write_all(&request).await?;
            match read_message(&mut socket, max_response_bytes, false).await {
                Ok(response) => Ok(Attempt::Http {
                    status: response.status.expect("response parser supplies status"),
                    body: response.bytes.split_at(response.bytes.iter().position(|_| false).unwrap_or(0)).1.to_vec(),
                }),
                Err(e) if e.kind() == io::ErrorKind::ConnectionAborted || e.kind() == io::ErrorKind::ConnectionReset => {
                    Ok(Attempt::TransportError)
                }
                Err(e) => Err(e),
            }
        }).await.map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "client deadline"))??;
        let retry = outcome == Attempt::TransportError;
        attempts.push(outcome);
        if !retry { break; }
    }
    Ok(WorkloadResult { attempts })
}
