use crashledger_core::{evaluate, Decision, Evaluation, Verdict};
use crashledger_evidence::{read_report, write_bundle, Capture};
use crashledger_http::{Attempt, Event, EventKind};
use crashledger_scenario::parse_str;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

fn facts() -> (Vec<Event>, Vec<Attempt>) {
    (
        vec![
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
        ],
        vec![
            Attempt::TransportError,
            Attempt::Http {
                status: 200,
                body: b"{\"token\":\"hidden\"}".to_vec(),
            },
        ],
    )
}
fn verdict(observation: Option<i64>, fired: usize, attempts: usize) -> Decision {
    evaluate(&Evaluation {
        infrastructure_failed: false,
        matching_faults_fired: fired as u32,
        required_matching_faults: 1,
        attempts_sent: attempts as u32,
        required_attempts: 2,
        final_http_status: if attempts == 2 { Some(200) } else { None },
        independent_observation: observation,
        expected_integer: 1,
    })
}

fn create(root: &Path, observed: Option<i64>) -> PathBuf {
    let scenario_text = include_str!("../../../scenarios/lost-ack.yaml");
    let mut scenario = parse_str(scenario_text).unwrap();
    scenario
        .workload
        .request
        .headers
        .insert("Authorization".into(), "Bearer SECRET_CREDENTIAL".into());
    scenario.workload.request.json["api_token"] = serde_json::json!("SECRET_BODY_VALUE");
    let (events, attempts) = facts();
    let decision = verdict(observed, 1, 2);
    let dir = root.join("r-test");
    fs::create_dir(&dir).unwrap();
    write_bundle(
        &dir,
        &Capture {
            scenario: &scenario,
            mode: "fixed",
            proxy_events: &events,
            attempts: &attempts,
            observed,
            observation_error: if observed.is_none() {
                Some("sensitive/path/here")
            } else {
                None
            },
            decision: &decision,
        },
    )
    .unwrap();
    dir
}
fn assert_secret_absent(dir: &Path) {
    for file in [
        "scenario.snapshot.yaml",
        "run.json",
        "events.jsonl",
        "observation.json",
        "verdict.json",
        "manifest.sha256",
    ] {
        let s = fs::read_to_string(dir.join(file)).unwrap();
        for secret in [
            "SECRET_CREDENTIAL",
            "SECRET_BODY_VALUE",
            "sensitive/path/here",
            "token\\\":\\\"hidden",
        ] {
            assert!(!s.contains(secret), "{file} leaked secret");
        }
    }
}
#[test]
fn roundtrip_pass_and_redaction() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = create(tmp.path(), Some(1));
    let r = read_report(&dir).unwrap();
    assert_eq!(r.verdict, Verdict::Pass);
    assert!(r.manifest_verified);
    assert_eq!((r.fault_fired, r.attempts, r.retry_occurred), (1, 2, true));
    assert_eq!((r.expected, r.observed), (1, Some(1)));
    assert_secret_absent(&dir);
}
#[test]
fn roundtrip_fail_and_inconclusive() {
    for (val, want) in [(Some(2), Verdict::Fail), (None, Verdict::Inconclusive)] {
        let tmp = tempfile::tempdir().unwrap();
        let dir = create(tmp.path(), val);
        let r = read_report(&dir).unwrap();
        assert_eq!(r.verdict, want);
        assert_eq!(r.observed, val);
        assert_secret_absent(&dir);
    }
}
#[test]
fn corruption_missing_and_spoofed_manifest_are_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = create(tmp.path(), Some(1));
    let target = dir.join("events.jsonl");
    fs::write(&target, b"corrupted\n").unwrap();
    assert!(read_report(&dir).is_err(), "corrupted bytes must reject");

    let tmp = tempfile::tempdir().unwrap();
    let dir = create(tmp.path(), Some(1));
    fs::remove_file(dir.join("observation.json")).unwrap();
    assert!(read_report(&dir).is_err(), "missing artifact must reject");

    let tmp = tempfile::tempdir().unwrap();
    let dir = create(tmp.path(), Some(1));
    let manifest = dir.join("manifest.sha256");
    let lines = fs::read_to_string(&manifest)
        .unwrap()
        .replace("  verdict.json", "  ../verdict.json");
    fs::write(&manifest, lines).unwrap();
    assert!(
        read_report(&dir).is_err(),
        "manifest pathname change must reject"
    );
}
#[test]
fn matching_manifest_cannot_hide_semantic_verdict_forgery() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = create(tmp.path(), Some(1));
    let target = dir.join("verdict.json");
    let mut json: serde_json::Value = serde_json::from_slice(&fs::read(&target).unwrap()).unwrap();
    json["verdict"] = serde_json::json!("FAIL");
    let bytes = serde_json::to_vec_pretty(&json).unwrap();
    fs::write(&target, &bytes).unwrap();
    let m = fs::read_to_string(dir.join("manifest.sha256")).unwrap();
    let mut rebuilt = String::new();
    for line in m.lines() {
        let (_, name) = line.split_once("  ").unwrap();
        let sum = if name == "verdict.json" {
            format!("{:x}", Sha256::digest(&bytes))
        } else {
            line.split_once("  ").unwrap().0.to_string()
        };
        rebuilt.push_str(&format!("{sum}  {name}\n"));
    }
    fs::write(dir.join("manifest.sha256"), rebuilt).unwrap();
    assert!(
        read_report(&dir).is_err(),
        "reader must recompute rather than trust stored verdict"
    );
}
#[test]
fn persistence_failure_is_not_silent() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = create(tmp.path(), Some(1));
    let scenario = parse_str(include_str!("../../../scenarios/lost-ack.yaml")).unwrap();
    let (events, attempts) = facts();
    let decision = verdict(Some(1), 1, 2);
    let err = write_bundle(
        &dir,
        &Capture {
            scenario: &scenario,
            mode: "fixed",
            proxy_events: &events,
            attempts: &attempts,
            observed: Some(1),
            observation_error: None,
            decision: &decision,
        },
    )
    .unwrap_err();
    assert!(
        err.to_string().contains("exists"),
        "create_new refuses overwrite: {err}"
    );
}
