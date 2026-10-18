//! Independent read-only SQLite scalar observer. Never calls a target endpoint.
//! The engine runs this after proxy/client quiescence.
use rusqlite::{params_from_iter, types::Value, Connection, OpenFlags};
use serde_json::Value as JsonValue;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum OracleError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("oracle query must return exactly one integer cell: {0}")]
    InvalidShape(&'static str),
    #[error("unsupported SQL parameter: {0}")]
    InvalidParameter(&'static str),
}

fn as_sql_value(v: &JsonValue) -> Result<Value, OracleError> {
    Ok(match v {
        JsonValue::Null => Value::Null,
        JsonValue::String(s) => Value::Text(s.clone()),
        JsonValue::Bool(b) => Value::Integer(i64::from(*b)),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Integer(i)
            } else if let Some(u) = n.as_u64() {
                Value::Integer(i64::try_from(u).map_err(|_| {
                    OracleError::InvalidParameter("integer exceeds SQLite signed range")
                })?)
            } else {
                let f = n
                    .as_f64()
                    .ok_or(OracleError::InvalidParameter("invalid number"))?;
                if !f.is_finite() {
                    return Err(OracleError::InvalidParameter("non-finite number"));
                }
                Value::Real(f)
            }
        }
        _ => {
            return Err(OracleError::InvalidParameter(
                "only scalar parameters supported",
            ))
        }
    })
}

/// Opens an independent READ_ONLY connection; requires a read-only statement,
/// exactly one column, one row, and a SQLite INTEGER result.
pub fn observe_sqlite_scalar(
    path: &Path,
    sql: &str,
    parameters: &[JsonValue],
) -> Result<i64, OracleError> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    let mut stmt = connection.prepare(sql)?;
    if !stmt.readonly() {
        return Err(OracleError::InvalidShape("statement is not read-only"));
    }
    if stmt.column_count() != 1 {
        return Err(OracleError::InvalidShape("expected one column"));
    }
    if stmt.parameter_count() != parameters.len() {
        return Err(OracleError::InvalidShape("incorrect bound-parameter count"));
    }
    let bound: Vec<Value> = parameters
        .iter()
        .map(as_sql_value)
        .collect::<Result<_, _>>()?;
    let mut rows = stmt.query(params_from_iter(bound.iter()))?;
    let row = rows.next()?.ok_or(OracleError::InvalidShape("zero rows"))?;
    let scalar = row
        .get::<_, i64>(0)
        .map_err(|_| OracleError::InvalidShape("value is not a SQLite integer"))?;
    if rows.next()?.is_some() {
        return Err(OracleError::InvalidShape("multiple rows"));
    }
    Ok(scalar)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;
    fn fixture() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("state.sqlite");
        let conn = Connection::open(&db).unwrap();
        conn.execute_batch("CREATE TABLE orders(id INTEGER PRIMARY KEY, key TEXT);")
            .unwrap();
        for _ in 0..2 {
            conn.execute("INSERT INTO orders(key) VALUES(?1)", params!["abc"])
                .unwrap();
        }
        (dir, db)
    }
    #[test]
    fn independent_parameterized_count_reads_committed_state() {
        let (_dir, path) = fixture();
        let count = observe_sqlite_scalar(
            &path,
            "SELECT COUNT(*) FROM orders WHERE key=?1",
            &[JsonValue::String("abc".into())],
        )
        .unwrap();
        assert_eq!(count, 2);
        let absent = observe_sqlite_scalar(
            &path,
            "SELECT COUNT(*) FROM orders WHERE key=?1",
            &[JsonValue::String("missing".into())],
        )
        .unwrap();
        assert_eq!(absent, 0);
    }
    #[test]
    fn absent_or_malformed_observation_cannot_return_integer() {
        let (_dir, path) = fixture();
        let missing = observe_sqlite_scalar(
            &path.with_file_name("absent.sqlite"),
            "SELECT COUNT(*) FROM orders",
            &[],
        );
        assert!(missing.is_err());
        assert!(observe_sqlite_scalar(&path, "SELECT id,key FROM orders", &[]).is_err());
        assert!(observe_sqlite_scalar(&path, "SELECT id FROM orders", &[]).is_err());
        assert!(observe_sqlite_scalar(&path, "SELECT NULL", &[]).is_err());
        assert!(observe_sqlite_scalar(&path, "SELECT 'one'", &[]).is_err());
        assert!(
            observe_sqlite_scalar(&path, "SELECT COUNT(*) FROM orders WHERE key=?1", &[]).is_err()
        );
        assert!(observe_sqlite_scalar(&path, "UPDATE orders SET key='mutated'", &[]).is_err());
        assert_eq!(
            observe_sqlite_scalar(&path, "SELECT COUNT(*) FROM orders", &[]).unwrap(),
            2
        );
    }
}
