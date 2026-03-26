# Contributing

## Prerequisites

You need a working Rust toolchain. Install it via [rustup](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

No other system dependencies are required. SQLite is compiled from source via the `rusqlite` `bundled` feature.

## Building

Debug build:

```bash
cargo build
```

Release build:

```bash
cargo build --release
```

The binary is written to `target/debug/suda` or `target/release/suda`.

## Testing

Run the full test suite:

```bash
cargo test
```

Run a single test by name:

```bash
cargo test store_and_recall
```

### How TestEnv works

Integration tests use a `TestEnv` helper defined in `tests/integration.rs`. Each test creates its own isolated environment:

```rust
let env = TestEnv::new("my-test");
let out = env.stdout(&["store", "--type", "user", "--name", "n", "--description", "d", "content"]);
```

`TestEnv::new(name)` creates a temporary directory and `TestEnv::suda()` runs the suda binary with `SUDA_HOME` set to that temp directory. This means each test gets its own SQLite database, completely isolated from your real data and from other tests. The temp directory is cleaned up automatically when the `TestEnv` is dropped.

Key methods:

- `suda(&[&str]) -> Output` -- run suda with args, return raw output
- `stdout(&[&str]) -> String` -- run suda, assert success, return stdout
- `fails(&[&str]) -> String` -- run suda, assert failure, return stderr+stdout

## Project layout

```
src/
  main.rs    -- CLI definition (Cli, Commands, MemoryType enums), argument parsing, run() dispatch
  db.rs      -- Database connection, path resolution (SUDA_HOME env var), schema initialization with FTS5
  memory.rs  -- Memory CRUD operations: store, recall (FTS and recent), update, forget, export, import
  display.rs -- All output formatting: tables, detail views, JSON serialization, markdown export
  project.rs -- Project registry CRUD: add, remove, show, list
  state.rs   -- State store: legacy flat (get, set, list, delete) and per-key namespaced (get_key, set_key, get_all_keys, delete_key, verify_key, staleness)
tests/
  integration.rs -- End-to-end tests using TestEnv for process-level isolation
```

## Adding a new command

Adding a command touches four places. Here is the sequence:

1. **Add a variant to `Commands`** in `main.rs`. Use clap derive attributes for arguments:

```rust
enum Commands {
    // ... existing variants ...

    /// Description shown in --help
    MyCommand {
        #[arg(long)]
        some_flag: bool,
    },
}
```

2. **Add a match arm in `run()`** in `main.rs`. This is where you call into module functions and display results:

```rust
Commands::MyCommand { some_flag } => {
    let result = my_module::do_thing(conn, some_flag)?;
    display::my_command_result(&result);
}
```

3. **Add the module function** (e.g., in a new `src/my_module.rs` or an existing module). If it is a new module, add `mod my_module;` at the top of `main.rs`.

4. **Add a display function** in `display.rs` for the output. Follow the existing pattern: plain text by default, JSON when `--json` is passed.

## Adding a new memory type

Memory types are constrained at two levels. Both must be updated:

1. **Update the `MemoryType` enum and `as_str()`** in `main.rs`:

```rust
#[derive(Clone, ValueEnum)]
enum MemoryType {
    User,
    Feedback,
    Project,
    Reference,
    MyType,       // add variant
}

impl MemoryType {
    fn as_str(&self) -> &'static str {
        match self {
            // ... existing arms ...
            MemoryType::MyType => "mytype",   // add arm
        }
    }
}
```

2. **Update the CHECK constraint** in `db.rs` inside the `initialize` function. The `memories` table enforces allowed types at the database level:

```sql
type TEXT NOT NULL CHECK(type IN ('user', 'feedback', 'project', 'reference', 'mytype'))
```

Note: since the table is created with `CREATE TABLE IF NOT EXISTS`, existing databases will keep the old constraint. For development, delete your local `suda.db` to pick up schema changes, or run a migration.

## Code patterns

### Dynamic SQL building

Functions in `memory.rs` build SQL dynamically based on which optional filters are present. The pattern uses a mutable `String` for the query and a counter `idx` for positional parameter placeholders:

```rust
let mut sql = String::from("SELECT ... FROM memories WHERE 1=1");
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
```

Because rusqlite requires all parameters at once, the actual `query_map` call uses a match on the combination of `Option` values to supply the correct params tuple:

```rust
let rows = match (memory_type, project) {
    (Some(t), Some(p)) => stmt.query_map(params![t, p, limit], row_to_memory)?,
    (Some(t), None)    => stmt.query_map(params![t, limit], row_to_memory)?,
    (None, Some(p))    => stmt.query_map(params![p, limit], row_to_memory)?,
    (None, None)       => stmt.query_map(params![limit], row_to_memory)?,
};
```

The `update` function uses a different approach -- it collects `Box<dyn ToSql>` values into a `Vec` and builds the SET clause dynamically, so it does not need the combinatorial match.

### Row-to-struct conversion

Each module defines a private `row_to_*` function that maps a `rusqlite::Row` to the module's struct by positional index:

```rust
fn row_to_memory(row: &rusqlite::Row<'_>) -> Result<Memory> {
    Ok(Memory {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        memory_type: row.get(3)?,
        content: row.get(4)?,
        project: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}
```

This function is passed as a callback to `stmt.query_map()`. The same pattern appears in `project.rs` (`row_to_project`) and `state.rs` (`row_to_entry` for legacy state, `row_to_key_entry` for structured state). Column order must match the SELECT clause.
