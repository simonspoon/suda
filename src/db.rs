use rusqlite::{Connection, Result};
use std::path::PathBuf;

pub fn data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("SUDA_HOME") {
        return PathBuf::from(dir);
    }
    let home = dirs::home_dir().expect("Could not determine home directory");
    home.join(".suda")
}

pub fn db_path() -> PathBuf {
    data_dir().join("suda.db")
}

pub fn connect() -> Result<Connection> {
    let dir = data_dir();
    std::fs::create_dir_all(&dir).expect("Could not create data directory ~/.suda/");
    let conn = Connection::open(db_path())?;
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;
    initialize(&conn)?;
    Ok(conn)
}

fn initialize(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS memories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            description TEXT NOT NULL,
            type TEXT NOT NULL CHECK(type IN ('user', 'feedback', 'project', 'reference')),
            content TEXT NOT NULL,
            project TEXT,
            strength INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS projects (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            path TEXT NOT NULL,
            description TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS state (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS state_keys (
            namespace TEXT NOT NULL,
            key TEXT NOT NULL,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            verified_at TEXT,
            PRIMARY KEY (namespace, key)
        );
        ",
    )?;

    // Migration: add strength column to existing databases
    let has_strength: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM pragma_table_info('memories') WHERE name = 'strength'",
        [],
        |row| row.get(0),
    )?;
    if !has_strength {
        conn.execute_batch("ALTER TABLE memories ADD COLUMN strength INTEGER NOT NULL DEFAULT 1")?;
    }

    // Create FTS5 virtual table if it doesn't exist
    let fts_exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='memories_fts'",
        [],
        |row| row.get(0),
    )?;

    if !fts_exists {
        conn.execute_batch(
            "
            CREATE VIRTUAL TABLE memories_fts USING fts5(
                name, description, content, type,
                content=memories, content_rowid=id
            );

            -- Triggers to keep FTS in sync
            CREATE TRIGGER memories_ai AFTER INSERT ON memories BEGIN
                INSERT INTO memories_fts(rowid, name, description, content, type)
                VALUES (new.id, new.name, new.description, new.content, new.type);
            END;

            CREATE TRIGGER memories_ad AFTER DELETE ON memories BEGIN
                INSERT INTO memories_fts(memories_fts, rowid, name, description, content, type)
                VALUES ('delete', old.id, old.name, old.description, old.content, old.type);
            END;

            CREATE TRIGGER memories_au AFTER UPDATE ON memories BEGIN
                INSERT INTO memories_fts(memories_fts, rowid, name, description, content, type)
                VALUES ('delete', old.id, old.name, old.description, old.content, old.type);
                INSERT INTO memories_fts(rowid, name, description, content, type)
                VALUES (new.id, new.name, new.description, new.content, new.type);
            END;
            ",
        )?;
    }

    Ok(())
}
