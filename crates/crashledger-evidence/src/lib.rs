//! Bounded evidence v1. Integrity against accidental corruption, not a signature
//! or an attestation. The reporter rechecks semantics without running a target.
mod report;
mod writer;

pub use report::{read_report, VerifiedReport};
pub use writer::{write_bundle, Capture};

use crashledger_core::Verdict;
use serde::{Deserialize, Serialize};
use std::io;

pub const SCHEMA_VERSION: u32 = 1;
pub const FILES: [&str; 5] = [
    "scenario.snapshot.yaml",
    "run.json",
    "events.jsonl",
    "observation.json",
    "verdict.json",
];
pub const MANIFEST: &str = "manifest.sha256";
pub const MAX_FILE: usize = 128 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum EvidenceError {
    #[error("evidence I/O: {0}")]
    Io(#[from] io::Error),
    #[error("evidence JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("evidence YAML: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("invalid evidence: {0}")]
    Invalid(String),
}

fn invalid(message: impl Into<String>) -> EvidenceError {
    EvidenceError::Invalid(message.into())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunMetadata {
    pub schema_version: u32,
    pub run_id: String,
    pub scenario_id: String,
    pub fixture_mode: String,
    pub tool_version: String,
    pub source_commit: Option<String>,
    pub platform: String,
    pub recorded_unix_ms: u128,
    pub scenario_sha256: String,
    pub snapshot_redacted: bool,
    pub upstream_timeout_ms: u64,
    pub experiment_timeout_ms: u64,
    pub attempts: usize,
    pub matching_faults_fired: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub schema_version: u32,
    pub kind: String,
    pub read_only: bool,
    pub expected: i64,
    pub observed: Option<i64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinalVerdict {
    pub schema_version: u32,
    pub verdict: Verdict,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceEvent {
    pub seq: usize,
    pub run_id: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<i64>,
}
