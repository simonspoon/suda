# Agent Integration

Suda provides persistent, structured memory for AI agents across conversation boundaries. Rather than starting each session from scratch, agents use suda to recall prior decisions, user preferences, corrections, and project context. This document describes the integration patterns that make that possible.

## CLAUDE.md startup sequence

The primary integration point is a CLAUDE.md file that instructs Claude Code to load suda context at the start of every session. Place the following in your global `~/.claude/CLAUDE.md` or in a project-level `CLAUDE.md`:

```bash
# Check availability
which suda

# Load session state
suda state get session-state 2>/dev/null

# Load user preferences and feedback
suda recall --type user --json --limit 20 2>/dev/null
suda recall --type feedback --json --limit 20 2>/dev/null

# Load project registry
suda projects --json 2>/dev/null

# Load project-specific memories (if working directory matches a registered project)
suda recall --project <project-name> --json 2>/dev/null
```

Every command is piped through `2>/dev/null` so that a missing key or empty result does not interrupt startup. The `--json` flag ensures output is machine-parseable rather than formatted for human display.

This sequence gives the agent four layers of context before it begins work:

1. **Session state** -- a free-text summary of where the last conversation left off.
2. **User-level memories** -- preferences and corrections that apply everywhere.
3. **Project registry** -- which projects exist and where they live on disk.
4. **Project-specific memories** -- decisions, deadlines, and context scoped to the current project.

## Memory types and agent use cases

Suda defines four memory types. Each serves a distinct role in shaping agent behavior.

### user

Stores information about the user: role, preferences, expertise level, tool choices, communication style. Agents use these memories to tailor tone, detail level, and tool selection.

```bash
suda store --type user --name "preferred-lang" --description "Primary language" "Rust"
```

### feedback

Stores corrections and confirmations from the user. When an agent makes a mistake and the user corrects it, storing that correction as feedback prevents the same mistake in future sessions.

```bash
suda store --type feedback --name "no-emojis" --description "User preference" "Do not use emojis in files or commit messages"
```

### project

Stores ongoing work context: architectural decisions, deadlines, task status, design constraints. Agents use these to inform suggestions and avoid contradicting prior decisions.

```bash
suda store --type project --name "db-choice" --project myapp --description "Database decision" "Using SQLite via rusqlite, no ORM"
```

### reference

Stores pointers to external systems, documentation, or resources. Helps agents find information without asking the user to repeat URLs or paths.

```bash
suda store --type reference --name "ci-dashboard" --description "CI system" "https://ci.example.com/myapp"
```

## Session state

The state subsystem is a simple key-value store. Its primary agent use case is persisting a session summary so the next conversation can pick up where the previous one left off.

At the end of a session, the agent writes a summary:

```bash
suda state set session-state "Finished refactoring the parser module. All tests pass. Next: add error recovery for malformed input."
```

At the start of the next session, the agent reads it back:

```bash
suda state get session-state
```

The state store uses upsert semantics -- setting a key that already exists overwrites the previous value. This keeps session-state always pointing to the most recent summary.

The `--stdin` flag allows writing longer or multi-line values:

```bash
echo "Line one
Line two
Line three" | suda state set session-state --stdin
```

Other useful state keys beyond `session-state` are up to you. The store accepts any string key.

## Project registry

Registering a project associates a name with an absolute path on disk. This lets the CLAUDE.md startup sequence match the current working directory to a project and load the right memories automatically.

Register a project:

```bash
suda project add myapp /Users/me/code/myapp --description "Main application"
```

List all registered projects:

```bash
suda projects --json
```

Show details for a single project:

```bash
suda project show myapp --json
```

Remove a project:

```bash
suda project remove myapp
```

Once a project is registered, any memory stored with `--project myapp` can be recalled with `suda recall --project myapp`, giving agents a scoped view of context relevant to the current work.

## JSON output

All read commands support a `--json` flag that switches output from human-readable tables to machine-parseable JSON. This is the format agents should use.

Commands that accept `--json`:

```bash
suda recall --json
suda recall --type user --json --limit 20
suda recall --project myapp --json
suda projects --json
suda project show myapp --json
suda state get session-state --json
suda state list --json
```

Without `--json`, output is formatted as tables or detail views intended for terminal use. Agents should always pass `--json` to get structured data they can parse reliably.

## Deduplication

Agents create memories frequently, and without care they will store the same information repeatedly. Before storing a new memory, search for existing matches:

```bash
suda recall --json "preferred language"
```

If a relevant memory already exists, update it instead of creating a duplicate:

```bash
suda update 42 --content "Rust (confirmed again)"
```

The `update` command accepts `--name`, `--description`, `--content`, `--type`, and `--project` flags. Only the fields you pass are changed.

If the memory is no longer relevant, remove it:

```bash
suda forget 42
```

This discipline keeps the memory store lean and avoids conflicting or redundant entries that would confuse the agent.
