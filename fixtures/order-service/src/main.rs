//! Standalone HTTP order fixture. No dependency on runner, proxy, or evaluator.
mod broken;
mod db;
mod fixed;

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use clap::{Parser, ValueEnum};
use db::{OrderRequest, Outcome};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Mode {
    Broken,
    Fixed,
}

#[derive(Parser)]
#[command(name = "crashledger-demo", about = "Standalone broken/fixed order fixture")]
struct Args {
    #[arg(long, value_enum)]
    mode: Mode,
    #[arg(long)]
    db_path: PathBuf,
    #[arg(long, default_value = "127.0.0.1:0")]
    listen: SocketAddr,
}

#[derive(Clone)]
struct AppState {
    db_path: PathBuf,
    mode: Mode,
}
type Reply = (StatusCode, Json<Value>);

fn error(status: StatusCode, reason: &str) -> Reply {
    (status, Json(json!({"error": reason})))
}

async fn ready(State(state): State<AppState>) -> StatusCode {
    let result = tokio::task::spawn_blocking(move || {
        let conn = db::open(&state.db_path)?;
        conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('orders','idempotency_requests')",
            [],
            |row| row.get::<_, i64>(0),
        )
    }).await;
    match result {
        Ok(Ok(2)) => StatusCode::OK,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    }
}

async fn order(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> Reply {
    let keys = headers.get_all("idempotency-key");
    if keys.iter().count() != 1 {
        return error(StatusCode::BAD_REQUEST, "exactly one Idempotency-Key is required");
    }
    let Some(key) = keys.iter().next().and_then(|v| v.to_str().ok()) else {
        return error(StatusCode::BAD_REQUEST, "invalid Idempotency-Key");
    };
    if key.is_empty() || key.len() > 128 || key.trim() != key || !key.is_ascii() {
        return error(StatusCode::BAD_REQUEST, "invalid Idempotency-Key");
    }
    let types = headers.get_all("content-type");
    if types.iter().count() != 1 ||
        !types.iter().any(|v| v.to_str().is_ok_and(|s| s.eq_ignore_ascii_case("application/json")))
    {
        return error(StatusCode::UNSUPPORTED_MEDIA_TYPE, "application/json required");
    }
    if body.len() > 65_536 {
        return error(StatusCode::PAYLOAD_TOO_LARGE, "request exceeds 65536 bytes");
    }
    let parsed: OrderRequest = match serde_json::from_slice(&body) {
        Ok(value) => value,
        Err(_) => return error(StatusCode::BAD_REQUEST, "invalid order JSON"),
    };
    if !parsed.valid() {
        return error(StatusCode::BAD_REQUEST, "invalid sku or quantity");
    }

    let key = key.to_owned();
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = db::open(&state.db_path)?;
        match state.mode {
            Mode::Broken => broken::insert(&mut conn, &key, &parsed),
            Mode::Fixed => fixed::insert(&mut conn, &key, &parsed, &body),
        }
    }).await;
    match result {
        Ok(Ok(Outcome::Created(id))) => (StatusCode::CREATED, Json(json!({"order_id": id}))),
        Ok(Ok(Outcome::Replayed(id))) => (StatusCode::OK, Json(json!({"order_id": id}))),
        Ok(Ok(Outcome::Conflict)) => error(StatusCode::CONFLICT, "idempotency key reused with changed request"),
        Ok(Err(_)) | Err(_) => error(StatusCode::INTERNAL_SERVER_ERROR, "storage unavailable"),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if !args.listen.ip().is_loopback() {
        return Err("fixture only permits loopback HTTP listeners".into());
    }
    db::initialize(&args.db_path)?;
    let listener = tokio::net::TcpListener::bind(args.listen).await?;
    let address = listener.local_addr()?;
    let app = Router::new()
        .route("/ready", get(ready))
        .route("/orders", post(order))
        .layer(DefaultBodyLimit::max(65_536))
        .with_state(AppState { db_path: args.db_path, mode: args.mode });
    println!("READY http://{address}");
    axum::serve(listener, app).await?;
    Ok(())
}
