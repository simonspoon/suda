# Getting Started

Suda is a structured memory and knowledge management CLI built in Rust, designed for AI agent workflows. It stores named, typed memories in a local SQLite database with full-text search, making it easy for tools like Claude Code to persist context, preferences, and project knowledge across sessions.

## Installation

### From source

Clone the repository and install with Cargo:

```
git clone https://github.com/simonspoon/suda.git
cd suda
cargo install --path .
```

### Via Homebrew

```
brew install simonspoon/tap/suda
```

## First use

Run `suda init` to create the database:

```
suda init
```

This creates the SQLite database at `~/.suda/suda.db`. The database schema, including full-text search indexes, is set up automatically.

## Quick walkthrough

### Store a memory

```
suda store --type user --name "editor-preference" --description "Preferred text editor" "I use Neovim as my primary editor with LazyVim config"
```

### Recall memories

Search by keyword using full-text search:

```
suda recall "editor"
```

You can also filter by type or project:

```
suda recall --type user --limit 10
```

### Update a memory

Update an existing memory by its ID (shown when you store or recall):

```
suda update 1 --content "Switched to Helix editor with custom keybindings"
```

### Reinforce a memory

Increment a memory's strength to signal importance:

```
suda reinforce 1
```

Each call increases strength by 1. To set an explicit value:

```
suda reinforce 1 --set 5
```

### Forget a memory

Delete a memory by its ID:

```
suda forget 1
```

## Configuration

Suda stores its database in `~/.suda/suda.db` by default. To customize the storage location, set the `SUDA_HOME` environment variable:

```
export SUDA_HOME=/path/to/custom/directory
```

The database file will be created at `$SUDA_HOME/suda.db`.

## Memory types

Suda supports four memory types, specified with the `--type` flag:

- **user** -- Personal preferences, habits, and environment details about the user.
- **feedback** -- Corrections and guidance the user has given to improve agent behavior.
- **project** -- Architecture decisions, conventions, and context specific to a codebase.
- **reference** -- Reusable knowledge such as API patterns, templates, or documentation notes.

## Next steps

See [commands.md](commands.md) for the full command reference, including project registration, session state management, and import/export.
