# Commands

Complete CLI reference for `suda`, a structured memory and knowledge management tool.

## store

```
suda store [CONTENT] --type <TYPE> --name <NAME> [OPTIONS]
```

Store a new memory. Content can be provided as a positional argument or piped via `--stdin`.

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `CONTENT` | string (positional) | — | Memory content. Required unless `--stdin` is used. |
| `--type` | enum: `user`, `feedback`, `project`, `reference` | *required* | Memory type. |
| `--name` | string | *required* | Memory name. |
| `--description` | string | `""` | Memory description. |
| `--project` | string | — | Associated project name. |
| `--stdin` | flag | `false` | Read content from stdin instead of the positional argument. |

```sh
suda store "Always use explicit error handling" --type user --name error-style --description "Coding preference"
```

## recall

```
suda recall [QUERY] [OPTIONS]
```

Search and retrieve memories. When a query is provided, it uses FTS5 full-text search. Without a query, returns all memories matching the filters.

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `QUERY` | string (positional) | — | FTS5 full-text search query. |
| `--type` | enum: `user`, `feedback`, `project`, `reference` | — | Filter results by memory type. |
| `--project` | string | — | Filter results by project name. |
| `--limit` | integer | `20` | Maximum number of results to return. |
| `--json` | flag | `false` | Output results as JSON. |

```sh
suda recall "error handling" --type user --json --limit 10
```

## update

```
suda update <ID> [OPTIONS]
```

Update an existing memory by its ID. All fields are optional; only provided fields are changed.

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `ID` | integer (positional) | *required* | Memory ID to update. |
| `--name` | string | — | New name. |
| `--description` | string | — | New description. |
| `--content` | string | — | New content. |
| `--type` | enum: `user`, `feedback`, `project`, `reference` | — | New memory type. |
| `--project` | string | — | New project association. |

```sh
suda update 42 --content "Updated preference" --description "Revised coding style"
```

## forget

```
suda forget <ID>
```

Delete a memory by its ID.

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `ID` | integer (positional) | *required* | Memory ID to delete. |

```sh
suda forget 42
```

## projects

```
suda projects [OPTIONS]
```

List all registered projects.

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--json` | flag | `false` | Output results as JSON. |

```sh
suda projects --json
```

## project

Manage project registrations.

### project add

```
suda project add <NAME> <PATH> [OPTIONS]
```

Register a new project.

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `NAME` | string (positional) | *required* | Project name. |
| `PATH` | string (positional) | *required* | Absolute path to the project directory. |
| `--description` | string | — | Project description. |

```sh
suda project add my-app /Users/me/projects/my-app --description "Main application"
```

### project remove

```
suda project remove <NAME>
```

Unregister a project.

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `NAME` | string (positional) | *required* | Project name to remove. |

```sh
suda project remove my-app
```

### project show

```
suda project show <NAME> [OPTIONS]
```

Show details for a registered project.

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `NAME` | string (positional) | *required* | Project name. |
| `--json` | flag | `false` | Output as JSON. |

```sh
suda project show my-app --json
```

## state

Manage session state. Supports both legacy flat key-value state and structured per-key namespaced state with verification and staleness detection.

### state get

```
suda state get <NAME> [OPTIONS]
```

Retrieve state by namespace. Without `--key`, returns all structured keys in the namespace (falling back to the legacy flat state entry if no structured keys exist). With `--key`, returns a specific key within the namespace.

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `NAME` | string (positional) | *required* | State namespace (e.g. `session-state`). |
| `--key` | string | — | Specific key within the namespace. |
| `--json` | flag | `false` | Output as JSON. |
| `--check-stale` | string | — | Flag entries as stale if older than this duration (e.g. `24h`, `30m`, `7d`). Compares against the more recent of `updated_at` and `verified_at`. |

```sh
# Get all keys in a namespace
suda state get session-state --json

# Get a specific key
suda state get session-state --key current-task --json

# Check for stale entries (older than 24 hours)
suda state get session-state --check-stale 24h --json
```

### state set

```
suda state set <NAME> [VALUE] [OPTIONS]
```

Set a state value. Without `--key`, writes to the legacy flat state store. With `--key`, writes to the structured per-key store within the given namespace. The value can be provided as a positional argument or piped via `--stdin`.

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `NAME` | string (positional) | *required* | State namespace (e.g. `session-state`). |
| `VALUE` | string (positional) | — | State value. Required unless `--stdin` is used. |
| `--key` | string | — | Specific key within the namespace. |
| `--stdin` | flag | `false` | Read value from stdin instead of the positional argument. |

```sh
# Legacy flat state
suda state set session-state "Completed refactor of auth module"

# Structured per-key state
suda state set session-state --key current-task "Implement error recovery"
suda state set session-state --key last-file "src/parser.rs"
```

### state verify

```
suda state verify <NAME> --key <KEY>
```

Update the `verified_at` timestamp on a structured state key without changing its value. Useful for marking state as still-current, which resets the staleness clock used by `--check-stale`.

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `NAME` | string (positional) | *required* | State namespace. |
| `--key` | string | *required* | Key to verify. |

```sh
suda state verify session-state --key current-task
```

### state list

```
suda state list [OPTIONS]
```

List all legacy flat state entries.

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--json` | flag | `false` | Output as JSON. |

```sh
suda state list --json
```

### state delete

```
suda state delete <NAME> [OPTIONS]
```

Delete a state entry. Without `--key`, deletes the legacy flat state entry. With `--key`, deletes a specific key from the structured per-key store.

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `NAME` | string (positional) | *required* | State namespace. |
| `--key` | string | — | Specific key within the namespace to delete. |

```sh
# Delete legacy flat state
suda state delete session-state

# Delete a specific structured key
suda state delete session-state --key current-task
```

## init

```
suda init
```

Initialize the suda database. Creates the database file and schema if they do not already exist.

```sh
suda init
```

## export

```
suda export [OPTIONS]
```

Export memories, optionally filtered by type or project.

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--type` | enum: `user`, `feedback`, `project`, `reference` | — | Filter exported memories by type. |
| `--project` | string | — | Filter exported memories by project. |
| `--format` | enum: `json`, `md` | `json` | Output format. |

```sh
suda export --type user --format md
```

## import

```
suda import <FILE>
```

Import memories from a JSON file.

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `FILE` | string (positional) | *required* | Path to the JSON file to import. |

```sh
suda import memories.json
```
