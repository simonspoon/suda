use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

fn default_strength() -> i64 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: i64,
    pub name: String,
    pub description: String,
    #[serde(rename = "type")]
    pub memory_type: String,
    pub content: String,
    pub project: Option<String>,
    #[serde(default = "default_strength")]
    pub strength: i64,
    pub created_at: String,
    pub updated_at: String,
}

pub fn store(
    conn: &Connection,
    name: &str,
    description: &str,
    memory_type: &str,
    content: &str,
    project: Option<&str>,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO memories (name, description, type, content, project) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![name, description, memory_type, content, project],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn recall(
    conn: &Connection,
    query: Option<&str>,
    memory_type: Option<&str>,
    project: Option<&str>,
    limit: i64,
) -> Result<Vec<Memory>> {
    match query {
        Some(q) => recall_fts(conn, q, memory_type, project, limit),
        None => recall_recent(conn, memory_type, project, limit),
    }
}

fn recall_fts(
    conn: &Connection,
    query: &str,
    memory_type: Option<&str>,
    project: Option<&str>,
    limit: i64,
) -> Result<Vec<Memory>> {
    let mut sql = String::from(
        "SELECT m.id, m.name, m.description, m.type, m.content, m.project, m.strength, m.created_at, m.updated_at
         FROM memories m
         JOIN memories_fts f ON m.id = f.rowid
         WHERE memories_fts MATCH ?1",
    );
    let mut idx = 2;
    if memory_type.is_some() {
        sql.push_str(&format!(" AND m.type = ?{idx}"));
        idx += 1;
    }
    if project.is_some() {
        sql.push_str(&format!(" AND m.project = ?{idx}"));
        idx += 1;
    }
    sql.push_str(&format!(" ORDER BY rank LIMIT ?{idx}"));

    let mut stmt = conn.prepare(&sql)?;

    let rows = match (memory_type, project) {
        (Some(t), Some(p)) => stmt.query_map(params![query, t, p, limit], row_to_memory)?,
        (Some(t), None) => stmt.query_map(params![query, t, limit], row_to_memory)?,
        (None, Some(p)) => stmt.query_map(params![query, p, limit], row_to_memory)?,
        (None, None) => stmt.query_map(params![query, limit], row_to_memory)?,
    };

    rows.collect()
}

fn recall_recent(
    conn: &Connection,
    memory_type: Option<&str>,
    project: Option<&str>,
    limit: i64,
) -> Result<Vec<Memory>> {
    let mut sql = String::from(
        "SELECT id, name, description, type, content, project, strength, created_at, updated_at FROM memories WHERE 1=1",
    );
    let mut idx = 1;
    if memory_type.is_some() {
        sql.push_str(&format!(" AND type = ?{idx}"));
        idx += 1;
    }
    if project.is_some() {
        sql.push_str(&format!(" AND project = ?{idx}"));
        idx += 1;
    }
    sql.push_str(&format!(" ORDER BY updated_at DESC LIMIT ?{idx}"));

    let mut stmt = conn.prepare(&sql)?;

    let rows = match (memory_type, project) {
        (Some(t), Some(p)) => stmt.query_map(params![t, p, limit], row_to_memory)?,
        (Some(t), None) => stmt.query_map(params![t, limit], row_to_memory)?,
        (None, Some(p)) => stmt.query_map(params![p, limit], row_to_memory)?,
        (None, None) => stmt.query_map(params![limit], row_to_memory)?,
    };

    rows.collect()
}

fn row_to_memory(row: &rusqlite::Row<'_>) -> Result<Memory> {
    Ok(Memory {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        memory_type: row.get(3)?,
        content: row.get(4)?,
        project: row.get(5)?,
        strength: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

pub fn get(conn: &Connection, id: i64) -> Result<Option<Memory>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, description, type, content, project, strength, created_at, updated_at FROM memories WHERE id = ?1",
    )?;
    let mut rows = stmt.query_map(params![id], row_to_memory)?;
    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

pub fn update(
    conn: &Connection,
    id: i64,
    name: Option<&str>,
    description: Option<&str>,
    content: Option<&str>,
    memory_type: Option<&str>,
    project: Option<&str>,
) -> Result<bool> {
    let mut sets = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(v) = name {
        sets.push("name = ?");
        values.push(Box::new(v.to_string()));
    }
    if let Some(v) = description {
        sets.push("description = ?");
        values.push(Box::new(v.to_string()));
    }
    if let Some(v) = content {
        sets.push("content = ?");
        values.push(Box::new(v.to_string()));
    }
    if let Some(v) = memory_type {
        sets.push("type = ?");
        values.push(Box::new(v.to_string()));
    }
    if let Some(v) = project {
        sets.push("project = ?");
        values.push(Box::new(v.to_string()));
    }

    if sets.is_empty() {
        return Ok(false);
    }

    sets.push("updated_at = datetime('now')");

    let sql = format!("UPDATE memories SET {} WHERE id = ?", sets.join(", "));
    values.push(Box::new(id));

    let params: Vec<&dyn rusqlite::types::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    let changed = conn.execute(&sql, params.as_slice())?;
    Ok(changed > 0)
}

pub fn reinforce(conn: &Connection, id: i64) -> Result<bool> {
    let changed = conn.execute(
        "UPDATE memories SET strength = strength + 1, updated_at = datetime('now') WHERE id = ?1",
        params![id],
    )?;
    Ok(changed > 0)
}

pub fn reinforce_set(conn: &Connection, id: i64, value: i64) -> Result<bool> {
    let changed = conn.execute(
        "UPDATE memories SET strength = ?2, updated_at = datetime('now') WHERE id = ?1",
        params![id, value],
    )?;
    Ok(changed > 0)
}

pub fn forget(conn: &Connection, id: i64) -> Result<bool> {
    let changed = conn.execute("DELETE FROM memories WHERE id = ?1", params![id])?;
    Ok(changed > 0)
}

pub fn export(
    conn: &Connection,
    memory_type: Option<&str>,
    project: Option<&str>,
) -> Result<Vec<Memory>> {
    recall_recent(conn, memory_type, project, i64::MAX)
}

pub fn import(conn: &Connection, memories: &[Memory]) -> Result<usize> {
    let mut count = 0;
    for m in memories {
        conn.execute(
            "INSERT INTO memories (name, description, type, content, project, strength, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                m.name,
                m.description,
                m.memory_type,
                m.content,
                m.project,
                m.strength,
                m.created_at,
                m.updated_at,
            ],
        )?;
        count += 1;
    }
    Ok(count)
}
