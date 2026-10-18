//! SQLite storage contract shared by broken/fixed mode.
//! HTTP success is constructed only after transaction commit.
use rusqlite::{Connection, OpenFlags};
use serde::Deserialize;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderRequest {
    pub sku: String,
    pub quantity: u32,
}
impl OrderRequest {
    pub fn valid(&self) -> bool {
        !self.sku.trim().is_empty()
            && self.sku.len() <= 128
            && (1..=1_000_000).contains(&self.quantity)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Created(i64),
    Replayed(i64),
    Conflict,
}
pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
    conn.busy_timeout(Duration::from_secs(10))?;
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;
    Ok(conn)
}
pub fn initialize(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let conn = Connection::open(path)?;
    conn.busy_timeout(Duration::from_secs(10))?;
    conn.execute_batch(include_str!("../migrations/001_init.sql"))?;
    Ok(())
}
