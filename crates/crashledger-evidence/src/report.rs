use crate::{
    invalid, EvidenceError, EvidenceEvent, FinalVerdict, Observation, RunMetadata, FILES, MANIFEST,
    MAX_FILE, SCHEMA_VERSION,
};
use crashledger_core::{evaluate, Evaluation, Verdict};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct VerifiedReport {
    pub schema_version: u32,
    pub run_id: String,
    pub scenario_id: String,
    pub fixture_mode: String,
    pub verdict: Verdict,
    pub reason: String,
    pub expected: i64,
    pub observed: Option<i64>,
    pub fault_fired: usize,
    pub attempts: usize,
    pub retry_occurred: bool,
    pub final_http_status: Option<u16>,
    pub manifest_verified: bool,
    pub tool_version: String,
    pub source_commit: Option<String>,
}
impl VerifiedReport {
    pub fn text(&self) -> String {
        format!(
            "Evidence: VERIFIED (SHA-256, schema v{})\nRun: {}\nScenario: {}\nFixture mode: {}\nFault fired: {}\nAttempts: {} (retry: {})\nFinal HTTP status: {}\nExpected orders: {}\nObserved orders: {}\nVerdict: {} ({})\n",
            self.schema_version, self.run_id, self.scenario_id, self.fixture_mode,
            self.fault_fired, self.attempts, self.retry_occurred,
            self.final_http_status.map_or_else(|| "none".into(), |s| s.to_string()),
            self.expected,
            self.observed.map_or_else(|| "unavailable".into(), |v| v.to_string()),
            match self.verdict { Verdict::Pass => "PASS", Verdict::Fail => "FAIL", Verdict::Inconclusive => "INCONCLUSIVE", Verdict::Error => "ERROR" }, self.reason,
        )
    }
}
fn read_file(directory: &Path, name: &str, limit: usize) -> Result<Vec<u8>, EvidenceError> {
    let path = directory.join(name);
    // Never follow a symlink within an artifact bundle.
    if !fs::symlink_metadata(&path)?.file_type().is_file() {
        return Err(invalid(format!("{name}: not a regular file")));
    }
    if fs::metadata(&path)?.len() > limit as u64 {
        return Err(invalid(format!("{name}: exceeds file limit")));
    }
    let data = fs::read(path)?;
    if data.len() > limit {
        return Err(invalid(format!("{name}: oversized during read")));
    }
    Ok(data)
}

fn valid_digest(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

/// Verify all five exact, named artifacts against the last-written manifest.
/// Any missing, replaced, truncated, appended or reordered entry is a hard error.
fn verified_files(directory: &Path) -> Result<Vec<Vec<u8>>, EvidenceError> {
    if !fs::symlink_metadata(directory)?.file_type().is_dir() {
        return Err(invalid("bundle root is not a real directory"));
    }
    let manifest = read_file(directory, MANIFEST, 4096)?;
    let text = std::str::from_utf8(&manifest).map_err(|_| invalid("invalid UTF-8 manifest"))?;
    let records = text.lines().collect::<Vec<_>>();
    if records.len() != FILES.len() || !text.ends_with('\n') {
        return Err(invalid("incomplete or malformed manifest"));
    }
    let mut verified = Vec::new();
    for (line, expected_name) in records.iter().zip(FILES) {
        let (declared, file) = line
            .split_once("  ")
            .ok_or_else(|| invalid("malformed SHA-256 entry"))?;
        if file != expected_name || !valid_digest(declared) {
            return Err(invalid("manifest file order/name/digest invalid"));
        }
        let bytes = read_file(directory, expected_name, MAX_FILE)?;
        let actual = format!("{:x}", Sha256::digest(&bytes));
        if declared != actual {
            return Err(invalid(format!("{expected_name}: SHA-256 mismatch")));
        }
        verified.push(bytes);
    }
    Ok(verified)
}

#[derive(Default)]
struct Trace {
    sent: bool,
    received: bool,
    upstream: Option<u16>,
    fault: bool,
    downstream: Option<u16>,
    transport_error: bool,
    client_response: Option<u16>,
}

pub fn read_report(directory: &Path) -> Result<VerifiedReport, EvidenceError> {
    let data = verified_files(directory)?;
    let _: serde_yaml::Value = serde_yaml::from_slice(&data[0])?;
    let run: RunMetadata = serde_json::from_slice(&data[1])?;
    let observed: Observation = serde_json::from_slice(&data[3])?;
    let verdict: FinalVerdict = serde_json::from_slice(&data[4])?;

    if run.schema_version != SCHEMA_VERSION
        || observed.schema_version != SCHEMA_VERSION
        || verdict.schema_version != SCHEMA_VERSION
        || !run.snapshot_redacted
        || run.tool_version.is_empty()
        || run.platform.is_empty()
        || run.recorded_unix_ms == 0
        || !valid_digest(&run.scenario_sha256)
        || !["broken", "fixed", "external"].contains(&run.fixture_mode.as_str())
        || run.scenario_id.is_empty()
        || run.upstream_timeout_ms == 0
        || run.experiment_timeout_ms == 0
        || run.attempts > 2
        || run.run_id != directory.file_name().and_then(|f| f.to_str()).unwrap_or("")
        || observed.kind != "sqlite_scalar"
        || !observed.read_only
        || (observed.observed.is_some() == observed.error.is_some())
        || observed
            .error
            .as_deref()
            .is_some_and(|s| s != "oracle_unavailable")
    {
        return Err(invalid(
            "unsupported, malformed, or inconsistent evidence metadata",
        ));
    }

    let mut traces = [Trace::default(), Trace::default()];
    let mut events = Vec::new();
    if data[2].is_empty() || !data[2].ends_with(b"\n") {
        return Err(invalid(
            "JSONL must contain complete newline-terminated events",
        ));
    }
    for line in data[2].split(|b| *b == b'\n').filter(|s| !s.is_empty()) {
        if events.len() >= 128 {
            return Err(invalid("event count exceeds 128"));
        }
        let event: EvidenceEvent = serde_json::from_slice(line)?;
        if event.seq != events.len() + 1 || event.run_id != run.run_id {
            return Err(invalid("non-monotonic or foreign event"));
        }
        events.push(event);
    }
    if events.is_empty()
        || events
            .last()
            .is_none_or(|e| e.kind != "oracle_observation" && e.kind != "oracle_unavailable")
    {
        return Err(invalid("final event must be the independent oracle"));
    }

    let mut fired = 0;
    let mut proxy_errors = 0;
    for (index, event) in events.iter().enumerate() {
        let last = index + 1 == events.len();
        if last {
            match event.kind.as_str() {
                "oracle_observation"
                    if observed.observed == event.value
                        && event.value.is_some()
                        && event.attempt.is_none()
                        && event.status.is_none() => {}
                "oracle_unavailable"
                    if observed.observed.is_none()
                        && event.value.is_none()
                        && event.attempt.is_none()
                        && event.status.is_none() => {}
                _ => return Err(invalid("oracle event does not match separate observation")),
            }
            continue;
        }
        if event.value.is_some() {
            return Err(invalid("non-oracle event carries observation value"));
        }
        let attempt = event
            .attempt
            .ok_or_else(|| invalid("transport event without attempt"))?;
        if attempt == 0 || attempt as usize > run.attempts {
            return Err(invalid("transport event attempt out of range"));
        }
        let prior_received = attempt == 1 || traces[(attempt - 2) as usize].received;
        let trace = &mut traces[(attempt - 1) as usize];
        match event.kind.as_str() {
            "request_sent" if event.status.is_none() && !trace.sent => {
                if !prior_received {
                    return Err(invalid("retry occurs before prior request"));
                }
                trace.sent = true;
            }
            "request_received" if event.status.is_none() && trace.sent && !trace.received => {
                trace.received = true;
            }
            "upstream_response_complete"
                if trace.received
                    && trace.upstream.is_none()
                    && event.status.is_some_and(|s| (200..=599).contains(&s)) =>
            {
                trace.upstream = event.status;
            }
            "fault_fired"
                if trace.upstream.is_some_and(|s| (200..300).contains(&s))
                    && !trace.fault
                    && event.status.is_none()
                    && trace.downstream.is_none() =>
            {
                trace.fault = true;
                fired += 1;
            }
            "downstream_response"
                if trace.upstream.is_some()
                    && !trace.fault
                    && trace.downstream.is_none()
                    && event.status == trace.upstream =>
            {
                trace.downstream = event.status;
            }
            "client_transport_error"
                if trace.fault && !trace.transport_error && event.status.is_none() =>
            {
                trace.transport_error = true;
            }
            "client_response"
                if trace.downstream.is_some()
                    && trace.client_response.is_none()
                    && event.status == trace.downstream =>
            {
                trace.client_response = event.status;
            }
            "proxy_error" if event.status.is_none() && trace.received => {
                proxy_errors += 1;
            }
            _ => {
                return Err(invalid(format!(
                    "invalid or causally inconsistent event: {}",
                    event.kind
                )))
            }
        }
    }
    if fired != run.matching_faults_fired
        || fired > 1
        || traces
            .iter()
            .take(run.attempts)
            .any(|t| !t.sent || !t.received)
        || traces
            .iter()
            .skip(run.attempts)
            .any(|t| t.sent || t.received)
    {
        return Err(invalid(
            "fault and attempt metadata contradict event history",
        ));
    }
    let first_was_transport_error = traces[0].transport_error;
    let retry_occurred = run.attempts == 2 && first_was_transport_error;
    // The core requires both attempts and the retry's HTTP success. Do not
    // permit an HTTP response for the faulted attempt.
    if traces
        .iter()
        .any(|t| t.transport_error && t.client_response.is_some())
    {
        return Err(invalid("contradictory transport/client response"));
    }
    let final_status = if retry_occurred {
        traces[1].client_response
    } else {
        None
    };
    let decision = evaluate(&Evaluation {
        infrastructure_failed: proxy_errors > 0,
        matching_faults_fired: fired as u32,
        required_matching_faults: 1,
        attempts_sent: run.attempts as u32,
        required_attempts: 2,
        final_http_status: final_status,
        independent_observation: observed.observed,
        expected_integer: observed.expected,
    });
    // Stored claims do not override the replayed evaluation.
    if decision.verdict != verdict.verdict || decision.reason != verdict.reason {
        return Err(invalid(
            "recorded verdict does not match recomputed verdict",
        ));
    }
    Ok(VerifiedReport {
        schema_version: SCHEMA_VERSION,
        run_id: run.run_id,
        scenario_id: run.scenario_id,
        fixture_mode: run.fixture_mode,
        verdict: decision.verdict,
        reason: decision.reason.into(),
        expected: observed.expected,
        observed: observed.observed,
        fault_fired: fired,
        attempts: run.attempts,
        retry_occurred,
        final_http_status: final_status,
        manifest_verified: true,
        tool_version: run.tool_version,
        source_commit: run.source_commit,
    })
}
