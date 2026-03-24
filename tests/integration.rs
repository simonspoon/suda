use std::path::PathBuf;
use std::process::Command;

struct TestEnv {
    dir: PathBuf,
}

impl TestEnv {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("suda-test-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        Self { dir }
    }

    fn suda(&self, args: &[&str]) -> std::process::Output {
        let bin = env!("CARGO_BIN_EXE_suda");
        Command::new(bin)
            .args(args)
            .env("SUDA_HOME", &self.dir)
            .output()
            .expect("failed to run suda")
    }

    fn stdout(&self, args: &[&str]) -> String {
        let out = self.suda(args);
        assert!(
            out.status.success(),
            "suda {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap()
    }

    fn fails(&self, args: &[&str]) -> String {
        let out = self.suda(args);
        assert!(!out.status.success(), "expected suda {:?} to fail", args);
        let mut combined = String::from_utf8(out.stderr).unwrap();
        combined.push_str(&String::from_utf8(out.stdout).unwrap());
        combined
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

// --- Memory CRUD ---

#[test]
fn store_and_recall() {
    let env = TestEnv::new("store-recall");
    let out = env.stdout(&[
        "store",
        "--type",
        "user",
        "--name",
        "role",
        "--description",
        "user role info",
        "senior rust developer",
    ]);
    assert!(out.contains("Stored memory 1"));

    let out = env.stdout(&["recall", "rust"]);
    assert!(out.contains("senior rust developer"));
    assert!(out.contains("role"));
}

#[test]
fn recall_json_output() {
    let env = TestEnv::new("recall-json");
    env.stdout(&[
        "store",
        "--type",
        "feedback",
        "--name",
        "terse",
        "--description",
        "style pref",
        "keep responses short",
    ]);

    let out = env.stdout(&["recall", "--json", "terse"]);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0]["name"], "terse");
    assert_eq!(parsed[0]["type"], "feedback");
    assert_eq!(parsed[0]["content"], "keep responses short");
}

#[test]
fn recall_filter_by_type() {
    let env = TestEnv::new("recall-filter-type");
    env.stdout(&[
        "store",
        "--type",
        "user",
        "--name",
        "a",
        "--description",
        "d",
        "user content",
    ]);
    env.stdout(&[
        "store",
        "--type",
        "feedback",
        "--name",
        "b",
        "--description",
        "d",
        "feedback content",
    ]);

    let out = env.stdout(&["recall", "--type", "feedback", "--json"]);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0]["type"], "feedback");
}

#[test]
fn recall_filter_by_project() {
    let env = TestEnv::new("recall-filter-project");
    env.stdout(&[
        "store",
        "--type",
        "project",
        "--name",
        "a",
        "--description",
        "d",
        "--project",
        "suda",
        "suda stuff",
    ]);
    env.stdout(&[
        "store",
        "--type",
        "project",
        "--name",
        "b",
        "--description",
        "d",
        "--project",
        "wisp",
        "wisp stuff",
    ]);

    let out = env.stdout(&["recall", "--project", "suda", "--json"]);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0]["project"], "suda");
}

#[test]
fn recall_empty_db() {
    let env = TestEnv::new("recall-empty");
    let out = env.stdout(&["recall"]);
    assert!(out.contains("No memories found"));
}

#[test]
fn update_memory() {
    let env = TestEnv::new("update");
    env.stdout(&[
        "store",
        "--type",
        "user",
        "--name",
        "role",
        "--description",
        "role info",
        "junior dev",
    ]);

    env.stdout(&["update", "1", "--content", "senior dev"]);

    let out = env.stdout(&["recall", "--json", "senior"]);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0]["content"], "senior dev");
}

#[test]
fn forget_memory() {
    let env = TestEnv::new("forget");
    env.stdout(&[
        "store",
        "--type",
        "user",
        "--name",
        "tmp",
        "--description",
        "d",
        "delete me",
    ]);

    let out = env.stdout(&["forget", "1"]);
    assert!(out.contains("Deleted memory 1"));

    let out = env.stdout(&["recall"]);
    assert!(out.contains("No memories found"));
}

#[test]
fn forget_nonexistent() {
    let env = TestEnv::new("forget-nonexistent");
    let out = env.stdout(&["forget", "999"]);
    assert!(out.contains("not found") || out.contains("No memory"));
}

// --- FTS5 search ---

#[test]
fn fts_search_matches_content() {
    let env = TestEnv::new("fts-content");
    env.stdout(&[
        "store",
        "--type",
        "reference",
        "--name",
        "api-docs",
        "--description",
        "API reference",
        "the authentication endpoint uses JWT tokens",
    ]);
    env.stdout(&[
        "store",
        "--type",
        "reference",
        "--name",
        "db-docs",
        "--description",
        "database notes",
        "postgres connection pooling config",
    ]);

    let out = env.stdout(&["recall", "--json", "JWT"]);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0]["name"], "api-docs");
}

#[test]
fn fts_search_matches_name_and_description() {
    let env = TestEnv::new("fts-name-desc");
    env.stdout(&[
        "store",
        "--type",
        "user",
        "--name",
        "kubernetes-expertise",
        "--description",
        "cloud infrastructure skills",
        "knows k8s well",
    ]);

    // Search by name
    let out = env.stdout(&["recall", "--json", "kubernetes"]);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed.len(), 1);

    // Search by description
    let out = env.stdout(&["recall", "--json", "infrastructure"]);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed.len(), 1);
}

// --- Projects ---

#[test]
fn project_lifecycle() {
    let env = TestEnv::new("project-lifecycle");

    // Add
    let out = env.stdout(&[
        "project",
        "add",
        "myapp",
        "/home/user/myapp",
        "--description",
        "main application",
    ]);
    assert!(out.contains("Registered project"));

    // List
    let out = env.stdout(&["projects"]);
    assert!(out.contains("myapp"));
    assert!(out.contains("/home/user/myapp"));

    // Show
    let out = env.stdout(&["project", "show", "myapp"]);
    assert!(out.contains("main application"));

    // Show JSON
    let out = env.stdout(&["project", "show", "myapp", "--json"]);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed[0]["name"], "myapp");

    // Remove
    let out = env.stdout(&["project", "remove", "myapp"]);
    assert!(out.contains("Unregistered"));

    // Verify removed
    let out = env.stdout(&["projects"]);
    assert!(!out.contains("myapp"));
}

#[test]
fn project_show_nonexistent() {
    let env = TestEnv::new("project-nonexistent");
    let out = env.fails(&["project", "show", "nope"]);
    assert!(out.contains("not found"));
}

// --- State ---

#[test]
fn state_lifecycle() {
    let env = TestEnv::new("state-lifecycle");

    // Set
    let out = env.stdout(&["state", "set", "session", "working on suda"]);
    assert!(out.contains("Set 'session'"));

    // Get
    let out = env.stdout(&["state", "get", "session"]);
    assert!(out.contains("working on suda"));

    // Get JSON
    let out = env.stdout(&["state", "get", "session", "--json"]);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed[0]["value"], "working on suda");

    // List
    let out = env.stdout(&["state", "list"]);
    assert!(out.contains("session"));

    // Upsert (set again)
    env.stdout(&["state", "set", "session", "now testing"]);
    let out = env.stdout(&["state", "get", "session"]);
    assert!(out.contains("now testing"));

    // Delete
    let out = env.stdout(&["state", "delete", "session"]);
    assert!(out.contains("Deleted"));

    // Verify deleted
    let out = env.fails(&["state", "get", "session"]);
    assert!(out.contains("not found"));
}

// --- Export / Import ---

#[test]
fn export_import_roundtrip() {
    let env = TestEnv::new("export-import");

    // Store some memories
    env.stdout(&[
        "store",
        "--type",
        "user",
        "--name",
        "role",
        "--description",
        "user role",
        "engineer",
    ]);
    env.stdout(&[
        "store",
        "--type",
        "feedback",
        "--name",
        "style",
        "--description",
        "style pref",
        "be terse",
    ]);

    // Export to JSON
    let exported = env.stdout(&["export", "--format", "json"]);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&exported).unwrap();
    assert_eq!(parsed.len(), 2);

    // Write to a temp file
    let export_path = env.dir.join("export.json");
    std::fs::write(&export_path, &exported).unwrap();

    // Create a fresh env and import
    let env2 = TestEnv::new("export-import-target");
    env2.stdout(&["import", export_path.to_str().unwrap()]);

    // Verify imported
    let out = env2.stdout(&["recall", "--json"]);
    let imported: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
    assert_eq!(imported.len(), 2);
}

#[test]
fn export_filter_by_type() {
    let env = TestEnv::new("export-filter");
    env.stdout(&[
        "store",
        "--type",
        "user",
        "--name",
        "a",
        "--description",
        "d",
        "user stuff",
    ]);
    env.stdout(&[
        "store",
        "--type",
        "feedback",
        "--name",
        "b",
        "--description",
        "d",
        "feedback stuff",
    ]);

    let out = env.stdout(&["export", "--format", "json", "--type", "user"]);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0]["type"], "user");
}

#[test]
fn export_markdown() {
    let env = TestEnv::new("export-md");
    env.stdout(&[
        "store",
        "--type",
        "user",
        "--name",
        "role",
        "--description",
        "user role info",
        "rust developer",
    ]);

    let out = env.stdout(&["export", "--format", "md"]);
    assert!(out.contains("# role"));
    assert!(out.contains("rust developer"));
}

// --- Init ---

#[test]
fn init_creates_db() {
    let env = TestEnv::new("init");
    let out = env.stdout(&["init"]);
    assert!(out.contains("initialized"));
    assert!(env.dir.join("suda.db").exists());
}

// --- Isolation ---

#[test]
fn separate_envs_are_isolated() {
    let env1 = TestEnv::new("isolation-1");
    let env2 = TestEnv::new("isolation-2");

    env1.stdout(&[
        "store",
        "--type",
        "user",
        "--name",
        "env1-only",
        "--description",
        "d",
        "only in env1",
    ]);

    let out = env2.stdout(&["recall"]);
    assert!(out.contains("No memories found"));
}
