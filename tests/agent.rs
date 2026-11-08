//! Integration tests for the agent-first contract (docs/agent-protocol.md).
//!
//! Every test runs the binary with its own temporary HOME; the real `~/.tam`
//! is never written. Only `std` + existing dependencies are used.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Output, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::Value;

static NEXT_HOME: AtomicU32 = AtomicU32::new(0);

struct TempHome {
    path: PathBuf,
}

impl TempHome {
    fn new() -> Self {
        let unique = format!(
            "tam-agent-test-{}-{}-{}",
            std::process::id(),
            NEXT_HOME.fetch_add(1, Ordering::SeqCst),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock before UNIX epoch")
                .as_nanos()
        );
        let path = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&path).expect("failed to create temporary HOME");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn tam_dir(&self) -> PathBuf {
        self.path.join(".tam")
    }
}

impl Drop for TempHome {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn command_for(home: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tam"));
    command
        .env_clear()
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("TZ", "UTC");
    if let Some(path) = std::env::var_os("PATH") {
        command.env("PATH", path);
    }
    command
}

fn run_tam(home: &TempHome, args: &[&str]) -> Output {
    command_for(home.path())
        .args(args)
        .output()
        .expect("failed to spawn tam binary")
}

fn run_tam_env(home: &TempHome, args: &[&str], envs: &[(&str, &str)]) -> Output {
    let mut command = command_for(home.path());
    command.args(args);
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().expect("failed to spawn tam binary")
}

#[track_caller]
fn json_stdout(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout is not valid JSON: {}\nstdout:\n{}\nstderr:\n{}",
            error,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

#[track_caller]
fn json_stderr(output: &Output) -> Value {
    serde_json::from_slice(&output.stderr).unwrap_or_else(|error| {
        panic!(
            "stderr is not valid JSON: {}\nstdout:\n{}\nstderr:\n{}",
            error,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

#[track_caller]
fn assert_ok(output: &Output) -> Value {
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected success\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value = json_stdout(output);
    assert_eq!(value["ok"], Value::Bool(true));
    assert_eq!(value["schema"], "tam.agent/v1");
    assert!(
        value.get("command").and_then(Value::as_str).is_some(),
        "missing command field: {value}"
    );
    value
}

#[track_caller]
fn assert_error(output: &Output, exit_code: i32, code: &str) -> Value {
    assert_eq!(
        output.status.code(),
        Some(exit_code),
        "unexpected exit code\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stdout.is_empty(),
        "error envelope must go to stderr only:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let value = json_stderr(output);
    assert_eq!(value["ok"], Value::Bool(false));
    assert_eq!(value["schema"], "tam.agent/v1");
    assert_eq!(value["error"]["code"], code);
    value
}

fn create_project(home: &TempHome, name: &str) {
    let output = run_tam(home, &["project", "add", name, "--json"]);
    assert_ok(&output);
}

// ---------------------------------------------------------------------------
// Envelope / exit codes
// ---------------------------------------------------------------------------

#[test]
fn json_envelopes_and_exit_codes() {
    let home = TempHome::new();

    let added = assert_ok(&run_tam(&home, &["project", "add", "alpha", "--json"]));
    assert_eq!(added["data"]["project"]["name"], "alpha");
    assert_eq!(added["data"]["restored"], false);
    assert!(added["data"]["project"]["id"].as_i64().unwrap_or(0) > 0);

    let list = assert_ok(&run_tam(&home, &["project", "list", "--json"]));
    assert_eq!(list["data"].as_array().unwrap().len(), 1);
    assert_eq!(list["data"][0]["name"], "alpha");

    // No current project yet.
    let current = assert_ok(&run_tam(&home, &["project", "current", "--json"]));
    assert!(current["data"]["project"].is_null());

    let use_output = run_tam(&home, &["project", "use", "alpha", "--json"]);
    // `project use` is a human command; --json is rejected explicitly.
    assert_error(&use_output, 1, "invalid");

    // missing project context
    assert_error(
        &run_tam(&home, &["task", "list", "--json"]),
        1,
        "missing_project",
    );
    // not found
    assert_error(
        &run_tam(&home, &["task", "show", "999", "--json"]),
        3,
        "not_found",
    );
    // invalid priority -> validation error exit 1
    assert_error(
        &run_tam(
            &home,
            &[
                "task",
                "add",
                "x",
                "--project",
                "alpha",
                "--priority",
                "9",
                "--json",
            ],
        ),
        1,
        "invalid",
    );
    // clap usage error keeps exit code 2 and does not emit an envelope
    let usage = run_tam(&home, &["task", "show", "abc", "--json"]);
    assert_eq!(usage.status.code(), Some(2));
    assert!(usage.stdout.is_empty());
}

#[test]
fn context_from_env_and_explicit_flag() {
    let home = TempHome::new();
    create_project(&home, "alpha");
    create_project(&home, "beta");

    let from_env = run_tam_env(
        &home,
        &["task", "add", "env task", "--json"],
        &[("TAM_PROJECT", "alpha"), ("TAM_AGENT", "agent-env")],
    );
    let value = assert_ok(&from_env);
    assert_eq!(value["data"]["task"]["project"], "alpha");
    assert_eq!(value["meta"]["agent"], "agent-env");

    // Explicit --project wins over TAM_PROJECT.
    let explicit = run_tam_env(
        &home,
        &["task", "add", "flag task", "--project", "beta", "--json"],
        &[("TAM_PROJECT", "alpha")],
    );
    let value = assert_ok(&explicit);
    assert_eq!(value["data"]["task"]["project"], "beta");
}

// ---------------------------------------------------------------------------
// Task lifecycle: add / filters / show / update / events
// ---------------------------------------------------------------------------

#[test]
fn task_crud_views_and_events() {
    let home = TempHome::new();
    create_project(&home, "alpha");

    let created = assert_ok(&run_tam(
        &home,
        &[
            "task",
            "add",
            "Fix login",
            "--project",
            "alpha",
            "--description",
            "login bug",
            "--priority",
            "1",
            "--tag",
            "Bug",
            "--tag",
            "backend",
            "--agent",
            "agent-a",
            "--json",
        ],
    ));
    let task = &created["data"]["task"];
    assert_eq!(task["title"], "Fix login");
    assert_eq!(task["description"], "login bug");
    assert_eq!(task["priority"], 1);
    assert_eq!(task["tags"], serde_json::json!(["bug", "backend"]));
    assert_eq!(task["status"], "todo");
    assert_eq!(task["revision"], 1);
    assert_eq!(
        task["lease_expired"], false,
        "an unassigned task must not be reported as lease_expired"
    );
    assert_eq!(created["data"]["deduplicated"], false);
    let id = task["id"].as_i64().unwrap();

    let updated = assert_ok(&run_tam(
        &home,
        &[
            "task",
            "update",
            &id.to_string(),
            "--project",
            "alpha",
            "--title",
            "Fix login v2",
            "--status",
            "in-progress",
            "--priority",
            "2",
            "--json",
        ],
    ));
    assert_eq!(updated["data"]["task"]["title"], "Fix login v2");
    assert_eq!(updated["data"]["task"]["status"], "in-progress");
    assert_eq!(updated["data"]["task"]["priority"], 2);
    assert_eq!(updated["data"]["task"]["revision"], 2);

    let shown = assert_ok(&run_tam(
        &home,
        &["task", "show", &id.to_string(), "--json"],
    ));
    assert_eq!(shown["data"]["task"]["id"], id);
    let events = shown["data"]["events"].as_array().unwrap();
    let event_names: Vec<&str> = events
        .iter()
        .map(|event| event["event"].as_str().unwrap())
        .collect();
    assert_eq!(event_names, vec!["created", "updated"]);

    let events_only = assert_ok(&run_tam(
        &home,
        &["task", "events", &id.to_string(), "--json"],
    ));
    assert_eq!(events_only["data"]["events"].as_array().unwrap().len(), 2);

    // Human output stays human (no JSON when --json is absent).
    let human = run_tam(&home, &["task", "list", "--project", "alpha"]);
    assert_eq!(human.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&human.stdout).contains("Fix login v2"));
    assert!(!String::from_utf8_lossy(&human.stdout).contains("tam.agent/v1"));
}

#[test]
fn task_update_requires_a_field() {
    let home = TempHome::new();
    create_project(&home, "alpha");
    assert_ok(&run_tam(
        &home,
        &["task", "add", "x", "--project", "alpha", "--json"],
    ));
    let output = run_tam(
        &home,
        &["task", "update", "1", "--project", "alpha", "--json"],
    );
    assert_error(&output, 1, "invalid");
}

#[test]
fn filters_and_next_ordering() {
    let home = TempHome::new();
    create_project(&home, "alpha");

    for (title, priority, tag) in [
        ("low-task", "5", "backend"),
        ("high-task", "1", "bug"),
        ("mid-task", "3", "bug"),
    ] {
        let output = run_tam(
            &home,
            &[
                "task",
                "add",
                title,
                "--project",
                "alpha",
                "--priority",
                priority,
                "--tag",
                tag,
                "--json",
            ],
        );
        assert_ok(&output);
    }

    let filtered = assert_ok(&run_tam(
        &home,
        &[
            "task",
            "list",
            "--project",
            "alpha",
            "--priority",
            "1",
            "--json",
        ],
    ));
    assert_eq!(filtered["data"]["count"], 1);
    assert_eq!(filtered["data"]["tasks"][0]["title"], "high-task");

    let tagged = assert_ok(&run_tam(
        &home,
        &[
            "task",
            "list",
            "--project",
            "alpha",
            "--tag",
            "BUG",
            "--json",
        ],
    ));
    assert_eq!(tagged["data"]["count"], 2);

    let searched = assert_ok(&run_tam(
        &home,
        &[
            "task",
            "list",
            "--project",
            "alpha",
            "--search",
            "HIGH",
            "--json",
        ],
    ));
    assert_eq!(searched["data"]["count"], 1);
    assert_eq!(searched["data"]["tasks"][0]["title"], "high-task");

    let limited = assert_ok(&run_tam(
        &home,
        &[
            "task",
            "list",
            "--project",
            "alpha",
            "--limit",
            "2",
            "--json",
        ],
    ));
    assert_eq!(limited["data"]["count"], 2);
    assert_eq!(limited["data"]["truncated"], true);

    // next: priority ascending (1 first), then created ascending.
    let next = assert_ok(&run_tam(
        &home,
        &["task", "next", "--project", "alpha", "--json"],
    ));
    assert_eq!(next["data"]["task"]["title"], "high-task");

    let claimed = assert_ok(&run_tam(
        &home,
        &[
            "task",
            "next",
            "--project",
            "alpha",
            "--agent",
            "agent-a",
            "--claim",
            "--json",
        ],
    ));
    assert_eq!(claimed["data"]["task"]["title"], "high-task");
    assert_eq!(claimed["data"]["task"]["status"], "in-progress");

    let after = assert_ok(&run_tam(
        &home,
        &["task", "next", "--project", "alpha", "--json"],
    ));
    assert_eq!(after["data"]["task"]["title"], "mid-task");
}

// ---------------------------------------------------------------------------
// Leases, claim atomics, idempotency, isolation
// ---------------------------------------------------------------------------

fn add_task(home: &TempHome, project: &str, title: &str) -> i64 {
    let output = run_tam(
        home,
        &["task", "add", title, "--project", project, "--json"],
    );
    let value = assert_ok(&output);
    value["data"]["task"]["id"].as_i64().unwrap()
}

#[test]
fn concurrent_claim_has_exactly_one_winner() {
    let home = TempHome::new();
    create_project(&home, "alpha");
    add_task(&home, "alpha", "contended");

    let home_path = home.path().to_path_buf();
    let mut handles = Vec::new();
    for agent in ["agent-a", "agent-b"] {
        let home_path = home_path.clone();
        handles.push(std::thread::spawn(move || {
            let output = command_for(&home_path)
                .args(["task", "claim", "1", "--agent", agent, "--json"])
                .output()
                .expect("failed to spawn claim");
            (agent.to_string(), output.status.code(), output)
        }));
    }
    let results: Vec<(String, Option<i32>, Output)> = handles
        .into_iter()
        .map(|handle| handle.join().expect("claim thread panicked"))
        .collect();

    let winners = results
        .iter()
        .filter(|(_, code, _)| *code == Some(0))
        .count();
    let conflicts = results
        .iter()
        .filter(|(_, code, _)| *code == Some(4))
        .count();
    assert_eq!(
        (winners, conflicts),
        (1, 1),
        "expected exactly one winner and one conflict: {:?}",
        results
            .iter()
            .map(|(agent, code, output)| (
                agent,
                code,
                String::from_utf8_lossy(&output.stderr).to_string()
            ))
            .collect::<Vec<_>>()
    );
    for (_, code, output) in &results {
        if *code == Some(4) {
            assert_error(output, 4, "conflict");
        }
    }
}

#[test]
fn lease_expiry_allows_new_owner() {
    let home = TempHome::new();
    create_project(&home, "alpha");
    add_task(&home, "alpha", "leased");

    let claimed = assert_ok(&run_tam(
        &home,
        &[
            "task", "claim", "1", "--agent", "agent-a", "--lease", "5", "--json",
        ],
    ));
    assert_eq!(claimed["data"]["task"]["assigned_agent"], "agent-a");
    assert_eq!(claimed["data"]["task"]["lease_expired"], false);

    // Short lease (the protocol minimum) plus a little slack.
    std::thread::sleep(Duration::from_millis(5500));

    let taken = assert_ok(&run_tam(
        &home,
        &["task", "claim", "1", "--agent", "agent-b", "--json"],
    ));
    assert_eq!(taken["data"]["task"]["assigned_agent"], "agent-b");
}

#[test]
fn heartbeat_and_release_lifecycle() {
    let home = TempHome::new();
    create_project(&home, "alpha");
    add_task(&home, "alpha", "lifecycle");

    assert_ok(&run_tam(
        &home,
        &[
            "task", "claim", "1", "--agent", "agent-a", "--lease", "60", "--json",
        ],
    ));

    // Non-holder heartbeat is a conflict.
    let wrong = run_tam(
        &home,
        &["task", "heartbeat", "1", "--agent", "agent-b", "--json"],
    );
    assert_error(&wrong, 4, "conflict");

    let renewed = assert_ok(&run_tam(
        &home,
        &[
            "task",
            "heartbeat",
            "1",
            "--agent",
            "agent-a",
            "--lease",
            "120",
            "--json",
        ],
    ));
    assert_eq!(renewed["data"]["task"]["assigned_agent"], "agent-a");
    assert_eq!(renewed["data"]["task"]["lease_expired"], false);
    assert_eq!(renewed["data"]["task"]["revision"], 3);

    // Non-holder release requires --force.
    let wrong_release = run_tam(
        &home,
        &["task", "release", "1", "--agent", "agent-b", "--json"],
    );
    assert_error(&wrong_release, 4, "conflict");

    let released = assert_ok(&run_tam(
        &home,
        &["task", "release", "1", "--agent", "agent-a", "--json"],
    ));
    assert_eq!(released["data"]["task"]["assigned_agent"], Value::Null);
    assert_eq!(released["data"]["task"]["status"], "todo");

    // Released tasks can be claimed again.
    let reclaimed = assert_ok(&run_tam(
        &home,
        &["task", "claim", "1", "--agent", "agent-b", "--json"],
    ));
    assert_eq!(reclaimed["data"]["task"]["assigned_agent"], "agent-b");
}

#[test]
fn done_conflicts_only_for_agents_holding_other_leases() {
    let home = TempHome::new();
    create_project(&home, "alpha");
    add_task(&home, "alpha", "guarded");

    assert_ok(&run_tam(
        &home,
        &["task", "claim", "1", "--agent", "agent-a", "--json"],
    ));

    // Another agent cannot complete a validly leased task.
    let conflict = run_tam(
        &home,
        &["task", "done", "1", "--agent", "agent-b", "--json"],
    );
    assert_error(&conflict, 4, "conflict");

    // Humans (no --agent) are allowed, per the protocol.
    let human = assert_ok(&run_tam(&home, &["task", "done", "1", "--json"]));
    assert_eq!(human["data"]["task"]["status"], "done");
    assert_eq!(human["data"]["task"]["assigned_agent"], Value::Null);

    // Done tasks cannot be claimed.
    assert_error(
        &run_tam(
            &home,
            &["task", "claim", "1", "--agent", "agent-c", "--json"],
        ),
        4,
        "conflict",
    );
}

#[test]
fn idempotency_key_is_project_scoped() {
    let home = TempHome::new();
    create_project(&home, "alpha");
    create_project(&home, "beta");

    let first = assert_ok(&run_tam(
        &home,
        &[
            "task",
            "add",
            "dedup me",
            "--project",
            "alpha",
            "--idempotency-key",
            "key-1",
            "--json",
        ],
    ));
    let first_id = first["data"]["task"]["id"].as_i64().unwrap();
    assert_eq!(first["data"]["deduplicated"], false);

    let second = assert_ok(&run_tam(
        &home,
        &[
            "task",
            "add",
            "dedup me",
            "--project",
            "alpha",
            "--idempotency-key",
            "key-1",
            "--json",
        ],
    ));
    assert_eq!(second["data"]["deduplicated"], true);
    assert_eq!(second["data"]["task"]["id"], first_id);

    // Same key in another project is independent.
    let other = assert_ok(&run_tam(
        &home,
        &[
            "task",
            "add",
            "dedup me",
            "--project",
            "beta",
            "--idempotency-key",
            "key-1",
            "--json",
        ],
    ));
    assert_eq!(other["data"]["deduplicated"], false);
    assert_ne!(other["data"]["task"]["id"], first_id);
}

#[test]
fn project_isolation_for_explicit_project() {
    let home = TempHome::new();
    create_project(&home, "alpha");
    create_project(&home, "beta");
    let beta_task = add_task(&home, "beta", "beta only");

    // Without explicit --project the legacy global-id behavior applies.
    let global = assert_ok(&run_tam(
        &home,
        &["task", "show", &beta_task.to_string(), "--json"],
    ));
    assert_eq!(global["data"]["task"]["project"], "beta");

    // With explicit --project, cross-project by-id access is not_found.
    let scoped = run_tam(
        &home,
        &[
            "task",
            "show",
            &beta_task.to_string(),
            "--project",
            "alpha",
            "--json",
        ],
    );
    assert_error(&scoped, 3, "not_found");

    let scoped_claim = run_tam(
        &home,
        &[
            "task",
            "claim",
            &beta_task.to_string(),
            "--project",
            "alpha",
            "--agent",
            "agent-a",
            "--json",
        ],
    );
    assert_error(&scoped_claim, 3, "not_found");
}

// ---------------------------------------------------------------------------
// Status summary
// ---------------------------------------------------------------------------

#[test]
fn status_summary_counts_and_next() {
    let home = TempHome::new();
    create_project(&home, "alpha");
    for (title, priority) in [("a", "3"), ("b", "1"), ("c", "2")] {
        assert_ok(&run_tam(
            &home,
            &[
                "task",
                "add",
                title,
                "--project",
                "alpha",
                "--priority",
                priority,
                "--json",
            ],
        ));
    }
    assert_ok(&run_tam(
        &home,
        &["task", "claim", "3", "--agent", "agent-a", "--json"],
    ));

    let status = assert_ok(&run_tam(&home, &["status", "--project", "alpha", "--json"]));
    assert_eq!(status["data"]["project"]["name"], "alpha");
    assert_eq!(status["data"]["counts"]["todo"], 2);
    assert_eq!(status["data"]["counts"]["in_progress"], 1);
    assert_eq!(status["data"]["counts"]["done"], 0);
    assert_eq!(status["data"]["leased"], 1);
    assert_eq!(status["data"]["unassigned"], 2);
    let next = status["data"]["next"].as_array().unwrap();
    assert_eq!(next[0]["title"], "b");
    assert_eq!(next[1]["title"], "a");
}

// ---------------------------------------------------------------------------
// Migration on a copy of the real user database
// ---------------------------------------------------------------------------

#[test]
fn migrates_copy_of_real_database() {
    let real_db = match std::env::var_os("HOME") {
        Some(home) => PathBuf::from(home).join(".tam").join("tam.db3"),
        None => return,
    };
    if !real_db.is_file() {
        eprintln!("skipping: no real ~/.tam/tam.db3 on this machine");
        return;
    }

    let home = TempHome::new();
    std::fs::create_dir_all(home.tam_dir()).unwrap();
    let copy = home.tam_dir().join("tam.db3");
    std::fs::copy(&real_db, &copy).expect("failed to copy the user database");
    let config = format!(
        "db_file = \"{}\"\ncurrent_project = \"pastoid\"\nlogfile_path = \"{}/logs/tam.log\"\nlogfile_pattern = \"{}/logs/tam.{{}}.log\"\ntheme = \"dark\"\n",
        copy.display(),
        home.tam_dir().display(),
        home.tam_dir().display()
    );
    std::fs::write(home.tam_dir().join("tam.config.toml"), config).unwrap();

    let (before_version, before_projects, before_tasks) = {
        let conn = rusqlite::Connection::open(&copy).unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        let projects: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM project WHERE is_deleted = 0",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let tasks: i64 = conn
            .query_row("SELECT COUNT(*) FROM task", [], |row| row.get(0))
            .unwrap();
        (version, projects, tasks)
    };

    // First open runs the migration (or is a no-op if already migrated).
    let listed = assert_ok(&run_tam(&home, &["task", "list", "--json"]));
    assert!(
        listed["data"]["count"].as_i64().unwrap_or(0) > 0,
        "migrated copy must still expose the user's tasks"
    );
    let projects = assert_ok(&run_tam(&home, &["project", "list", "--json"]));
    assert_eq!(
        projects["data"].as_array().unwrap().len() as i64,
        before_projects
    );

    if before_version >= 2 {
        // The user database was already migrated by an earlier run.
        return;
    }

    let conn = rusqlite::Connection::open(&copy).unwrap();
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, 2, "user_version must be bumped to 2");
    let index_sql: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='index' AND name='idx_task_idempotency'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(
        index_sql.contains("is_deleted = 0"),
        "migrated idempotency index must exclude soft-deleted rows: {index_sql}"
    );
    let columns: Vec<String> = conn
        .prepare("PRAGMA table_info(task)")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    for expected in [
        "priority",
        "tags",
        "assigned_agent",
        "lease_expires_at",
        "revision",
        "idempotency_key",
    ] {
        assert!(
            columns.iter().any(|column| column == expected),
            "missing migrated column {expected}: {columns:?}"
        );
    }
    let events: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='task_events'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(events, 1);
    let tasks: i64 = conn
        .query_row("SELECT COUNT(*) FROM task", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        tasks, before_tasks,
        "migration must not change the row count"
    );
    drop(conn);

    assert!(
        home.tam_dir().join("tam.db3.bak.v1").is_file(),
        "migration must create tam.db3.bak.v1"
    );

    // Second run is idempotent.
    assert_ok(&run_tam(&home, &["project", "list", "--json"]));
    let conn = rusqlite::Connection::open(&copy).unwrap();
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, 2);
}

// ---------------------------------------------------------------------------
// MCP stdio server
// ---------------------------------------------------------------------------

struct McpClient {
    child: Child,
    stdin: ChildStdin,
    lines: mpsc::Receiver<String>,
}

impl McpClient {
    fn start(home: &TempHome) -> Self {
        let mut child = command_for(home.path())
            .arg("mcp")
            .env("TAM_PROJECT", "alpha")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to spawn tam mcp");
        let stdin = child.stdin.take().expect("missing stdin");
        let stdout = child.stdout.take().expect("missing stdout");
        let (sender, lines) = mpsc::channel();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        if sender.send(line).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            child,
            stdin,
            lines,
        }
    }

    fn send(&mut self, message: Value) {
        writeln!(self.stdin, "{}", message).expect("failed to write MCP request");
        self.stdin.flush().expect("failed to flush MCP request");
    }

    fn read(&mut self) -> Value {
        let line = self
            .lines
            .recv_timeout(Duration::from_secs(10))
            .expect("timed out waiting for an MCP response");
        serde_json::from_str(&line)
            .unwrap_or_else(|error| panic!("non-JSON MCP stdout line {:?}: {}", line, error))
    }

    fn request(&mut self, id: i64, method: &str, params: Value) -> Value {
        self.send(serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        }));
        self.read()
    }

    /// Call one tool and return `(result_object, parsed_text_payload)`.
    fn tool_call(&mut self, id: i64, name: &str, arguments: Value) -> (Value, Value) {
        let response = self.request(
            id,
            "tools/call",
            serde_json::json!({ "name": name, "arguments": arguments }),
        );
        let result = response.get("result").cloned().unwrap_or(Value::Null);
        let text = result
            .get("content")
            .and_then(|content| content.get(0))
            .and_then(|item| item.get("text"))
            .and_then(Value::as_str)
            .unwrap_or("{}");
        let data = serde_json::from_str(text).unwrap_or(Value::Null);
        (result, data)
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn mcp_handshake_and_tools() {
    let home = TempHome::new();
    create_project(&home, "alpha");
    let mut client = McpClient::start(&home);

    let initialize = client.request(1, "initialize", serde_json::json!({}));
    assert_eq!(initialize["jsonrpc"], "2.0");
    assert_eq!(initialize["result"]["protocolVersion"], "2024-11-05");
    assert!(initialize["result"]["capabilities"]["tools"].is_object());

    // Notification: no response. The next reply must belong to id 2.
    client.send(serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized",
    }));
    let tools = client.request(2, "tools/list", serde_json::json!({}));
    assert_eq!(tools["id"], 2);
    let tool_list = tools["result"]["tools"].as_array().unwrap();
    assert_eq!(tool_list.len(), 12);
    let names: Vec<&str> = tool_list
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    for expected in [
        "tam_status",
        "tam_project_list",
        "tam_task_list",
        "tam_task_get",
        "tam_task_create",
        "tam_task_update",
        "tam_task_claim",
        "tam_task_heartbeat",
        "tam_task_release",
        "tam_task_complete",
        "tam_task_next",
        "tam_task_events",
    ] {
        assert!(names.contains(&expected), "missing MCP tool {expected}");
    }

    let created = client.request(
        3,
        "tools/call",
        serde_json::json!({
            "name": "tam_task_create",
            "arguments": { "title": "mcp task", "priority": 1, "tags": ["mcp"] },
        }),
    );
    let text = created["result"]["content"][0]["text"].as_str().unwrap();
    let created_data: Value = serde_json::from_str(text).unwrap();
    assert_eq!(created_data["task"]["title"], "mcp task");
    assert_eq!(created["result"]["isError"], false);

    let listed = client.request(
        4,
        "tools/call",
        serde_json::json!({ "name": "tam_task_list", "arguments": {} }),
    );
    let listed_data: Value =
        serde_json::from_str(listed["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(listed_data["count"], 1);

    let claimed = client.request(
        5,
        "tools/call",
        serde_json::json!({
            "name": "tam_task_claim",
            "arguments": { "id": 1, "agent": "agent-mcp" },
        }),
    );
    let claimed_data: Value =
        serde_json::from_str(claimed["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(claimed_data["task"]["assigned_agent"], "agent-mcp");

    let done = client.request(
        6,
        "tools/call",
        serde_json::json!({
            "name": "tam_task_complete",
            "arguments": { "id": 1, "agent": "agent-mcp" },
        }),
    );
    let done_data: Value =
        serde_json::from_str(done["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(done_data["task"]["status"], "done");

    // Business errors surface as isError, not JSON-RPC errors.
    let conflict = client.request(
        7,
        "tools/call",
        serde_json::json!({
            "name": "tam_task_claim",
            "arguments": { "id": 1, "agent": "agent-other" },
        }),
    );
    assert_eq!(conflict["result"]["isError"], true);
    let conflict_body: Value =
        serde_json::from_str(conflict["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(conflict_body["code"], "conflict");

    // Unknown method: JSON-RPC error -32601.
    let unknown = client.request(8, "unknown/method", serde_json::json!({}));
    assert_eq!(unknown["error"]["code"], -32601);

    // Every stdout line parsed as JSON above (read() enforces purity).
}

#[test]
fn mcp_unknown_tool_is_iserror_and_missing_params_are_protocol_errors() {
    let home = TempHome::new();
    create_project(&home, "alpha");
    let mut client = McpClient::start(&home);
    assert_eq!(
        client.request(1, "initialize", serde_json::json!({}))["result"]["protocolVersion"],
        "2024-11-05"
    );

    let unknown_tool = client.request(
        2,
        "tools/call",
        serde_json::json!({ "name": "tam_not_a_tool", "arguments": {} }),
    );
    assert_eq!(unknown_tool["result"]["isError"], true);

    let missing_name = client.request(3, "tools/call", serde_json::json!({}));
    assert_eq!(missing_name["error"]["code"], -32602);

    // Invalid JSON gets a parse error envelope with id null.
    writeln!(client.stdin, "this is not json").unwrap();
    client.stdin.flush().unwrap();
    let parse_error = client.read();
    assert_eq!(parse_error["error"]["code"], -32700);
    assert!(parse_error["id"].is_null());
}

// ---------------------------------------------------------------------------
// Gate B review fixes: D1-D5 and P3 polish
// ---------------------------------------------------------------------------

#[test]
fn d1_idempotency_key_reusable_after_soft_delete() {
    let home = TempHome::new();
    create_project(&home, "alpha");

    let first = assert_ok(&run_tam(
        &home,
        &[
            "task",
            "add",
            "keyed",
            "--project",
            "alpha",
            "--idempotency-key",
            "reuse-1",
            "--json",
        ],
    ));
    let first_id = first["data"]["task"]["id"].as_i64().unwrap();

    // Delete the task (soft delete) and retry with the same key.
    let deleted = run_tam(&home, &["task", "del", &first_id.to_string()]);
    assert_eq!(deleted.status.code(), Some(0));

    let second = assert_ok(&run_tam(
        &home,
        &[
            "task",
            "add",
            "keyed again",
            "--project",
            "alpha",
            "--idempotency-key",
            "reuse-1",
            "--json",
        ],
    ));
    let second_id = second["data"]["task"]["id"].as_i64().unwrap();
    assert_ne!(second_id, first_id, "a new task must be created");
    assert_eq!(
        second["data"]["deduplicated"], false,
        "reusing a key after soft deletion is a fresh create"
    );

    // The new active row still deduplicates on retry.
    let third = assert_ok(&run_tam(
        &home,
        &[
            "task",
            "add",
            "keyed again",
            "--project",
            "alpha",
            "--idempotency-key",
            "reuse-1",
            "--json",
        ],
    ));
    assert_eq!(third["data"]["deduplicated"], true);
    assert_eq!(third["data"]["task"]["id"], second_id);
}

#[test]
fn d2_update_done_respects_lease_and_emits_completed_event() {
    let home = TempHome::new();
    create_project(&home, "alpha");
    add_task(&home, "alpha", "guarded update");
    assert_ok(&run_tam(
        &home,
        &[
            "task",
            "claim",
            "1",
            "--project",
            "alpha",
            "--agent",
            "agent-a",
            "--json",
            "--lease",
            "600",
        ],
    ));

    // Another agent cannot complete via update either.
    let conflict = run_tam(
        &home,
        &[
            "task",
            "update",
            "1",
            "--project",
            "alpha",
            "--status",
            "done",
            "--agent",
            "agent-b",
            "--json",
        ],
    );
    assert_error(&conflict, 4, "conflict");

    let shown = assert_ok(&run_tam(&home, &["task", "show", "1", "--json"]));
    assert_eq!(shown["data"]["task"]["status"], "in-progress");
    assert_eq!(shown["data"]["task"]["assigned_agent"], "agent-a");
    assert_eq!(
        shown["data"]["task"]["revision"], 2,
        "conflict must not write"
    );

    // The holder (or a human) can complete; the event must be `completed`.
    let done = assert_ok(&run_tam(
        &home,
        &[
            "task",
            "update",
            "1",
            "--project",
            "alpha",
            "--status",
            "done",
            "--agent",
            "agent-a",
            "--json",
        ],
    ));
    assert_eq!(done["data"]["task"]["status"], "done");
    assert_eq!(done["data"]["task"]["assigned_agent"], Value::Null);
    assert_eq!(done["data"]["task"]["revision"], 3);

    let events = assert_ok(&run_tam(&home, &["task", "events", "1", "--json"]));
    let last = events["data"]["events"].as_array().unwrap().last().unwrap();
    assert_eq!(last["event"], "completed");
    assert_eq!(last["to_status"], "done");
    assert_eq!(last["actor"], "agent-a");
}

#[test]
fn d3_update_todo_clears_lease_and_next_returns_task() {
    let home = TempHome::new();
    create_project(&home, "alpha");
    add_task(&home, "alpha", "todo transition");
    assert_ok(&run_tam(
        &home,
        &[
            "task",
            "claim",
            "1",
            "--project",
            "alpha",
            "--agent",
            "agent-a",
            "--json",
        ],
    ));

    let updated = assert_ok(&run_tam(
        &home,
        &[
            "task",
            "update",
            "1",
            "--project",
            "alpha",
            "--status",
            "todo",
            "--json",
        ],
    ));
    assert_eq!(updated["data"]["task"]["status"], "todo");
    assert_eq!(updated["data"]["task"]["assigned_agent"], Value::Null);
    assert_eq!(updated["data"]["task"]["lease_expires_at"], Value::Null);

    // The task becomes visible to `next` again immediately.
    let next = assert_ok(&run_tam(
        &home,
        &["task", "next", "--project", "alpha", "--json"],
    ));
    assert_eq!(
        next["data"]["task"]["id"], 1,
        "todo task must be claimable again"
    );
}

#[test]
fn agent_b_cannot_update_task_leased_by_agent_a() {
    let home = TempHome::new();
    create_project(&home, "alpha");
    add_task(&home, "alpha", "leased update");
    assert_ok(&run_tam(
        &home,
        &[
            "task",
            "claim",
            "1",
            "--project",
            "alpha",
            "--agent",
            "agent-a",
            "--lease",
            "600",
            "--json",
        ],
    ));

    // Any agent write to a task under another agent's valid lease conflicts:
    // title-only updates and status changes alike.
    for args in [
        vec![
            "task",
            "update",
            "1",
            "--project",
            "alpha",
            "--title",
            "stolen",
            "--agent",
            "agent-b",
            "--json",
        ],
        vec![
            "task",
            "update",
            "1",
            "--project",
            "alpha",
            "--status",
            "todo",
            "--agent",
            "agent-b",
            "--json",
        ],
    ] {
        let output = run_tam(&home, &args);
        assert_error(&output, 4, "conflict");

        let shown = assert_ok(&run_tam(&home, &["task", "show", "1", "--json"]));
        let task = &shown["data"]["task"];
        assert_eq!(
            task["title"], "leased update",
            "conflict must not write the title"
        );
        assert_eq!(task["status"], "in-progress");
        assert_eq!(task["assigned_agent"], "agent-a");
        assert!(task["lease_expires_at"].as_str().is_some());
        assert_eq!(task["revision"], 2, "conflict must not bump the revision");
    }

    // The lease holder may still update its own task.
    let holder = assert_ok(&run_tam(
        &home,
        &[
            "task",
            "update",
            "1",
            "--project",
            "alpha",
            "--title",
            "holder edit",
            "--agent",
            "agent-a",
            "--json",
        ],
    ));
    assert_eq!(holder["data"]["task"]["title"], "holder edit");
    assert_eq!(holder["data"]["task"]["revision"], 3);

    // Humans (no --agent) may still update and clear the lease.
    let human = assert_ok(&run_tam(
        &home,
        &[
            "task",
            "update",
            "1",
            "--project",
            "alpha",
            "--status",
            "todo",
            "--json",
        ],
    ));
    assert_eq!(human["data"]["task"]["status"], "todo");
    assert_eq!(human["data"]["task"]["assigned_agent"], Value::Null);
    assert_eq!(human["data"]["task"]["lease_expires_at"], Value::Null);
}

#[test]
fn d4_mcp_update_can_clear_tags() {
    let home = TempHome::new();
    create_project(&home, "alpha");
    let created = assert_ok(&run_tam(
        &home,
        &[
            "task",
            "add",
            "tagged",
            "--project",
            "alpha",
            "--tag",
            "x",
            "--json",
        ],
    ));
    let id = created["data"]["task"]["id"].as_i64().unwrap();

    let mut client = McpClient::start(&home);
    let init = client.request(1, "initialize", serde_json::json!({}));
    assert_eq!(init["result"]["protocolVersion"], "2024-11-05");

    // Explicit empty array is a valid field and clears the tags.
    let (result, data) = client.tool_call(
        2,
        "tam_task_update",
        serde_json::json!({ "id": id, "tags": [] }),
    );
    assert_eq!(result["isError"], false);
    assert_eq!(data["task"]["tags"], serde_json::json!([]));

    // Re-add tags, then clear with an empty string.
    let (result, data) = client.tool_call(
        3,
        "tam_task_update",
        serde_json::json!({ "id": id, "tags": ["y", "z"] }),
    );
    assert_eq!(result["isError"], false);
    assert_eq!(data["task"]["tags"], serde_json::json!(["y", "z"]));

    let (result, data) = client.tool_call(
        4,
        "tam_task_update",
        serde_json::json!({ "id": id, "tags": "" }),
    );
    assert_eq!(result["isError"], false);
    assert_eq!(data["task"]["tags"], serde_json::json!([]));

    // Missing tags still means "do not change".
    client.tool_call(
        5,
        "tam_task_update",
        serde_json::json!({ "id": id, "title": "tagged v2" }),
    );
    let (_, data) = client.tool_call(6, "tam_task_get", serde_json::json!({ "id": id }));
    assert_eq!(data["task"]["tags"], serde_json::json!([]));
}

#[test]
fn d5_broken_config_json_uses_error_envelope() {
    let home = TempHome::new();
    std::fs::create_dir_all(home.tam_dir()).unwrap();
    std::fs::write(home.tam_dir().join("tam.config.toml"), "this is not toml\n").unwrap();

    let output = run_tam(&home, &["project", "list", "--json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        output.stdout.is_empty(),
        "startup error must not write to stdout: {:?}",
        String::from_utf8_lossy(&output.stdout)
    );
    let value = json_stderr(&output);
    assert_eq!(value["ok"], false);
    assert_eq!(value["schema"], "tam.agent/v1");
    assert_eq!(value["command"], "startup");
    assert_eq!(value["error"]["code"], "internal");
}

#[test]
fn p3_project_view_timestamps_are_populated() {
    let home = TempHome::new();

    let added = assert_ok(&run_tam(&home, &["project", "add", "alpha", "--json"]));
    assert!(
        added["data"]["project"]["created"].as_str().is_some(),
        "project add must return created/updated timestamps: {}",
        added["data"]["project"]
    );
    assert!(added["data"]["project"]["updated"].as_str().is_some());

    let list = assert_ok(&run_tam(&home, &["project", "list", "--json"]));
    assert!(list["data"][0]["created"].as_str().is_some());
    assert!(list["data"][0]["updated"].as_str().is_some());

    assert_eq!(
        run_tam(&home, &["project", "use", "alpha"]).status.code(),
        Some(0)
    );
    let current = assert_ok(&run_tam(&home, &["project", "current", "--json"]));
    assert!(
        current["data"]["project"]["created"].as_str().is_some(),
        "project current must return timestamps: {}",
        current["data"]["project"]
    );
}

#[test]
fn p3_by_id_meta_project_is_populated() {
    let home = TempHome::new();
    create_project(&home, "alpha");
    add_task(&home, "alpha", "meta task");
    // Use config current_project (no --project / TAM_PROJECT) so the envelope
    // meta.project can only come from the loaded task itself.
    assert_eq!(
        run_tam(&home, &["project", "use", "alpha"]).status.code(),
        Some(0)
    );

    let shown = assert_ok(&run_tam(&home, &["task", "show", "1", "--json"]));
    assert_eq!(shown["meta"]["project"], "alpha");
    let events = assert_ok(&run_tam(&home, &["task", "events", "1", "--json"]));
    assert_eq!(events["meta"]["project"], "alpha");
    let claimed = assert_ok(&run_tam(
        &home,
        &["task", "claim", "1", "--agent", "agent-a", "--json"],
    ));
    assert_eq!(claimed["meta"]["project"], "alpha");
    let updated = assert_ok(&run_tam(
        &home,
        &["task", "update", "1", "--title", "meta task v2", "--json"],
    ));
    assert_eq!(updated["meta"]["project"], "alpha");
    let next = assert_ok(&run_tam(&home, &["task", "next", "--json"]));
    assert_eq!(next["meta"]["project"], "alpha");
    let done = assert_ok(&run_tam(&home, &["task", "done", "1", "--json"]));
    assert_eq!(done["meta"]["project"], "alpha");
}

#[test]
fn p3_task_delete_json_writes_event() {
    let home = TempHome::new();
    create_project(&home, "alpha");
    add_task(&home, "alpha", "delete me");

    let deleted = assert_ok(&run_tam(&home, &["task", "del", "1", "--json"]));
    assert_eq!(deleted["command"], "task.del");
    assert_eq!(deleted["data"]["task"]["id"], 1);

    let gone = run_tam(&home, &["task", "show", "1", "--json"]);
    assert_error(&gone, 3, "not_found");

    let conn = rusqlite::Connection::open(home.tam_dir().join("tam.db3")).unwrap();
    let deleted_events: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM task_events WHERE task_id = 1 AND event = 'deleted'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(deleted_events, 1, "soft delete must append a deleted event");
}

// ---------------------------------------------------------------------------
// Human mode compatibility
// ---------------------------------------------------------------------------

#[test]
fn human_output_is_not_json() {
    let home = TempHome::new();
    create_project(&home, "alpha");
    assert_eq!(
        run_tam(&home, &["project", "use", "alpha"]).status.code(),
        Some(0)
    );
    let task = run_tam(&home, &["task", "add", "human task"]);
    assert_eq!(task.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&task.stdout);
    assert!(stdout.contains("SUCCESS"));
    assert!(!stdout.contains("tam.agent/v1"));

    let list = run_tam(&home, &["task", "list"]);
    assert_eq!(list.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&list.stdout);
    assert!(stdout.contains("human task"));
    assert!(!stdout.contains("\"ok\""));
}
