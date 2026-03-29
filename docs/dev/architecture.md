# Architecture

Suda is a Rust CLI for structured memory and knowledge management, built for AI agent workflows. It stores memories, project registrations, and key-value state in a single SQLite database with FTS5 full-text search.

## Module Overview

| Module | File | Purpose |
|--------|------|---------|
| `db` | `src/db.rs` | Database path resolution, connection setup, schema initialization, FTS5 virtual table and trigger creation |
| `main` | `src/main.rs` | CLI argument parsing (clap), command enum definitions, top-level command routing via `run()` |
| `memory` | `src/memory.rs` | `Memory` struct, CRUD operations (store/recall/update/forget/reinforce), FTS5 search, export/import |
| `project` | `src/project.rs` | `Project` struct, project registry operations (add/remove/show/list) |
| `state` | `src/state.rs` | `StateEntry` and `StateKeyEntry` structs, legacy flat state (get/set/list/delete), per-key namespaced state (get_key/set_key/get_all_keys/delete_key/verify_key), staleness detection |
| `display` | `src/display.rs` | All output formatting: tables, detail views, JSON serialization, markdown export |

### Module dependencies

```
main.rs
  +-- db        (connect)
  +-- memory    (store, recall, update, reinforce, forget, export, import)
  +-- project   (add, remove, show, list)
  +-- state     (get, set, list, delete, get_key, set_key, get_all_keys, delete_key, verify_key)
  +-- display   (all output functions)
```

`main.rs` is the only module that calls `display`. The `memory`, `project`, and `state` modules return data; they never print directly. `db` is called once at startup to obtain a `Connection`, which is passed to all command handlers.

## Data Flow

```
CLI invocation
  |
  v
Cli::parse()              -- clap parses args into Commands enum (src/main.rs:216)
  |
  v
db::connect()             -- open/create SQLite DB, set PRAGMAs, run initialize() (src/db.rs:16)
  |
  v
run(command, &conn)       -- match on Commands, call into memory/project/state (src/main.rs:233)
  |
  v
module function           -- e.g. memory::recall() builds SQL, queries DB (src/memory.rs:32)
  |
  v
display::*                -- format and print results to stdout (src/display.rs)
```

Errors propagate as `Result<(), Box<dyn std::error::Error>>` from `run()`. On failure, `main()` prints the error to stderr and exits with code 1 (`src/main.rs:228-230`).

## SQLite Schema

All tables are created in `db::initialize()` (`src/db.rs:26`).

### memories

```sql
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
```

The `type` column is constrained to four values: `user`, `feedback`, `project`, `reference`. The `project` column is nullable (memories can exist without a project association). The `strength` column tracks reinforcement frequency (default 1); it is incremented by `reinforce` and can be set to an explicit value via `reinforce --set`.

#### Migration

Existing databases that lack the `strength` column are migrated automatically. On connect, `initialize()` checks `pragma_table_info('memories')` for the column and runs `ALTER TABLE memories ADD COLUMN strength INTEGER NOT NULL DEFAULT 1` if it is missing (`src/db.rs:68-75`). All pre-existing memories receive strength 1.

### projects

```sql
CREATE TABLE IF NOT EXISTS projects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    path TEXT NOT NULL,
    description TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

The `name` column has a `UNIQUE` constraint. The `description` column is nullable.

### state

```sql
CREATE TABLE IF NOT EXISTS state (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

Uses `key` as a natural primary key (no autoincrement). The `set` operation uses `INSERT ... ON CONFLICT(key) DO UPDATE` for upsert behavior.

### state_keys

```sql
CREATE TABLE IF NOT EXISTS state_keys (
    namespace TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    verified_at TEXT,
    PRIMARY KEY (namespace, key)
);
```

Structured per-key state with namespace scoping. The composite primary key `(namespace, key)` allows multiple keys within a namespace. The `verified_at` column is nullable and set explicitly via `state verify` to track when a key was last confirmed as still-current. The `set_key` operation uses `INSERT ... ON CONFLICT(namespace, key) DO UPDATE` for upsert behavior.

## FTS5 Integration

The FTS5 virtual table is created conditionally -- `initialize()` checks `sqlite_master` for its existence before creating it (`src/db.rs:58-62`). This is a content-sync table (external content) backed by the `memories` table.

### Virtual table

```sql
CREATE VIRTUAL TABLE memories_fts USING fts5(
    name, description, content, type,
    content=memories, content_rowid=id
);
```

The indexed columns are `name`, `description`, `content`, and `type`. The `content=memories` directive makes this an external-content FTS table -- it does not store its own copy of the text, instead reading from the `memories` table when needed.

### Sync triggers

Three triggers keep the FTS index synchronized with the `memories` table (`src/db.rs:73-88`):

**After insert** (`memories_ai`):
```sql
CREATE TRIGGER memories_ai AFTER INSERT ON memories BEGIN
    INSERT INTO memories_fts(rowid, name, description, content, type)
    VALUES (new.id, new.name, new.description, new.content, new.type);
END;
```

**After delete** (`memories_ad`):
```sql
CREATE TRIGGER memories_ad AFTER DELETE ON memories BEGIN
    INSERT INTO memories_fts(memories_fts, rowid, name, description, content, type)
    VALUES ('delete', old.id, old.name, old.description, old.content, old.type);
END;
```

**After update** (`memories_au`) -- performs a delete-then-insert to update the index:
```sql
CREATE TRIGGER memories_au AFTER UPDATE ON memories BEGIN
    INSERT INTO memories_fts(memories_fts, rowid, name, description, content, type)
    VALUES ('delete', old.id, old.name, old.description, old.content, old.type);
    INSERT INTO memories_fts(rowid, name, description, content, type)
    VALUES (new.id, new.name, new.description, new.content, new.type);
END;
```

### Search behavior

When `recall` is called with a query string, it uses FTS5 `MATCH` with `ORDER BY rank` for relevance-ranked results (`src/memory.rs:45-79`). Without a query, it falls back to `recall_recent`, which orders by `updated_at DESC` (`src/memory.rs:81-111`).

## Key Types

### Memory (`src/memory.rs:8-21`)

Derives: `Debug`, `Clone`, `Serialize`, `Deserialize`

| Field | Rust Type | Serde | DB Column |
|-------|-----------|-------|-----------|
| `id` | `i64` | `id` | `id` (INTEGER PK AUTOINCREMENT) |
| `name` | `String` | `name` | `name` (TEXT NOT NULL) |
| `description` | `String` | `description` | `description` (TEXT NOT NULL) |
| `memory_type` | `String` | `type` (renamed) | `type` (TEXT NOT NULL, CHECK constraint) |
| `content` | `String` | `content` | `content` (TEXT NOT NULL) |
| `project` | `Option<String>` | `project` | `project` (TEXT, nullable) |
| `strength` | `i64` | `strength` (default 1) | `strength` (INTEGER NOT NULL, default 1) |
| `created_at` | `String` | `created_at` | `created_at` (TEXT NOT NULL, default now) |
| `updated_at` | `String` | `updated_at` | `updated_at` (TEXT NOT NULL, default now) |

The `memory_type` field is renamed to `type` in JSON via `#[serde(rename = "type")]`. The `strength` field uses `#[serde(default = "default_strength")]` so that importing JSON without a `strength` key defaults to 1 (backward compatibility with older exports).

### Project (`src/project.rs:4-12`)

Derives: `Debug`, `Clone`, `Serialize`

| Field | Rust Type | DB Column |
|-------|-----------|-----------|
| `id` | `i64` | `id` (INTEGER PK AUTOINCREMENT) |
| `name` | `String` | `name` (TEXT NOT NULL UNIQUE) |
| `path` | `String` | `path` (TEXT NOT NULL) |
| `description` | `Option<String>` | `description` (TEXT, nullable) |
| `created_at` | `String` | `created_at` (TEXT NOT NULL, default now) |
| `updated_at` | `String` | `updated_at` (TEXT NOT NULL, default now) |

`Project` implements `Serialize` but not `Deserialize` -- projects are not imported/exported.

### StateEntry (`src/state.rs:4-9`)

Derives: `Debug`, `Clone`, `Serialize`

| Field | Rust Type | DB Column |
|-------|-----------|-----------|
| `key` | `String` | `key` (TEXT PRIMARY KEY) |
| `value` | `String` | `value` (TEXT NOT NULL) |
| `updated_at` | `String` | `updated_at` (TEXT NOT NULL, default now) |

`StateEntry` implements `Serialize` but not `Deserialize`. The `set` function uses raw parameters rather than deserializing into the struct.

### StateKeyEntry (`src/state.rs:11-21`)

Derives: `Debug`, `Clone`, `Serialize`

| Field | Rust Type | Serde | DB Column |
|-------|-----------|-------|-----------|
| `namespace` | `String` | `namespace` | `namespace` (TEXT NOT NULL) |
| `key` | `String` | `key` | `key` (TEXT NOT NULL) |
| `value` | `String` | `value` | `value` (TEXT NOT NULL) |
| `updated_at` | `String` | `updated_at` | `updated_at` (TEXT NOT NULL, default now) |
| `verified_at` | `Option<String>` | `verified_at` (skip if None) | `verified_at` (TEXT, nullable) |
| `stale` | `Option<bool>` | `stale` (skip if None) | *computed, not stored* |

The `verified_at` and `stale` fields use `#[serde(skip_serializing_if = "Option::is_none")]` to omit them from JSON when not set. The `stale` field is computed at query time by `apply_staleness()` and is never persisted to the database.

### MemoryType enum (`src/main.rs:182-198`)

Defined in `main.rs` for CLI parsing only (not used in data modules). Derives `Clone`, `ValueEnum`.

| Variant | String value |
|---------|-------------|
| `User` | `"user"` |
| `Feedback` | `"feedback"` |
| `Project` | `"project"` |
| `Reference` | `"reference"` |

### ExportFormat enum (`src/main.rs:201-205`)

Derives `Clone`, `ValueEnum`. Variants: `Json`, `Md`.

## Storage

### Database location

Resolved by `db::data_dir()` (`src/db.rs:4-10`):

1. If `SUDA_HOME` environment variable is set, use that path.
2. Otherwise, use `~/.suda/`.

The database file is `suda.db` within the data directory (`src/db.rs:12-14`). The directory is created automatically with `create_dir_all` on first connection (`src/db.rs:18`).

### Connection configuration

Two PRAGMAs are set on every connection (`src/db.rs:20-21`):

- **`journal_mode=WAL`** -- Write-Ahead Logging for concurrent read access and improved write performance.
- **`foreign_keys=ON`** -- Enables foreign key constraint enforcement (off by default in SQLite).

Note: while foreign keys are enabled, the current schema does not define any `FOREIGN KEY` constraints. The `memories.project` column is a plain nullable `TEXT` field, not a foreign key reference to `projects.name`.
