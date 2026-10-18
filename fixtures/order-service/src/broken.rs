//! Intentionally incorrect: retries of a committed request insert another order.
use crate::db::{OrderRequest, Outcome};
use rusqlite::{params, Connection, TransactionBehavior};

pub fn insert(
    conn: &mut Connection,
    key: &str,
    request: &OrderRequest,
) -> rusqlite::Result<Outcome> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    tx.execute(
        "INSERT INTO orders (idempotency_key, sku, quantity) VALUES (?1, ?2, ?3)",
        params![key, request.sku, request.quantity],
    )?;
    let id = tx.last_insert_rowid();
    tx.commit()?;
    Ok(Outcome::Created(id))
}
