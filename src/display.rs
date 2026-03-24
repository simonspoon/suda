use crate::memory::Memory;
use crate::project::Project;
use crate::state::StateEntry;

pub fn memory_table(memories: &[Memory]) {
    if memories.is_empty() {
        println!("No memories found.");
        return;
    }
    println!(
        "{:<6} {:<12} {:<30} {:<40} {:<12}",
        "ID", "TYPE", "NAME", "DESCRIPTION", "PROJECT"
    );
    println!("{}", "-".repeat(100));
    for m in memories {
        let project = m.project.as_deref().unwrap_or("-");
        let desc = truncate(&m.description, 38);
        let name = truncate(&m.name, 28);
        println!(
            "{:<6} {:<12} {:<30} {:<40} {:<12}",
            m.id, m.memory_type, name, desc, project
        );
    }
}

pub fn memory_detail(m: &Memory) {
    println!("ID:          {}", m.id);
    println!("Name:        {}", m.name);
    println!("Type:        {}", m.memory_type);
    println!("Description: {}", m.description);
    println!("Project:     {}", m.project.as_deref().unwrap_or("(none)"));
    println!("Created:     {}", m.created_at);
    println!("Updated:     {}", m.updated_at);
    println!("---");
    println!("{}", m.content);
}

pub fn memory_json(memories: &[Memory]) {
    let json = serde_json::to_string_pretty(memories).expect("Failed to serialize memories");
    println!("{json}");
}

pub fn memory_stored(id: i64) {
    println!("Stored memory {id}");
}

pub fn memory_updated(id: i64, updated: bool) {
    if updated {
        println!("Updated memory {id}");
    } else {
        println!("Memory {id} not found");
    }
}

pub fn memory_forgotten(id: i64, deleted: bool) {
    if deleted {
        println!("Deleted memory {id}");
    } else {
        println!("Memory {id} not found");
    }
}

pub fn project_table(projects: &[Project]) {
    if projects.is_empty() {
        println!("No projects registered.");
        return;
    }
    println!("{:<20} {:<50} {:<30}", "NAME", "PATH", "DESCRIPTION");
    println!("{}", "-".repeat(100));
    for p in projects {
        let desc = p.description.as_deref().unwrap_or("-");
        let desc = truncate(desc, 28);
        println!("{:<20} {:<50} {:<30}", p.name, p.path, desc);
    }
}

pub fn project_detail(p: &Project) {
    println!("Name:        {}", p.name);
    println!("Path:        {}", p.path);
    println!(
        "Description: {}",
        p.description.as_deref().unwrap_or("(none)")
    );
    println!("Created:     {}", p.created_at);
    println!("Updated:     {}", p.updated_at);
}

pub fn project_json(projects: &[Project]) {
    let json = serde_json::to_string_pretty(projects).expect("Failed to serialize projects");
    println!("{json}");
}

pub fn project_added(name: &str) {
    println!("Registered project '{name}'");
}

pub fn project_removed(name: &str, removed: bool) {
    if removed {
        println!("Unregistered project '{name}'");
    } else {
        println!("Project '{name}' not found");
    }
}

pub fn state_table(entries: &[StateEntry]) {
    if entries.is_empty() {
        println!("No state entries.");
        return;
    }
    println!("{:<30} {:<50} {:<20}", "KEY", "VALUE", "UPDATED");
    println!("{}", "-".repeat(100));
    for e in entries {
        let val = truncate(&e.value, 48);
        println!("{:<30} {:<50} {:<20}", e.key, val, e.updated_at);
    }
}

pub fn state_detail(entry: &StateEntry) {
    println!("{}", entry.value);
}

pub fn state_json(entries: &[StateEntry]) {
    let json = serde_json::to_string_pretty(entries).expect("Failed to serialize state");
    println!("{json}");
}

pub fn state_set(key: &str) {
    println!("Set '{key}'");
}

pub fn state_deleted(key: &str, deleted: bool) {
    if deleted {
        println!("Deleted '{key}'");
    } else {
        println!("Key '{key}' not found");
    }
}

pub fn export_json(memories: &[Memory]) {
    let json = serde_json::to_string_pretty(memories).expect("Failed to serialize");
    println!("{json}");
}

pub fn export_markdown(memories: &[Memory]) {
    for m in memories {
        println!("## {} (ID: {})", m.name, m.id);
        println!(
            "**Type:** {} | **Project:** {}",
            m.memory_type,
            m.project.as_deref().unwrap_or("none")
        );
        println!();
        println!("> {}", m.description);
        println!();
        println!("{}", m.content);
        println!();
        println!("---");
        println!();
    }
}

pub fn import_result(count: usize) {
    println!("Imported {count} memories");
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}
