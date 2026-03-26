use rusqlite::{params, Connection, Result};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct StateEntry {
    pub key: String,
    pub value: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StateKeyEntry {
    pub namespace: String,
    pub key: String,
    pub value: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stale: Option<bool>,
}

// --- Legacy flat state (backward compatible) ---

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

// --- Per-key structured state ---

pub fn get_key(conn: &Connection, namespace: &str, key: &str) -> Result<Option<StateKeyEntry>> {
    let mut stmt = conn.prepare(
        "SELECT namespace, key, value, updated_at, verified_at FROM state_keys WHERE namespace = ?1 AND key = ?2",
    )?;
    let mut rows = stmt.query_map(params![namespace, key], row_to_key_entry)?;
    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

pub fn get_all_keys(conn: &Connection, namespace: &str) -> Result<Vec<StateKeyEntry>> {
    let mut stmt = conn.prepare(
        "SELECT namespace, key, value, updated_at, verified_at FROM state_keys WHERE namespace = ?1 ORDER BY key",
    )?;
    let rows = stmt.query_map(params![namespace], row_to_key_entry)?;
    rows.collect()
}

pub fn set_key(conn: &Connection, namespace: &str, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO state_keys (namespace, key, value, updated_at) VALUES (?1, ?2, ?3, datetime('now'))
         ON CONFLICT(namespace, key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        params![namespace, key, value],
    )?;
    Ok(())
}

pub fn verify_key(conn: &Connection, namespace: &str, key: &str) -> Result<bool> {
    let changed = conn.execute(
        "UPDATE state_keys SET verified_at = datetime('now') WHERE namespace = ?1 AND key = ?2",
        params![namespace, key],
    )?;
    Ok(changed > 0)
}

pub fn delete_key(conn: &Connection, namespace: &str, key: &str) -> Result<bool> {
    let changed = conn.execute(
        "DELETE FROM state_keys WHERE namespace = ?1 AND key = ?2",
        params![namespace, key],
    )?;
    Ok(changed > 0)
}

fn row_to_key_entry(row: &rusqlite::Row<'_>) -> Result<StateKeyEntry> {
    Ok(StateKeyEntry {
        namespace: row.get(0)?,
        key: row.get(1)?,
        value: row.get(2)?,
        updated_at: row.get(3)?,
        verified_at: row.get(4)?,
        stale: None,
    })
}

/// Parse a duration string like "24h", "30m", "7d" into seconds.
pub fn parse_duration(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let (num_str, suffix) = s.split_at(s.len().saturating_sub(1));
    let num: i64 = num_str.parse().ok()?;
    match suffix {
        "s" => Some(num),
        "m" => Some(num * 60),
        "h" => Some(num * 3600),
        "d" => Some(num * 86400),
        _ => None,
    }
}

/// Check staleness of entries against a threshold in seconds.
/// Uses the more recent of updated_at and verified_at.
pub fn apply_staleness(entries: &mut [StateKeyEntry], threshold_secs: i64) {
    let now = chrono::Utc::now();
    for entry in entries.iter_mut() {
        let latest = match &entry.verified_at {
            Some(v) => latest_timestamp(&entry.updated_at, v),
            None => entry.updated_at.clone(),
        };
        let is_stale = if let Ok(ts) = chrono::NaiveDateTime::parse_from_str(&latest, "%Y-%m-%d %H:%M:%S") {
            let age = now.signed_duration_since(ts.and_utc());
            age.num_seconds() >= threshold_secs
        } else {
            true // can't parse = treat as stale
        };
        entry.stale = Some(is_stale);
    }
}

fn latest_timestamp(a: &str, b: &str) -> String {
    if b > a { b.to_string() } else { a.to_string() }
}
