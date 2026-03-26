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

Manage key-value session state.

### state get

```
suda state get <KEY> [OPTIONS]
```

Retrieve a state value by key.

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `KEY` | string (positional) | *required* | State key. |
| `--json` | flag | `false` | Output as JSON. |

```sh
suda state get session-state --json
```

### state set

```
suda state set <KEY> [VALUE] [OPTIONS]
```

Set a state value. The value can be provided as a positional argument or piped via `--stdin`.

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `KEY` | string (positional) | *required* | State key. |
| `VALUE` | string (positional) | — | State value. Required unless `--stdin` is used. |
| `--stdin` | flag | `false` | Read value from stdin instead of the positional argument. |

```sh
suda state set session-state "Completed refactor of auth module"
```

### state list

```
suda state list [OPTIONS]
```

List all state entries.

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--json` | flag | `false` | Output as JSON. |

```sh
suda state list --json
```

### state delete

```
suda state delete <KEY>
```

Delete a state entry by key.

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `KEY` | string (positional) | *required* | State key to delete. |

```sh
suda state delete session-state
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
