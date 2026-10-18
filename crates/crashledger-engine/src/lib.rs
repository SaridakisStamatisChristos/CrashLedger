//! PR-04: explicit demo lifecycle, independent oracle, and fail-closed verdict.
//! Transport and target state are never used as interchangeable evidence.

use crashledger_core::{evaluate, Decision, Evaluation, Phase, Verdict};
use crashledger_http::{retry_post, Attempt, Event, EventKind, Proxy, ProxyConfig, WorkloadResult};
use crashledger_oracle::observe_sqlite_scalar;
use crashledger_scenario::{parse_str, Scenario, ScenarioError};
use std::io::{self, BufRead, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

const GOLDEN: &str = include_str!("../../../scenarios/lost-ack.yaml");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemoMode {
    Broken,
    Fixed,
}
impl DemoMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Broken => "broken",
            Self::Fixed => "fixed",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DemoError {
    #[error("I/O failure: {0}")]
    Io(#[from] io::Error),
    #[error("scenario rejected: {0}")]
    Scenario(#[from] ScenarioError),
    #[error("JSON failure: {0}")]
    Json(#[from] serde_json::Error),
    #[error("experiment setup/transport error: {0}")]
    Internal(String),
}

pub struct RunSummary {
    pub mode: DemoMode,
    pub scenario_id: String,
    pub run_directory: PathBuf,
    pub decision: Decision,
    pub observation: Option<i64>,
    pub observation_error: Option<String>,
    pub expected: i64,
    pub fault_count: usize,
    pub attempts: Vec<Attempt>,
    pub events: Vec<Event>,
}
impl RunSummary {
    pub fn print(&self) {
        println!("Scenario: {}", self.scenario_id);
        println!("Fixture mode: {}", self.mode.as_str());
        println!(
            "Fault: suppress_response [fired {} time(s) after upstream response]",
            self.fault_count
        );
        let first = match self.attempts.first() {
            Some(Attempt::TransportError) => "transport error".to_owned(),
            Some(Attempt::Http { status, .. }) => format!("HTTP {status}"),
            None => "none".to_owned(),
        };
        let final_attempt = match self.attempts.get(1) {
            Some(Attempt::TransportError) => "transport error".to_owned(),
            Some(Attempt::Http { status, .. }) => format!("HTTP {status}"),
            None => "none".to_owned(),
        };
        println!(
            "Attempts: {} (first: {first}; second: {final_attempt})",
            self.attempts.len()
        );
        match self.observation {
            Some(value) => println!(
                "Independent SQLite observation: orders_for_key = {value}; expected = {}",
                self.expected
            ),
            None => println!(
                "Independent SQLite observation: unavailable ({}); expected = {}",
                self.observation_error.as_deref().unwrap_or("no result"),
                self.expected
            ),
        }
        let label = match self.decision.verdict {
            Verdict::Pass => "PASS",
            Verdict::Fail => "FAIL",
            Verdict::Inconclusive => "INCONCLUSIVE",
            Verdict::Error => "ERROR",
        };
        println!("Verdict: {label} ({})", self.decision.reason);
        println!("Artifacts: {}", self.run_directory.display());
        println!("Transport events: {}", self.events.len());
    }
}

struct OwnedFixture {
    child: Child,
}
impl Drop for OwnedFixture {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn fixture_binary() -> Result<PathBuf, DemoError> {
    if let Some(custom) = std::env::var_os("CRASHLEDGER_FIXTURE_BIN") {
        let path = PathBuf::from(custom);
        if !path.is_file() {
            return Err(DemoError::Internal(
                "CRASHLEDGER_FIXTURE_BIN does not exist".into(),
            ));
        }
        return Ok(path);
    }
    let basename = format!("crashledger-demo{}", std::env::consts::EXE_SUFFIX);
    let sibling = std::env::current_exe()?.with_file_name(&basename);
    if sibling.is_file() {
        return Ok(sibling);
    }
    let root = repository_root();
    // The source-checkout demo uses Cargo if the fixture binary was not yet
    // built. The target service itself is launched as a directly owned process.
    let status = Command::new("cargo")
        .current_dir(&root)
        .args(["build", "--locked", "--quiet", "-p", "crashledger-demo"])
        .status()?;
    if !status.success() {
        return Err(DemoError::Internal(
            "cannot build standalone order fixture".into(),
        ));
    }
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .map(|p| if p.is_absolute() { p } else { root.join(p) })
        .unwrap_or_else(|| root.join("target"));
    let built = target.join("debug").join(basename);
    if !built.is_file() {
        return Err(DemoError::Internal(
            "built fixture binary could not be located".into(),
        ));
    }
    Ok(built)
}

fn check_fixture_readiness(address: SocketAddr) -> Result<(), DemoError> {
    if !address.ip().is_loopback() || address.port() == 0 {
        return Err(DemoError::Internal(
            "fixture advertised unsafe address".into(),
        ));
    }
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(3))?;
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    stream.set_write_timeout(Some(Duration::from_secs(3)))?;
    let request = format!("GET /ready HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes())?;
    let mut reply = Vec::new();
    stream.read_to_end(&mut reply)?;
    if !reply.starts_with(b"HTTP/1.1 200 ") {
        return Err(DemoError::Internal(
            "fixture readiness did not confirm SQLite".into(),
        ));
    }
    Ok(())
}

async fn start_fixture(
    mode: DemoMode,
    path: &Path,
) -> Result<(OwnedFixture, SocketAddr), DemoError> {
    let bin = fixture_binary()?;
    let mut child = Command::new(bin)
        .args(["--mode", mode.as_str(), "--db-path"])
        .arg(path)
        .args(["--listen", "127.0.0.1:0"])
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| DemoError::Internal("fixture stdout not captured".into()))?;
    let owner = OwnedFixture { child };
    let line = tokio::time::timeout(
        Duration::from_secs(30),
        tokio::task::spawn_blocking(move || {
            let mut line = String::new();
            io::BufReader::new(stdout).read_line(&mut line)?;
            Ok::<_, io::Error>(line)
        }),
    )
    .await
    .map_err(|_| DemoError::Internal("fixture readiness timeout".into()))?
    .map_err(|e| DemoError::Internal(format!("fixture readiness join failed: {e}")))??;
    let address = line
        .trim()
        .strip_prefix("READY http://")
        .ok_or_else(|| DemoError::Internal(format!("fixture did not announce READY: {line:?}")))?
        .parse::<SocketAddr>()
        .map_err(|_| DemoError::Internal("invalid fixture listening address".into()))?;
    check_fixture_readiness(address)?;
    Ok((owner, address))
}

fn header<'a>(scenario: &'a Scenario, name: &str) -> Result<&'a str, DemoError> {
    scenario
        .workload
        .request
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
        .ok_or_else(|| DemoError::Internal(format!("missing scenario header {name}")))
}

fn validate_history(events: &[Event]) -> bool {
    let fault = events
        .iter()
        .filter(|e| e.kind == EventKind::FaultFired)
        .collect::<Vec<_>>();
    if fault.len() != 1 {
        return fault.is_empty(); // Missing fault -> INCONCLUSIVE, not an internal contradiction.
    }
    let claimed = fault[0];
    let completed_first = events.iter().any(|e| {
        e.attempt == claimed.attempt
            && e.seq < claimed.seq
            && matches!(e.kind, EventKind::UpstreamResponseComplete(200..=299))
    });
    let no_successful_first_delivery = !events.iter().any(|e| {
        e.attempt == claimed.attempt && matches!(e.kind, EventKind::DownstreamResponse(_))
    });
    completed_first && no_successful_first_delivery
}

pub fn judge(
    events: &[Event],
    result: &WorkloadResult,
    observed: Option<i64>,
    expected: i64,
) -> Decision {
    let fired = events
        .iter()
        .filter(|e| e.kind == EventKind::FaultFired)
        .count();
    let proxy_failed = events.iter().any(|e| e.kind == EventKind::ProxyError);
    let event_count = events
        .iter()
        .filter(|e| e.kind == EventKind::RequestReceived)
        .count();
    let trace_invalid = !validate_history(events)
        || event_count != result.attempts.len()
        || events.iter().enumerate().any(|(i, e)| e.seq != i + 1);
    let proper_retry =
        result.attempts.len() == 2 && result.attempts.first() == Some(&Attempt::TransportError);
    let final_status = if proper_retry {
        match result.attempts.get(1) {
            Some(Attempt::Http { status, .. }) => Some(*status),
            _ => None,
        }
    } else {
        None
    };
    evaluate(&Evaluation {
        infrastructure_failed: proxy_failed || trace_invalid,
        matching_faults_fired: u32::try_from(fired).unwrap_or(u32::MAX),
        required_matching_faults: 1,
        attempts_sent: u32::try_from(result.attempts.len()).unwrap_or(u32::MAX),
        required_attempts: 2,
        final_http_status: final_status,
        independent_observation: observed,
        expected_integer: expected,
    })
}

/// Self-contained canonical demo. The proxy and fixture are quiesced before the
/// SQLite observer runs. Each invocation creates a uniquely isolated run dir.
pub async fn run_demo(mode: DemoMode, artifacts: &Path) -> Result<RunSummary, DemoError> {
    let mut phase = Phase::Loaded;
    let mut scenario = parse_str(GOLDEN)?;
    phase = phase
        .transition(Phase::Validated)
        .map_err(|e| DemoError::Internal(e.to_string()))?;
    std::fs::create_dir_all(artifacts)?;
    let run_dir = tempfile::Builder::new()
        .prefix("r-")
        .tempdir_in(artifacts)?
        .keep();
    let db = run_dir.join("run-data").join("orders.sqlite");
    let (_fixture, upstream) = start_fixture(mode, &db).await?;
    scenario.target.base_url = format!("http://{upstream}");
    scenario.validate()?;
    phase = phase
        .transition(Phase::Ready)
        .map_err(|e| DemoError::Internal(e.to_string()))?;
    let config = ProxyConfig {
        upstream,
        fault_path: scenario.fault.matcher.path.clone(),
        max_request_bytes: scenario.limits.max_request_bytes,
        max_response_bytes: scenario.limits.max_response_bytes,
        upstream_timeout: Duration::from_millis(scenario.limits.upstream_timeout_ms),
    };
    let proxy = Proxy::start(config).await?;
    phase = phase
        .transition(Phase::Executing)
        .map_err(|e| DemoError::Internal(e.to_string()))?;
    let raw_json = serde_json::to_vec(&scenario.workload.request.json)?;
    let retry = tokio::time::timeout(
        Duration::from_millis(scenario.limits.experiment_timeout_ms),
        retry_post(
            proxy.address,
            &scenario.workload.request.path,
            header(&scenario, "Idempotency-Key")?,
            &raw_json,
            scenario.limits.max_request_bytes,
            scenario.limits.max_response_bytes,
            Duration::from_millis(scenario.limits.upstream_timeout_ms),
        ),
    )
    .await;
    phase = phase
        .transition(Phase::Quiescing)
        .map_err(|e| DemoError::Internal(e.to_string()))?;
    let events = proxy.shutdown().await;
    let result = retry
        .map_err(|_| DemoError::Internal("experiment workload timed out".into()))?
        .map_err(|e| DemoError::Internal(format!("experiment workload transport failed: {e}")))?;
    phase = phase
        .transition(Phase::Observing)
        .map_err(|e| DemoError::Internal(e.to_string()))?;
    // This direct connection is READ_ONLY. It is neither shared with the target
    // service nor derived from the target's HTTP response.
    let observation = observe_sqlite_scalar(&db, &scenario.oracle.sql, &scenario.oracle.parameters);
    let (observed, observation_error) = match observation {
        Ok(n) => (Some(n), None),
        Err(e) => (None, Some(e.to_string())),
    };
    phase = phase
        .transition(Phase::Evaluating)
        .map_err(|e| DemoError::Internal(e.to_string()))?;
    let decision = judge(&events, &result, observed, scenario.oracle.expected_integer);
    phase = phase
        .transition(match decision.verdict {
            Verdict::Error => Phase::Error,
            Verdict::Inconclusive => Phase::Inconclusive,
            Verdict::Pass | Verdict::Fail => Phase::Complete,
        })
        .map_err(|e| DemoError::Internal(e.to_string()))?;
    let _terminal = phase;
    Ok(RunSummary {
        mode,
        scenario_id: scenario.id,
        run_directory: run_dir,
        decision,
        observation: observed,
        observation_error,
        expected: scenario.oracle.expected_integer,
        fault_count: events
            .iter()
            .filter(|e| e.kind == EventKind::FaultFired)
            .count(),
        attempts: result.attempts,
        events,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic() -> (Vec<Event>, WorkloadResult) {
        let events = vec![
            Event {
                seq: 1,
                attempt: 1,
                kind: EventKind::RequestReceived,
            },
            Event {
                seq: 2,
                attempt: 1,
                kind: EventKind::UpstreamResponseComplete(201),
            },
            Event {
                seq: 3,
                attempt: 1,
                kind: EventKind::FaultFired,
            },
            Event {
                seq: 4,
                attempt: 2,
                kind: EventKind::RequestReceived,
            },
            Event {
                seq: 5,
                attempt: 2,
                kind: EventKind::UpstreamResponseComplete(200),
            },
            Event {
                seq: 6,
                attempt: 2,
                kind: EventKind::DownstreamResponse(200),
            },
        ];
        let result = WorkloadResult {
            attempts: vec![
                Attempt::TransportError,
                Attempt::Http {
                    status: 200,
                    body: vec![],
                },
            ],
        };
        (events, result)
    }
    #[test]
    fn independent_scalar_is_required_and_drives_verdict() {
        let (events, result) = synthetic();
        assert_eq!(judge(&events, &result, Some(1), 1).verdict, Verdict::Pass);
        assert_eq!(judge(&events, &result, Some(2), 1).verdict, Verdict::Fail);
        assert_eq!(
            judge(&events, &result, None, 1).verdict,
            Verdict::Inconclusive
        );
    }
    #[test]
    fn missing_fault_retry_or_transport_sequence_never_passes() {
        let (mut events, mut result) = synthetic();
        events.retain(|e| e.kind != EventKind::FaultFired);
        for (i, e) in events.iter_mut().enumerate() {
            e.seq = i + 1;
        }
        assert_eq!(
            judge(&events, &result, Some(1), 1).verdict,
            Verdict::Inconclusive
        );
        let (events, _) = synthetic();
        result.attempts.truncate(1);
        assert_ne!(judge(&events, &result, Some(1), 1).verdict, Verdict::Pass);
        let (_, mut result) = synthetic();
        result.attempts[0] = Attempt::Http {
            status: 201,
            body: vec![],
        };
        assert_eq!(
            judge(&events, &result, Some(1), 1).verdict,
            Verdict::Inconclusive
        );
    }
    #[test]
    fn mismatched_trace_order_or_proxy_error_fails_closed() {
        let (mut events, result) = synthetic();
        events[1].kind = EventKind::FaultFired;
        events[2].kind = EventKind::UpstreamResponseComplete(201);
        assert_eq!(judge(&events, &result, Some(1), 1).verdict, Verdict::Error);
        let (mut events, result) = synthetic();
        events.push(Event {
            seq: 7,
            attempt: 2,
            kind: EventKind::ProxyError,
        });
        assert_eq!(judge(&events, &result, Some(1), 1).verdict, Verdict::Error);
    }
}
