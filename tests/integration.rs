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
    assert_eq!(parsed[0]["strength"], 1);
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

    // Reinforce "role" so strength > 1
    env.stdout(&["reinforce", "1"]);
    env.stdout(&["reinforce", "1"]);
    // role should now have strength 3, style stays at 1

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

    // Verify imported memories preserve strength
    let out = env2.stdout(&["recall", "--json"]);
    let imported: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
    assert_eq!(imported.len(), 2);

    // Find the "role" memory and verify its strength survived
    let role = imported
        .iter()
        .find(|m| m["name"] == "role")
        .expect("role memory not found");
    assert_eq!(
        role["strength"], 3,
        "reinforced strength should survive export/import"
    );

    let style = imported
        .iter()
        .find(|m| m["name"] == "style")
        .expect("style memory not found");
    assert_eq!(
        style["strength"], 1,
        "default strength should survive export/import"
    );
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

// --- Structured state keys ---

#[test]
fn state_key_set_and_get() {
    let env = TestEnv::new("state-key-set-get");

    // Set a key within a namespace
    let out = env.stdout(&[
        "state",
        "set",
        "session-state",
        "--key",
        "ordis-status",
        "Fourth commit, not pushed",
    ]);
    assert!(out.contains("Set 'session-state:ordis-status'"));

    // Get that specific key
    let out = env.stdout(&["state", "get", "session-state", "--key", "ordis-status"]);
    assert!(out.contains("Fourth commit, not pushed"));
    assert!(out.contains("ordis-status"));
}

#[test]
fn state_key_get_all() {
    let env = TestEnv::new("state-key-get-all");

    env.stdout(&[
        "state",
        "set",
        "session-state",
        "--key",
        "ordis-status",
        "building UI",
    ]);
    env.stdout(&[
        "state",
        "set",
        "session-state",
        "--key",
        "wisp-status",
        "v0.3.0 pushed",
    ]);

    // Get all keys in namespace
    let out = env.stdout(&["state", "get", "session-state"]);
    assert!(out.contains("ordis-status"));
    assert!(out.contains("wisp-status"));
    assert!(out.contains("building UI"));
    assert!(out.contains("v0.3.0 pushed"));
}

#[test]
fn state_key_json_output() {
    let env = TestEnv::new("state-key-json");

    env.stdout(&[
        "state",
        "set",
        "session-state",
        "--key",
        "ordis-status",
        "Fourth commit",
    ]);

    let out = env.stdout(&[
        "state",
        "get",
        "session-state",
        "--key",
        "ordis-status",
        "--json",
    ]);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0]["key"], "ordis-status");
    assert_eq!(parsed[0]["value"], "Fourth commit");
    assert_eq!(parsed[0]["namespace"], "session-state");
    assert!(parsed[0]["updated_at"].is_string());
}

#[test]
fn state_key_verify() {
    let env = TestEnv::new("state-key-verify");

    env.stdout(&[
        "state",
        "set",
        "session-state",
        "--key",
        "ordis-status",
        "building UI",
    ]);

    // Verify the key
    let out = env.stdout(&["state", "verify", "session-state", "--key", "ordis-status"]);
    assert!(out.contains("Verified"));

    // Check verified_at appears in JSON
    let out = env.stdout(&[
        "state",
        "get",
        "session-state",
        "--key",
        "ordis-status",
        "--json",
    ]);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
    assert!(parsed[0]["verified_at"].is_string());
}

#[test]
fn state_key_verify_nonexistent() {
    let env = TestEnv::new("state-key-verify-none");
    let out = env.fails(&["state", "verify", "session-state", "--key", "nope"]);
    assert!(out.contains("not found"));
}

#[test]
fn state_key_check_stale_fresh() {
    let env = TestEnv::new("state-key-stale-fresh");

    env.stdout(&[
        "state",
        "set",
        "session-state",
        "--key",
        "ordis-status",
        "building UI",
    ]);

    // With a 24h threshold, a just-set key should be fresh
    let out = env.stdout(&[
        "state",
        "get",
        "session-state",
        "--key",
        "ordis-status",
        "--json",
        "--check-stale",
        "24h",
    ]);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed[0]["stale"], false);
}

#[test]
fn state_key_check_stale_expired() {
    let env = TestEnv::new("state-key-stale-expired");

    env.stdout(&[
        "state",
        "set",
        "session-state",
        "--key",
        "ordis-status",
        "building UI",
    ]);

    // With 0-second threshold, should be stale
    let out = env.stdout(&[
        "state",
        "get",
        "session-state",
        "--json",
        "--check-stale",
        "0s",
    ]);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed[0]["stale"], true);
}

#[test]
fn state_key_delete() {
    let env = TestEnv::new("state-key-delete");

    env.stdout(&[
        "state",
        "set",
        "session-state",
        "--key",
        "ordis-status",
        "building UI",
    ]);

    let out = env.stdout(&["state", "delete", "session-state", "--key", "ordis-status"]);
    assert!(out.contains("Deleted"));

    let out = env.fails(&["state", "get", "session-state", "--key", "ordis-status"]);
    assert!(out.contains("not found"));
}

#[test]
fn state_key_upsert() {
    let env = TestEnv::new("state-key-upsert");

    env.stdout(&[
        "state",
        "set",
        "session-state",
        "--key",
        "ordis-status",
        "first value",
    ]);

    env.stdout(&[
        "state",
        "set",
        "session-state",
        "--key",
        "ordis-status",
        "updated value",
    ]);

    let out = env.stdout(&["state", "get", "session-state", "--key", "ordis-status"]);
    assert!(out.contains("updated value"));
    assert!(!out.contains("first value"));
}

#[test]
fn state_legacy_and_keys_coexist() {
    let env = TestEnv::new("state-coexist");

    // Set legacy flat state
    env.stdout(&["state", "set", "session-state", "legacy blob of text"]);

    // Set structured keys in same namespace
    env.stdout(&[
        "state",
        "set",
        "session-state",
        "--key",
        "ordis-status",
        "building",
    ]);

    // Get without --key returns structured keys (they take priority)
    let out = env.stdout(&["state", "get", "session-state"]);
    assert!(out.contains("ordis-status"));

    // Legacy state still accessible via list
    let out = env.stdout(&["state", "list"]);
    assert!(out.contains("session-state"));
}

// --- Strength / Reinforce ---

#[test]
fn store_has_default_strength() {
    let env = TestEnv::new("store-default-strength");
    env.stdout(&[
        "store",
        "--type",
        "user",
        "--name",
        "role",
        "--description",
        "user role",
        "rust developer",
    ]);

    let out = env.stdout(&["recall", "--json"]);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0]["strength"], 1);
}

#[test]
fn reinforce_increments_strength() {
    let env = TestEnv::new("reinforce-increment");
    env.stdout(&[
        "store",
        "--type",
        "user",
        "--name",
        "role",
        "--description",
        "user role",
        "rust developer",
    ]);

    // First reinforce: 1 -> 2
    let out = env.stdout(&["reinforce", "1"]);
    assert!(out.contains("Reinforced memory 1"));
    assert!(out.contains("strength: 2"));

    let out = env.stdout(&["recall", "--json"]);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed[0]["strength"], 2);

    // Second reinforce: 2 -> 3
    let out = env.stdout(&["reinforce", "1"]);
    assert!(out.contains("strength: 3"));

    let out = env.stdout(&["recall", "--json"]);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed[0]["strength"], 3);
}

#[test]
fn reinforce_nonexistent() {
    let env = TestEnv::new("reinforce-nonexistent");
    let out = env.stdout(&["reinforce", "999"]);
    assert!(out.contains("not found"));
}

#[test]
fn reinforce_set_explicit() {
    let env = TestEnv::new("reinforce-set");
    env.stdout(&[
        "store",
        "--type",
        "user",
        "--name",
        "role",
        "--description",
        "user role",
        "rust developer",
    ]);

    // Set strength to exactly 5
    let out = env.stdout(&["reinforce", "1", "--set", "5"]);
    assert!(out.contains("Reinforced memory 1"));
    assert!(out.contains("strength: 5"));

    let out = env.stdout(&["recall", "--json"]);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed[0]["strength"], 5);

    // Set to a different value
    let out = env.stdout(&["reinforce", "1", "--set", "1"]);
    assert!(out.contains("strength: 1"));

    let out = env.stdout(&["recall", "--json"]);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed[0]["strength"], 1);
}

#[test]
fn reinforce_updated_at_changes() {
    let env = TestEnv::new("reinforce-updated-at");
    env.stdout(&[
        "store",
        "--type",
        "user",
        "--name",
        "role",
        "--description",
        "user role",
        "rust developer",
    ]);

    let out = env.stdout(&["recall", "--json"]);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
    let before = parsed[0]["updated_at"].as_str().unwrap().to_string();

    // Sleep to ensure SQLite datetime('now') produces a different second
    std::thread::sleep(std::time::Duration::from_secs(1));

    env.stdout(&["reinforce", "1"]);

    let out = env.stdout(&["recall", "--json"]);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
    let after = parsed[0]["updated_at"].as_str().unwrap().to_string();

    assert_ne!(before, after, "updated_at should change after reinforce");
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
