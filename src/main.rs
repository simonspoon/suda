mod db;
mod display;
mod memory;
mod project;
mod state;

use clap::{Parser, Subcommand, ValueEnum};
use std::io::Read;
use std::process;

#[derive(Parser)]
#[command(
    name = "suda",
    about = "Structured memory and knowledge management CLI"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Store a new memory
    Store {
        /// Memory content (or use --stdin to read from stdin)
        content: Option<String>,
        /// Memory type
        #[arg(long = "type", value_enum)]
        memory_type: MemoryType,
        /// Memory name
        #[arg(long)]
        name: String,
        /// Memory description
        #[arg(long, default_value = "")]
        description: String,
        /// Associated project
        #[arg(long)]
        project: Option<String>,
        /// Read content from stdin
        #[arg(long)]
        stdin: bool,
    },
    /// Search and retrieve memories
    Recall {
        /// Search query (FTS5 full-text search)
        query: Option<String>,
        /// Filter by type
        #[arg(long = "type", value_enum)]
        memory_type: Option<MemoryType>,
        /// Filter by project
        #[arg(long)]
        project: Option<String>,
        /// Maximum results
        #[arg(long, default_value = "20")]
        limit: i64,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Update an existing memory
    Update {
        /// Memory ID
        id: i64,
        /// New name
        #[arg(long)]
        name: Option<String>,
        /// New description
        #[arg(long)]
        description: Option<String>,
        /// New content
        #[arg(long)]
        content: Option<String>,
        /// New type
        #[arg(long = "type", value_enum)]
        memory_type: Option<MemoryType>,
        /// New project association
        #[arg(long)]
        project: Option<String>,
    },
    /// Delete a memory
    Forget {
        /// Memory ID
        id: i64,
    },
    /// List all registered projects
    Projects {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Manage projects
    Project {
        #[command(subcommand)]
        command: ProjectCommands,
    },
    /// Manage session state
    State {
        #[command(subcommand)]
        command: StateCommands,
    },
    /// Initialize the database
    Init,
    /// Export memories
    Export {
        /// Filter by type
        #[arg(long = "type", value_enum)]
        memory_type: Option<MemoryType>,
        /// Filter by project
        #[arg(long)]
        project: Option<String>,
        /// Output format
        #[arg(long, default_value = "json", value_enum)]
        format: ExportFormat,
    },
    /// Import memories from a JSON file
    Import {
        /// Path to JSON file
        file: String,
    },
}

#[derive(Subcommand)]
enum ProjectCommands {
    /// Register a project
    Add {
        /// Project name
        name: String,
        /// Absolute path to project
        path: String,
        /// Project description
        #[arg(long)]
        description: Option<String>,
    },
    /// Unregister a project
    Remove {
        /// Project name
        name: String,
    },
    /// Show project details
    Show {
        /// Project name
        name: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum StateCommands {
    /// Get a state value
    Get {
        /// State namespace (e.g. session-state)
        name: String,
        /// Specific key within the namespace
        #[arg(long)]
        key: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
        /// Flag entries as stale if older than this duration (e.g. 24h, 30m, 7d)
        #[arg(long)]
        check_stale: Option<String>,
    },
    /// Set a state value
    Set {
        /// State namespace (e.g. session-state)
        name: String,
        /// State value (or use --stdin to read from stdin)
        value: Option<String>,
        /// Specific key within the namespace
        #[arg(long)]
        key: Option<String>,
        /// Read value from stdin
        #[arg(long)]
        stdin: bool,
    },
    /// Verify a key (update its verified_at timestamp)
    Verify {
        /// State namespace
        name: String,
        /// Key to verify
        #[arg(long)]
        key: String,
    },
    /// List all state entries
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Delete a state key
    Delete {
        /// State namespace
        name: String,
        /// Specific key within the namespace
        #[arg(long)]
        key: Option<String>,
    },
}

#[derive(Clone, ValueEnum)]
enum MemoryType {
    User,
    Feedback,
    Project,
    Reference,
}

impl MemoryType {
    fn as_str(&self) -> &'static str {
        match self {
            MemoryType::User => "user",
            MemoryType::Feedback => "feedback",
            MemoryType::Project => "project",
            MemoryType::Reference => "reference",
        }
    }
}

#[derive(Clone, ValueEnum)]
enum ExportFormat {
    Json,
    Md,
}

fn read_stdin() -> String {
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .expect("Failed to read from stdin");
    buf
}

fn main() {
    let cli = Cli::parse();

    let conn = match db::connect() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: Failed to initialize database: {e}");
            process::exit(1);
        }
    };

    let result = run(cli.command, &conn);
    if let Err(e) = result {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}

fn run(command: Commands, conn: &rusqlite::Connection) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        Commands::Store {
            content,
            memory_type,
            name,
            description,
            project,
            stdin,
        } => {
            let content = if stdin {
                read_stdin()
            } else {
                content.ok_or("Content is required (provide as argument or use --stdin)")?
            };
            let id = memory::store(
                conn,
                &name,
                &description,
                memory_type.as_str(),
                &content,
                project.as_deref(),
            )?;
            display::memory_stored(id);
        }
        Commands::Recall {
            query,
            memory_type,
            project,
            limit,
            json,
        } => {
            let memories = memory::recall(
                conn,
                query.as_deref(),
                memory_type.as_ref().map(|t| t.as_str()),
                project.as_deref(),
                limit,
            )?;
            if json {
                display::memory_json(&memories);
            } else if memories.len() == 1 && query.is_some() {
                display::memory_detail(&memories[0]);
            } else {
                display::memory_table(&memories);
            }
        }
        Commands::Update {
            id,
            name,
            description,
            content,
            memory_type,
            project,
        } => {
            let updated = memory::update(
                conn,
                id,
                name.as_deref(),
                description.as_deref(),
                content.as_deref(),
                memory_type.as_ref().map(|t| t.as_str()),
                project.as_deref(),
            )?;
            display::memory_updated(id, updated);
        }
        Commands::Forget { id } => {
            let deleted = memory::forget(conn, id)?;
            display::memory_forgotten(id, deleted);
        }
        Commands::Projects { json } => {
            let projects = project::list(conn)?;
            if json {
                display::project_json(&projects);
            } else {
                display::project_table(&projects);
            }
        }
        Commands::Project { command } => match command {
            ProjectCommands::Add {
                name,
                path,
                description,
            } => {
                project::add(conn, &name, &path, description.as_deref())?;
                display::project_added(&name);
            }
            ProjectCommands::Remove { name } => {
                let removed = project::remove(conn, &name)?;
                display::project_removed(&name, removed);
            }
            ProjectCommands::Show { name, json } => {
                let p = project::show(conn, &name)?;
                match p {
                    Some(p) => {
                        if json {
                            display::project_json(&[p]);
                        } else {
                            display::project_detail(&p);
                        }
                    }
                    None => {
                        eprintln!("Project '{name}' not found");
                        process::exit(1);
                    }
                }
            }
        },
        Commands::State { command } => match command {
            StateCommands::Get {
                name,
                key,
                json,
                check_stale,
            } => {
                match key {
                    Some(k) => {
                        // Get a specific key from the namespace
                        let entry = state::get_key(conn, &name, &k)?;
                        match entry {
                            Some(mut e) => {
                                if let Some(ref dur) = check_stale {
                                    let secs = state::parse_duration(dur).ok_or(
                                        format!("Invalid duration '{dur}'. Use e.g. 24h, 30m, 7d"),
                                    )?;
                                    state::apply_staleness(std::slice::from_mut(&mut e), secs);
                                }
                                if json {
                                    display::state_key_json(&[e]);
                                } else {
                                    display::state_key_detail(&e);
                                }
                            }
                            None => {
                                eprintln!("Key '{k}' not found in '{name}'");
                                process::exit(1);
                            }
                        }
                    }
                    None => {
                        // No --key: try structured keys first, fall back to legacy
                        let keys = state::get_all_keys(conn, &name)?;
                        if !keys.is_empty() {
                            let mut keys = keys;
                            if let Some(ref dur) = check_stale {
                                let secs = state::parse_duration(dur).ok_or(
                                    format!("Invalid duration '{dur}'. Use e.g. 24h, 30m, 7d"),
                                )?;
                                state::apply_staleness(&mut keys, secs);
                            }
                            if json {
                                display::state_key_json(&keys);
                            } else {
                                for e in &keys {
                                    display::state_key_detail(e);
                                    println!();
                                }
                            }
                        } else {
                            // Fall back to legacy flat state
                            let entry = state::get(conn, &name)?;
                            match entry {
                                Some(e) => {
                                    if json {
                                        display::state_json(&[e]);
                                    } else {
                                        display::state_detail(&e);
                                    }
                                }
                                None => {
                                    eprintln!("Key '{name}' not found");
                                    process::exit(1);
                                }
                            }
                        }
                    }
                }
            }
            StateCommands::Set {
                name,
                value,
                key,
                stdin,
            } => {
                let value = if stdin {
                    read_stdin()
                } else {
                    value.ok_or("Value is required (provide as argument or use --stdin)")?
                };
                match key {
                    Some(k) => {
                        state::set_key(conn, &name, &k, &value)?;
                        display::state_set(&format!("{name}:{k}"));
                    }
                    None => {
                        state::set(conn, &name, &value)?;
                        display::state_set(&name);
                    }
                }
            }
            StateCommands::Verify { name, key } => {
                let found = state::verify_key(conn, &name, &key)?;
                if found {
                    println!("Verified '{name}:{key}'");
                } else {
                    eprintln!("Key '{key}' not found in '{name}'");
                    process::exit(1);
                }
            }
            StateCommands::List { json } => {
                let entries = state::list(conn)?;
                if json {
                    display::state_json(&entries);
                } else {
                    display::state_table(&entries);
                }
            }
            StateCommands::Delete { name, key } => {
                match key {
                    Some(k) => {
                        let deleted = state::delete_key(conn, &name, &k)?;
                        if deleted {
                            println!("Deleted '{name}:{k}'");
                        } else {
                            println!("Key '{k}' not found in '{name}'");
                        }
                    }
                    None => {
                        let deleted = state::delete(conn, &name)?;
                        display::state_deleted(&name, deleted);
                    }
                }
            }
        },
        Commands::Init => {
            println!("Database initialized at {:?}", db::db_path());
        }
        Commands::Export {
            memory_type,
            project,
            format,
        } => {
            let memories = memory::export(
                conn,
                memory_type.as_ref().map(|t| t.as_str()),
                project.as_deref(),
            )?;
            match format {
                ExportFormat::Json => display::export_json(&memories),
                ExportFormat::Md => display::export_markdown(&memories),
            }
        }
        Commands::Import { file } => {
            let data = std::fs::read_to_string(&file)
                .map_err(|e| format!("Failed to read file '{file}': {e}"))?;
            let memories: Vec<memory::Memory> =
                serde_json::from_str(&data).map_err(|e| format!("Failed to parse JSON: {e}"))?;
            let count = memory::import(conn, &memories)?;
            display::import_result(count);
        }
    }
    Ok(())
}
