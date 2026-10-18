//! Atomic ledger. Replay or conflict is decided under BEGIN IMMEDIATE.
use crate::db::{OrderRequest, Outcome};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use sha2::{Digest, Sha256};

pub fn fingerprint(body: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(b"POST\0/orders\0");
    digest.update(body);
    format!("{:x}", digest.finalize())
}
pub fn insert(
    conn: &mut Connection,
    key: &str,
    request: &OrderRequest,
    raw_body: &[u8],
) -> rusqlite::Result<Outcome> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let digest = fingerprint(raw_body);
    let previous: Option<(String, i64)> = tx.query_row(
        "SELECT request_fingerprint, order_id FROM idempotency_requests WHERE idempotency_key=?1",
        params![key],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).optional()?;
    if let Some((old_digest, order_id)) = previous {
        if old_digest != digest {
            return Ok(Outcome::Conflict);
        }
        tx.commit()?;
        return Ok(Outcome::Replayed(order_id));
    }
    tx.execute(
        "INSERT INTO orders (idempotency_key, sku, quantity) VALUES (?1, ?2, ?3)",
        params![key, request.sku, request.quantity],
    )?;
    let order_id = tx.last_insert_rowid();
    tx.execute(
        "INSERT INTO idempotency_requests (idempotency_key, request_fingerprint, order_id) VALUES (?1, ?2, ?3)",
        params![key, digest, order_id],
    )?;
    tx.commit()?;
    Ok(Outcome::Created(order_id))
}
