use crate::{
    invalid, EvidenceError, EvidenceEvent, FinalVerdict, Observation, RunMetadata, FILES, MANIFEST,
    MAX_FILE, SCHEMA_VERSION,
};
use crashledger_core::Decision;
use crashledger_http::{Attempt, Event, EventKind};
use crashledger_scenario::Scenario;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Capture<'a> {
    pub scenario: &'a Scenario,
    pub mode: &'a str,
    pub proxy_events: &'a [Event],
    pub attempts: &'a [Attempt],
    pub observed: Option<i64>,
    pub observation_error: Option<&'a str>,
    pub decision: &'a Decision,
}

fn hash(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}

fn save_new(path: &Path, data: &[u8]) -> Result<(), EvidenceError> {
    if data.len() > MAX_FILE {
        return Err(invalid("evidence file exceeds 128 KiB"));
    }
    let mut f = OpenOptions::new().create_new(true).write(true).open(path)?;
    f.write_all(data)?;
    f.sync_all()?;
    Ok(())
}

fn push(
    out: &mut Vec<EvidenceEvent>,
    run: &str,
    kind: &str,
    attempt: Option<u32>,
    status: Option<u16>,
    value: Option<i64>,
) {
    out.push(EvidenceEvent {
        seq: out.len() + 1,
        run_id: run.to_owned(),
        kind: kind.to_owned(),
        attempt,
        status,
        value,
    });
}

pub(crate) fn ordered_events(
    run: &str,
    proxy: &[Event],
    attempts: &[Attempt],
    observation: Option<i64>,
) -> Vec<EvidenceEvent> {
    let mut out = Vec::new();
    for e in proxy {
        if e.kind == EventKind::RequestReceived {
            // RequestSent is inferred from the proxy's observed receipt, not
            // fabricated from an unobserved client-side network callback.
            push(&mut out, run, "request_sent", Some(e.attempt), None, None);
        }
        let (kind, status) = match e.kind {
            EventKind::RequestReceived => ("request_received", None),
            EventKind::UpstreamResponseComplete(status) => {
                ("upstream_response_complete", Some(status))
            }
            EventKind::FaultFired => ("fault_fired", None),
            EventKind::DownstreamResponse(status) => ("downstream_response", Some(status)),
            EventKind::ProxyError => ("proxy_error", None),
        };
        push(&mut out, run, kind, Some(e.attempt), status, None);
        if e.kind == EventKind::FaultFired
            && matches!(
                attempts.get(e.attempt.saturating_sub(1) as usize),
                Some(Attempt::TransportError)
            )
        {
            push(
                &mut out,
                run,
                "client_transport_error",
                Some(e.attempt),
                None,
                None,
            );
        }
        if matches!(e.kind, EventKind::DownstreamResponse(_)) {
            if let Some(Attempt::Http { status, .. }) =
                attempts.get(e.attempt.saturating_sub(1) as usize)
            {
                push(
                    &mut out,
                    run,
                    "client_response",
                    Some(e.attempt),
                    Some(*status),
                    None,
                );
            }
        }
    }
    match observation {
        Some(value) => push(&mut out, run, "oracle_observation", None, None, Some(value)),
        None => push(&mut out, run, "oracle_unavailable", None, None, None),
    }
    out
}

fn safe_snapshot(scenario: &Scenario) -> Result<String, EvidenceError> {
    let mut scrubbed = scenario.clone();
    for (k, value) in &mut scrubbed.workload.request.headers {
        if !k.eq_ignore_ascii_case("content-type") {
            *value = "[REDACTED]".into();
        }
    }
    // No raw request body, token, ID or bound oracle parameter leaks.
    scrubbed.workload.request.json = Value::String("[REDACTED]".into());
    for value in &mut scrubbed.oracle.parameters {
        *value = Value::String("[REDACTED]".into());
    }
    Ok(format!(
        "# REDACTED SNAPSHOT: not executable; original scenario SHA-256 is in run.json\n{}",
        serde_yaml::to_string(&scrubbed)?
    ))
}

pub fn write_bundle(directory: &Path, capture: &Capture<'_>) -> Result<(), EvidenceError> {
    if !directory.is_dir() {
        return Err(invalid("run directory missing"));
    }
    let run_id = directory
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| invalid("invalid run directory name"))?;
    if run_id.len() > 100
        || run_id.is_empty()
        || !run_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(invalid("invalid run identifier"));
    }
    if capture.mode != "broken" && capture.mode != "fixed" {
        return Err(invalid("only canonical fixture modes supported"));
    }
    if capture.proxy_events.len() > 128 || capture.attempts.len() > 2 {
        return Err(invalid("too many transport facts"));
    }
    let canonical = serde_yaml::to_string(capture.scenario)?;
    let snapshot = safe_snapshot(capture.scenario)?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| invalid("system clock before UNIX epoch"))?
        .as_millis();
    let commit = option_env!("GITHUB_SHA").map(str::to_string);
    let metadata = RunMetadata {
        schema_version: SCHEMA_VERSION,
        run_id: run_id.to_owned(),
        scenario_id: capture.scenario.id.clone(),
        fixture_mode: capture.mode.to_owned(),
        tool_version: env!("CARGO_PKG_VERSION").to_owned(),
        source_commit: commit,
        platform: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
        recorded_unix_ms: now,
        scenario_sha256: hash(canonical.as_bytes()),
        snapshot_redacted: true,
        upstream_timeout_ms: capture.scenario.limits.upstream_timeout_ms,
        experiment_timeout_ms: capture.scenario.limits.experiment_timeout_ms,
        attempts: capture.attempts.len(),
        matching_faults_fired: capture
            .proxy_events
            .iter()
            .filter(|e| e.kind == EventKind::FaultFired)
            .count(),
    };
    let obs = Observation {
        schema_version: SCHEMA_VERSION,
        kind: "sqlite_scalar".into(),
        read_only: true,
        expected: capture.scenario.oracle.expected_integer,
        observed: capture.observed,
        // Do not persist raw DB errors: paths and SQL can be sensitive.
        error: capture
            .observation_error
            .map(|_| "oracle_unavailable".to_string()),
    };
    let verdict = FinalVerdict {
        schema_version: SCHEMA_VERSION,
        verdict: capture.decision.verdict,
        reason: capture.decision.reason.to_owned(),
    };
    let events = ordered_events(
        run_id,
        capture.proxy_events,
        capture.attempts,
        capture.observed,
    );
    let mut jsonl = Vec::new();
    for event in events {
        serde_json::to_writer(&mut jsonl, &event)?;
        jsonl.push(b'\n');
    }
    let payloads = [
        snapshot.into_bytes(),
        serde_json::to_vec_pretty(&metadata)?,
        jsonl,
        serde_json::to_vec_pretty(&obs)?,
        serde_json::to_vec_pretty(&verdict)?,
    ];
    let mut manifest = String::new();
    for (name, bytes) in FILES.iter().zip(payloads.iter()) {
        save_new(&directory.join(name), bytes)?;
        manifest.push_str(&format!("{}  {}\n", hash(bytes), name));
    }
    // Commit marker: manifest is created LAST. Readers reject partial bundles.
    save_new(&directory.join(MANIFEST), manifest.as_bytes())?;
    File::open(directory)?.sync_all()?;
    Ok(())
}
