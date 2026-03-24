use rusqlite::{params, Connection, Result};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct StateEntry {
    pub key: String,
    pub value: String,
    pub updated_at: String,
}

pub fn get(conn: &Connection, key: &str) -> Result<Option<StateEntry>> {
    let mut stmt = conn.prepare("SELECT key, value, updated_at FROM state WHERE key = ?1")?;
    let mut rows = stmt.query_map(params![key], row_to_entry)?;
    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

pub fn set(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO state (key, value, updated_at) VALUES (?1, ?2, datetime('now'))
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        params![key, value],
    )?;
    Ok(())
}

pub fn list(conn: &Connection) -> Result<Vec<StateEntry>> {
    let mut stmt = conn.prepare("SELECT key, value, updated_at FROM state ORDER BY key")?;
    let rows = stmt.query_map([], row_to_entry)?;
    rows.collect()
}

pub fn delete(conn: &Connection, key: &str) -> Result<bool> {
    let changed = conn.execute("DELETE FROM state WHERE key = ?1", params![key])?;
    Ok(changed > 0)
}

fn row_to_entry(row: &rusqlite::Row<'_>) -> Result<StateEntry> {
    Ok(StateEntry {
        key: row.get(0)?,
        value: row.get(1)?,
        updated_at: row.get(2)?,
    })
}
