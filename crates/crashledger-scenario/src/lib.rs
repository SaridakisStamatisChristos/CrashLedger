//! Typed, closed v1 scenario contract. Parsing and validation have no target I/O.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::net::IpAddr;
use std::path::{Component, Path};
use url::{Host, Url};

#[derive(Debug, thiserror::Error)]
pub enum ScenarioError {
    #[error("cannot read scenario: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid YAML/schema: {0}")]
    Parse(#[from] serde_yaml::Error),
    #[error("invalid scenario: {0}")]
    Validation(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub schema_version: u32,
    pub id: String,
    pub target: Target,
    pub workload: Workload,
    pub fault: Fault,
    pub oracle: Oracle,
    pub checks: Checks,
    pub limits: Limits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Workload {
    pub request: Request,
    pub retry: Retry,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub method: Method,
    pub path: String,
    pub headers: BTreeMap<String, String>,
    pub json: Value,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Method {
    #[serde(rename = "POST")]
    Post,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Retry {
    pub max_attempts: u32,
    pub retry_on: Vec<RetryReason>,
    pub backoff_ms: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RetryReason {
    TransportError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fault {
    pub kind: FaultKind,
    #[serde(rename = "match")]
    pub matcher: FaultMatch,
    pub when: FaultWhen,
    pub require_upstream_status_class: StatusClass,
    pub max_fires: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FaultKind {
    SuppressResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FaultMatch {
    pub method: Method,
    pub path: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FaultWhen {
    UpstreamResponseComplete,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum StatusClass {
    #[serde(rename = "2xx")]
    Success,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Oracle {
    pub kind: OracleKind,
    pub database_path: String,
    pub sql: String,
    pub parameters: Vec<Value>,
    pub expected_integer: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OracleKind {
    SqliteScalar,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Checks {
    pub matching_faults_fired: u32,
    pub sent_attempts: u32,
    pub final_response_status_class: StatusClass,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub experiment_timeout_ms: u64,
    pub upstream_timeout_ms: u64,
    pub max_request_bytes: usize,
    pub max_response_bytes: usize,
}

fn reject(message: impl Into<String>) -> ScenarioError {
    ScenarioError::Validation(message.into())
}

fn check_path(path: &str) -> Result<(), ScenarioError> {
    if !path.starts_with('/')
        || path.starts_with("//")
        || path.contains(['?', '#', '\\', '\r', '\n'])
        || path.split('/').any(|p| p == "." || p == "..")
        || path.len() > 2048
    {
        return Err(reject("path must be a bounded absolute HTTP path without query or traversal"));
    }
    Ok(())
}

fn check_url(raw: &str) -> Result<(), ScenarioError> {
    let url = Url::parse(raw).map_err(|_| reject("invalid target URL"))?;
    if url.scheme() != "http"
        || url.port().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != "/"
    {
        return Err(reject("target must be a plain HTTP loopback origin with explicit port"));
    }
    let loopback = match url.host() {
        Some(Host::Ipv4(ip)) => IpAddr::V4(ip).is_loopback(),
        Some(Host::Ipv6(ip)) => IpAddr::V6(ip).is_loopback(),
        _ => false, // Deliberately reject DNS names to avoid name-rebinding surprises.
    };
    if !loopback {
        return Err(reject("non-loopback or DNS target is forbidden in schema v1"));
    }
    Ok(())
}

fn check_headers(headers: &BTreeMap<String, String>) -> Result<(), ScenarioError> {
    if headers.len() > 32 {
        return Err(reject("too many HTTP headers"));
    }
    for (k, v) in headers {
        if k.is_empty()
            || !k.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
            || v.len() > 1024
            || v.bytes().any(|c| c == b'\r' || c == b'\n' || c == 0)
        {
            return Err(reject("malformed HTTP header name or value"));
        }
        if ["host", "connection", "transfer-encoding", "content-length", "upgrade", "proxy-authorization"]
            .iter()
            .any(|h| k.eq_ignore_ascii_case(h))
        {
            return Err(reject("hop-by-hop/framing headers are managed by the driver"));
        }
    }
    let find = |name: &str| {
        headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
    };
    let content_type = find("Content-Type").ok_or_else(|| reject("Content-Type is required"))?;
    if !content_type.eq_ignore_ascii_case("application/json") {
        return Err(reject("Content-Type must equal application/json"));
    }
    let key = find("Idempotency-Key").ok_or_else(|| reject("Idempotency-Key is required"))?;
    if key.is_empty() || key.len() > 128 || key.trim() != key || !key.is_ascii() {
        return Err(reject("Idempotency-Key must be 1..128 trimmed ASCII bytes"));
    }
    Ok(())
}

impl Scenario {
    pub fn validate(&self) -> Result<(), ScenarioError> {
        if self.schema_version != 1 {
            return Err(reject("unsupported schema_version (expected 1)"));
        }
        if self.id.is_empty()
            || self.id.len() > 64
            || !self.id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        {
            return Err(reject("id must be 1..64 lowercase ASCII letters, digits or hyphens"));
        }
        check_url(&self.target.base_url)?;
        check_path(&self.workload.request.path)?;
        check_path(&self.fault.matcher.path)?;
        check_headers(&self.workload.request.headers)?;
        if !self.workload.request.json.is_object() {
            return Err(reject("v1 requires a JSON object request"));
        }
        let size = serde_json::to_vec(&self.workload.request.json)
            .map_err(|_| reject("cannot serialize request JSON"))?
            .len();
        if self.limits.max_request_bytes == 0
            || self.limits.max_request_bytes > 65536
            || self.limits.max_response_bytes == 0
            || self.limits.max_response_bytes > 65536
            || size > self.limits.max_request_bytes
        {
            return Err(reject("request/response size bounds must be 1..65536 and fit request JSON"));
        }
        if self.workload.retry.max_attempts != 2
            || self.workload.retry.retry_on != [RetryReason::TransportError]
            || self.workload.retry.backoff_ms > 1000
        {
            return Err(reject("v1 requires exactly two transport-error attempts and backoff <= 1000ms"));
        }
        if self.fault.max_fires != 1
            || self.checks.matching_faults_fired != 1
            || self.checks.sent_attempts != 2
        {
            return Err(reject("v1 requires one fault and two attempted requests"));
        }
        if self.limits.experiment_timeout_ms == 0
            || self.limits.experiment_timeout_ms > 120000
            || self.limits.upstream_timeout_ms == 0
            || self.limits.upstream_timeout_ms >= self.limits.experiment_timeout_ms
        {
            return Err(reject("timeout bounds invalid or upstream timeout not less than total"));
        }
        let db = Path::new(&self.oracle.database_path);
        if self.oracle.database_path.is_empty()
            || db.is_absolute()
            || db.components().any(|c| matches!(c, Component::ParentDir | Component::RootDir | Component::Prefix(_)))
        {
            return Err(reject("oracle database path must be relative without traversal"));
        }
        // At runtime PR-04 additionally pins the scenario working directory and
        // opens the database read-only; this parse-time check alone is not a sandbox.
        let query = self.oracle.sql.trim();
        if query.len() > 4096
            || query.contains(';')
            || !query.get(..6).is_some_and(|s| s.eq_ignore_ascii_case("SELECT"))
            || !query.as_bytes().get(6).is_some_and(u8::is_ascii_whitespace)
            || self.oracle.parameters.len() > 16
        {
            return Err(reject("oracle must be a single bounded parameterized SELECT"));
        }
        for value in &self.oracle.parameters {
            if !value.is_null() && !value.is_boolean() && !value.is_string() && !value.is_number() {
                return Err(reject("SQL parameters must be scalar JSON values"));
            }
        }
        Ok(())
    }
}

pub fn parse_str(yaml: &str) -> Result<Scenario, ScenarioError> {
    let scenario: Scenario = serde_yaml::from_str(yaml)?;
    scenario.validate()?;
    Ok(scenario)
}

pub fn load_file(path: impl AsRef<Path>) -> Result<Scenario, ScenarioError> {
    let content = std::fs::read_to_string(path)?;
    parse_str(&content)
}

#[cfg(test)]
mod tests {
    use super::*;
    const GOLDEN: &str = include_str!("../../../scenarios/lost-ack.yaml");

    fn bad(from: &str, to: &str) {
        let changed = GOLDEN.replacen(from, to, 1);
        assert_ne!(changed, GOLDEN, "test fixture replacement failed");
        assert!(parse_str(&changed).is_err(), "invalid scenario was accepted");
    }

    #[test]
    fn canonical_scenario_is_valid_and_roundtrips() {
        let parsed = parse_str(GOLDEN).expect("golden scenario");
        assert_eq!(parsed.id, "lost-ack-order-retry");
        let encoded = serde_yaml::to_string(&parsed).expect("serialize");
        let again = parse_str(&encoded).expect("roundtrip");
        assert_eq!(again.workload.request.path, "/orders");
    }

    #[test]
    fn rejects_unknown_fields_and_versions() {
        bad("schema_version: 1", "schema_version: 2");
        bad("schema_version: 1", "schema_version: 1\nunknown: true");
        bad("    max_attempts: 2", "    max_attempts: 2\n    unknown: 3");
        bad("  max_fires: 1", "  max_fires: 1\n  unexpected: true");
    }

    #[test]
    fn rejects_remote_and_unsafe_urls() {
        bad("127.0.0.1:4001", "8.8.8.8:4001");
        bad("http://127.0.0.1:4001", "https://127.0.0.1:4001");
        bad("127.0.0.1:4001", "localhost:4001");
        bad("127.0.0.1:4001", "127.0.0.1:4001/private");
        bad("127.0.0.1:4001", "127.0.0.1:4001?to=remote");
    }

    #[test]
    fn rejects_unbounded_and_unsupported_runs() {
        bad("max_fires: 1", "max_fires: 2");
        bad("max_attempts: 2", "max_attempts: 0");
        bad("max_attempts: 2", "max_attempts: 3");
        bad("max_request_bytes: 65536", "max_request_bytes: 999999999");
        bad("upstream_timeout_ms: 3000", "upstream_timeout_ms: 20000");
        bad("retry_on: [transport_error]", "retry_on: [server_error]");
        bad("kind: sqlite_scalar", "kind: endpoint_status");
    }

    #[test]
    fn rejects_bad_oracle_and_headers() {
        bad("./run-data/orders.sqlite", "../outside.sqlite");
        bad("SELECT COUNT(*) FROM orders WHERE idempotency_key = ?1",
            "DELETE FROM orders");
        bad("SELECT COUNT(*) FROM orders WHERE idempotency_key = ?1",
            "SELECT COUNT(*) FROM orders; DELETE FROM orders");
        bad("Idempotency-Key: demo-order-0001", "Idempotency-Key: ''");
        bad("Content-Type: application/json", "Content-Type: text/plain");
    }

    #[test]
    fn wrong_fault_path_is_allowed_as_negative_control() {
        let changed = GOLDEN.replacen("    path: /orders\n  when:",
                                      "    path: /not-orders\n  when:", 1);
        assert_ne!(changed, GOLDEN);
        assert!(parse_str(&changed).is_ok());
    }
}
