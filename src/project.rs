use rusqlite::{params, Connection, Result};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub fn add(conn: &Connection, name: &str, path: &str, description: Option<&str>) -> Result<i64> {
    conn.execute(
        "INSERT INTO projects (name, path, description) VALUES (?1, ?2, ?3)",
        params![name, path, description],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn remove(conn: &Connection, name: &str) -> Result<bool> {
    let changed = conn.execute("DELETE FROM projects WHERE name = ?1", params![name])?;
    Ok(changed > 0)
}

pub fn show(conn: &Connection, name: &str) -> Result<Option<Project>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, path, description, created_at, updated_at FROM projects WHERE name = ?1",
    )?;
    let mut rows = stmt.query_map(params![name], row_to_project)?;
    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

pub fn list(conn: &Connection) -> Result<Vec<Project>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, path, description, created_at, updated_at FROM projects ORDER BY name",
    )?;
    let rows = stmt.query_map([], row_to_project)?;
    rows.collect()
}

fn row_to_project(row: &rusqlite::Row<'_>) -> Result<Project> {
    Ok(Project {
        id: row.get(0)?,
        name: row.get(1)?,
        path: row.get(2)?,
        description: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}
