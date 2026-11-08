//! Integration tests for the `tam` CLI.
//!
//! Every test runs the compiled binary with its own temporary `HOME` (and
//! `USERPROFILE` for portability), so the real user data under `~/.tam` is
//! never read or written. Only `std` and `CARGO_BIN_EXE_tam` are used: no
//! dev-dependencies were added for these tests.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_HOME: AtomicU32 = AtomicU32::new(0);

/// A unique temporary directory injected as the child process HOME.
struct TempHome {
    path: PathBuf,
}

impl TempHome {
    fn new() -> Self {
        let unique = format!(
            "tam-cli-test-{}-{}-{}",
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

/// Run the compiled binary with a clear environment and the given temp HOME.
fn run_tam(home: &TempHome, args: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tam"));
    command
        .args(args)
        .env_clear()
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env("TZ", "UTC");
    if let Some(path) = std::env::var_os("PATH") {
        command.env("PATH", path);
    }
    command.output().expect("failed to spawn tam binary")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[track_caller]
fn assert_code(output: &Output, expected: i32) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "unexpected exit code\nstdout:\n{}\nstderr:\n{}",
        stdout(output),
        stderr(output)
    );
}

// ---------------------------------------------------------------------------
// G10: hermetic harness. The binary must only write under the injected HOME.
// ---------------------------------------------------------------------------

#[test]
fn binary_writes_only_under_injected_home() {
    let home = TempHome::new();
    let output = run_tam(&home, &["project", "list"]);
    assert_code(&output, 0);

    let tam_dir = home.tam_dir();
    assert!(
        tam_dir.join("tam.config.toml").is_file(),
        "config was not created under the injected HOME: {}",
        tam_dir.display()
    );
    assert!(
        tam_dir.join("tam.db3").is_file(),
        "database was not created under the injected HOME: {}",
        tam_dir.display()
    );

    let config = std::fs::read_to_string(tam_dir.join("tam.config.toml"))
        .expect("failed to read generated config");
    let home_path = home.path().display().to_string();
    assert!(
        config.contains(&home_path),
        "generated config does not point at the injected HOME {home_path:?}:\n{config}"
    );
}

// ---------------------------------------------------------------------------
// Shared helpers for multi-command scenarios.
// ---------------------------------------------------------------------------

#[track_caller]
fn assert_stdout_empty(output: &Output) {
    let text = stdout(output);
    assert!(
        text.trim().is_empty(),
        "expected empty stdout, got:\n{text}\nstderr:\n{}",
        stderr(output)
    );
}

#[track_caller]
fn assert_stdout_contains(output: &Output, needle: &str) {
    let text = stdout(output);
    assert!(
        text.contains(needle),
        "stdout does not contain {needle:?}\nstdout:\n{text}\nstderr:\n{}",
        stderr(output)
    );
}

#[track_caller]
fn assert_stderr_contains(output: &Output, needle: &str) {
    let text = stderr(output);
    assert!(
        text.contains(needle),
        "stderr does not contain {needle:?}\nstdout:\n{}\nstderr:\n{text}",
        stdout(output)
    );
}

#[track_caller]
fn add_project(home: &TempHome, name: &str) {
    let output = run_tam(home, &["project", "add", name]);
    assert_code(&output, 0);
}

#[track_caller]
fn use_project(home: &TempHome, name: &str) {
    let output = run_tam(home, &["project", "use", name]);
    assert_code(&output, 0);
}

#[track_caller]
fn add_task(home: &TempHome, title: &str) {
    let output = run_tam(home, &["task", "add", title]);
    assert_code(&output, 0);
}

// ---------------------------------------------------------------------------
// G5: not-found messages, id parsing and error stream (stderr).
// ---------------------------------------------------------------------------

#[test]
fn task_not_found_message() {
    let home = TempHome::new();
    add_project(&home, "alpha");
    use_project(&home, "alpha");

    let output = run_tam(&home, &["task", "done", "999"]);
    assert_code(&output, 1);
    assert_stdout_empty(&output);
    assert_stderr_contains(&output, "task 999 not found");
    assert!(
        !stderr(&output).contains("Query returned no rows"),
        "raw SQL error leaked to stderr:\n{}",
        stderr(&output)
    );
}

#[test]
fn project_not_found_message() {
    let home = TempHome::new();

    let output = run_tam(&home, &["project", "use", "nope"]);
    assert_code(&output, 1);
    assert_stdout_empty(&output);
    assert_stderr_contains(&output, "project 'nope' not found");
    assert!(
        !stderr(&output).contains("Query returned no rows"),
        "raw SQL error leaked to stderr:\n{}",
        stderr(&output)
    );

    let output = run_tam(&home, &["project", "del", "nope"]);
    assert_code(&output, 1);
    assert_stderr_contains(&output, "project 'nope' not found");
}

#[test]
fn non_numeric_task_id_usage_error() {
    let home = TempHome::new();
    add_project(&home, "alpha");
    use_project(&home, "alpha");

    for args in [
        vec!["task", "done", "abc"],
        vec!["task", "done", "0"],
        vec!["task", "done", "-1"],
        vec!["task", "edit", "abc", "new title"],
    ] {
        let output = run_tam(&home, &args);
        assert_code(&output, 2);
        assert!(
            !stderr(&output).contains("Query returned no rows"),
            "clap should reject {args:?} before touching the database, stderr:\n{}",
            stderr(&output)
        );
    }
}

#[test]
fn errors_use_stderr() {
    let home = TempHome::new();
    add_project(&home, "alpha");

    // Business error: exit 1, message on stderr, nothing on stdout.
    let output = run_tam(&home, &["project", "use", "nope"]);
    assert_code(&output, 1);
    assert_stdout_empty(&output);
    assert_stderr_contains(&output, "project 'nope' not found");

    // Success path: output on stdout, stderr quiet.
    let output = run_tam(&home, &["project", "add", "beta"]);
    assert_code(&output, 0);
    assert_stdout_contains(&output, "SUCCESS");
    assert!(
        stderr(&output).trim().is_empty(),
        "success path wrote to stderr:\n{}",
        stderr(&output)
    );
}

// ---------------------------------------------------------------------------
// G7: blank names are rejected before touching the database.
// ---------------------------------------------------------------------------

#[test]
fn reject_blank_project_name() {
    let home = TempHome::new();

    for name in ["", "   "] {
        let output = run_tam(&home, &["project", "add", name]);
        assert_code(&output, 1);
        assert_stderr_contains(&output, "cannot be empty");
        assert_stdout_empty(&output);
    }

    let output = run_tam(&home, &["project", "list"]);
    assert_code(&output, 0);
    assert_stdout_contains(&output, "No projects found");
}

#[test]
fn reject_blank_task_title() {
    let home = TempHome::new();
    add_project(&home, "alpha");
    use_project(&home, "alpha");

    for title in ["", "   "] {
        let output = run_tam(&home, &["task", "add", title]);
        assert_code(&output, 1);
        assert_stderr_contains(&output, "cannot be empty");
        assert_stdout_empty(&output);
    }

    let output = run_tam(&home, &["task", "list"]);
    assert_code(&output, 0);
    assert_stdout_contains(&output, "No tasks found");
}

// ---------------------------------------------------------------------------
// Table helpers: extract the cells of the first rendered row containing a
// needle. `tam` renders tables with the `tabled` rounded style ("│ cell │").
// ---------------------------------------------------------------------------

#[track_caller]
fn table_row_cells(text: &str, needle: &str) -> Vec<String> {
    for line in text.lines() {
        if line.contains(needle) {
            let cells: Vec<String> = line
                .split('│')
                .map(|cell| cell.trim().to_string())
                .filter(|cell| !cell.is_empty())
                .collect();
            if !cells.is_empty() {
                return cells;
            }
        }
    }
    panic!("no table row containing {needle:?} in:\n{text}");
}

// ---------------------------------------------------------------------------
// G1: project list must read the `updated` column, not `created`.
// ---------------------------------------------------------------------------

#[test]
fn project_list_updated_column() {
    let home = TempHome::new();
    add_project(&home, "alpha");

    // SQLite timestamps have a one-second resolution: wait so created and
    // updated differ even after a single edit.
    std::thread::sleep(std::time::Duration::from_millis(1100));

    let output = run_tam(&home, &["project", "edit", "alpha", "alpha-renamed"]);
    assert_code(&output, 0);

    let output = run_tam(&home, &["project", "list"]);
    assert_code(&output, 0);
    let cells = table_row_cells(&stdout(&output), "alpha-renamed");
    assert!(
        cells.len() >= 3,
        "unexpected project list row: {cells:?}\n{}",
        stdout(&output)
    );
    assert_ne!(
        cells[1], cells[2],
        "Updated must differ from Created after a rename; row={cells:?}"
    );
}

// ---------------------------------------------------------------------------
// G2: rename writes the config only after the database update succeeded.
// ---------------------------------------------------------------------------

#[test]
fn rename_failure_keeps_current_project() {
    let home = TempHome::new();
    add_project(&home, "alpha");
    add_project(&home, "beta");
    use_project(&home, "alpha");

    let list_before = run_tam(&home, &["project", "list"]);
    assert_code(&list_before, 0);

    let output = run_tam(&home, &["project", "edit", "alpha", "beta"]);
    assert_code(&output, 1);
    assert_stderr_contains(&output, "already exists");

    let current = run_tam(&home, &["project", "current"]);
    assert_code(&current, 0);
    assert_stdout_contains(&current, "alpha");

    let list_after = run_tam(&home, &["project", "list"]);
    assert_code(&list_after, 0);
    assert_eq!(
        stdout(&list_before),
        stdout(&list_after),
        "failed rename must not change the project list"
    );
}

#[test]
fn rename_success_updates_current_project() {
    let home = TempHome::new();
    add_project(&home, "alpha");
    use_project(&home, "alpha");

    let output = run_tam(&home, &["project", "edit", "alpha", "gamma"]);
    assert_code(&output, 0);

    let current = run_tam(&home, &["project", "current"]);
    assert_code(&current, 0);
    assert_stdout_contains(&current, "gamma");

    let list = run_tam(&home, &["project", "list"]);
    assert_stdout_contains(&list, "gamma");
    assert!(
        !stdout(&list).contains("alpha"),
        "old project name still listed:\n{}",
        stdout(&list)
    );
}

#[test]
fn rename_to_soft_deleted_name_errors() {
    let home = TempHome::new();
    add_project(&home, "alpha");
    let output = run_tam(&home, &["project", "del", "alpha"]);
    assert_code(&output, 0);

    add_project(&home, "beta");
    let output = run_tam(&home, &["project", "edit", "beta", "alpha"]);
    assert_code(&output, 1);
    assert_stderr_contains(&output, "deleted project");
    assert_stderr_contains(&output, "tam project add alpha");

    let list = run_tam(&home, &["project", "list"]);
    assert_stdout_contains(&list, "beta");
    assert!(
        !stdout(&list).contains("alpha"),
        "soft-deleted project name must not be listed:\n{}",
        stdout(&list)
    );
}

// ---------------------------------------------------------------------------
// G3: deletes are refused while active tasks exist; deleting the current
// project clears `current_project` only after the database delete succeeded.
// ---------------------------------------------------------------------------

#[test]
fn delete_refuses_project_with_active_tasks() {
    let home = TempHome::new();
    add_project(&home, "alpha");
    use_project(&home, "alpha");
    add_task(&home, "first task");
    add_task(&home, "second task");

    let output = run_tam(&home, &["project", "del", "alpha"]);
    assert_code(&output, 1);
    assert_stderr_contains(&output, "project 'alpha' has 2 active task(s)");
    assert_stdout_empty(&output);

    // Nothing was deleted: the project and both tasks are still listed.
    let list = run_tam(&home, &["project", "list"]);
    assert_stdout_contains(&list, "alpha");
    let tasks = run_tam(&home, &["task", "list"]);
    assert_stdout_contains(&tasks, "first task");
    assert_stdout_contains(&tasks, "second task");
}

#[test]
fn delete_current_project_clears_config() {
    let home = TempHome::new();
    add_project(&home, "alpha");
    use_project(&home, "alpha");

    let output = run_tam(&home, &["project", "del", "alpha"]);
    assert_code(&output, 0);
    assert_stdout_contains(&output, "deleted successfully");

    let current = run_tam(&home, &["project", "current"]);
    assert_code(&current, 1);
    assert_stderr_contains(&current, "No current project set");

    let list = run_tam(&home, &["project", "list"]);
    assert_stdout_contains(&list, "No projects found");
}

#[test]
fn delete_missing_project_not_found() {
    let home = TempHome::new();
    add_project(&home, "alpha");

    let output = run_tam(&home, &["project", "del", "nope"]);
    assert_code(&output, 1);
    assert_stderr_contains(&output, "project 'nope' not found");
    assert_stdout_empty(&output);
}

#[test]
fn delete_allowed_when_only_soft_deleted_tasks() {
    let home = TempHome::new();
    add_project(&home, "alpha");
    use_project(&home, "alpha");
    add_task(&home, "only task");

    let output = run_tam(&home, &["task", "del", "1"]);
    assert_code(&output, 0);

    // Non-current project delete still prints a SUCCESS line.
    let output = run_tam(&home, &["project", "del", "alpha"]);
    assert_code(&output, 0);
    assert_stdout_contains(&output, "deleted successfully");

    let list = run_tam(&home, &["project", "list"]);
    assert_stdout_contains(&list, "No projects found");
}

// ---------------------------------------------------------------------------
// G4: re-adding a soft-deleted project name restores it; active duplicates
// keep failing with `already exists`.
// ---------------------------------------------------------------------------

#[test]
fn add_restores_soft_deleted_project() {
    let home = TempHome::new();
    add_project(&home, "beta");

    // Let the trigger-visible timestamp move past the create time.
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let output = run_tam(&home, &["project", "del", "beta"]);
    assert_code(&output, 0);

    let output = run_tam(&home, &["project", "add", "beta"]);
    assert_code(&output, 0);
    assert_stdout_contains(&output, "restored");

    let list = run_tam(&home, &["project", "list"]);
    assert_stdout_contains(&list, "beta");
    let cells = table_row_cells(&stdout(&list), "beta");
    assert_ne!(
        cells[1], cells[2],
        "restoring must refresh the trigger-maintained updated column; row={cells:?}"
    );

    // Restore never switches the current project.
    let current = run_tam(&home, &["project", "current"]);
    assert_code(&current, 1);
    assert_stderr_contains(&current, "No current project set");
}

#[test]
fn add_active_duplicate_already_exists() {
    let home = TempHome::new();
    add_project(&home, "beta");

    let output = run_tam(&home, &["project", "add", "beta"]);
    assert_code(&output, 1);
    assert_stderr_contains(&output, "already exists");
    assert_stdout_empty(&output);

    let list = run_tam(&home, &["project", "list"]);
    assert_stdout_contains(&list, "beta");
}

// ---------------------------------------------------------------------------
// G9: `task list --status <todo|in-progress|done>`.
// ---------------------------------------------------------------------------

/// Project `alpha` with one task per status; returns the task titles.
fn seed_tasks_with_all_statuses(home: &TempHome) {
    add_project(home, "alpha");
    use_project(home, "alpha");
    add_task(home, "task-todo");
    add_task(home, "task-progress");
    add_task(home, "task-done");

    let output = run_tam(home, &["task", "start", "2"]);
    assert_code(&output, 0);
    let output = run_tam(home, &["task", "done", "3"]);
    assert_code(&output, 0);
}

#[test]
fn task_list_status_filter() {
    let home = TempHome::new();
    seed_tasks_with_all_statuses(&home);

    let output = run_tam(&home, &["task", "list", "--status", "done"]);
    assert_code(&output, 0);
    assert_stdout_contains(&output, "task-done");
    assert!(
        !stdout(&output).contains("task-todo") && !stdout(&output).contains("task-progress"),
        "done filter leaked other statuses:\n{}",
        stdout(&output)
    );

    let output = run_tam(&home, &["task", "list", "--status", "todo"]);
    assert_code(&output, 0);
    assert_stdout_contains(&output, "task-todo");
    assert!(
        !stdout(&output).contains("task-done"),
        "{}",
        stdout(&output)
    );

    let output = run_tam(&home, &["task", "list", "--status", "in-progress"]);
    assert_code(&output, 0);
    assert_stdout_contains(&output, "task-progress");
    assert!(
        !stdout(&output).contains("task-todo"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn task_list_without_status_unchanged() {
    let home = TempHome::new();
    seed_tasks_with_all_statuses(&home);

    let output = run_tam(&home, &["task", "list"]);
    assert_code(&output, 0);
    let text = stdout(&output);
    for title in ["task-todo", "task-progress", "task-done"] {
        assert_eq!(
            text.matches(title).count(),
            1,
            "task {title:?} should be listed exactly once:\n{text}"
        );
    }
    assert_stdout_contains(&output, "Current project: alpha");

    // The no-argument output must not depend on any filter state.
    let again = run_tam(&home, &["task", "list"]);
    assert_eq!(stdout(&output), stdout(&again));
}

#[test]
fn invalid_status_value_usage_error() {
    let home = TempHome::new();
    seed_tasks_with_all_statuses(&home);

    let output = run_tam(&home, &["task", "list", "--status", "wip"]);
    assert_code(&output, 2);
    assert_stderr_contains(&output, "invalid value");

    let output = run_tam(&home, &["task", "list", "--status"]);
    assert_code(&output, 2);
    assert!(
        !stderr(&output).trim().is_empty(),
        "missing --status value must produce a clap usage error"
    );
}

// ---------------------------------------------------------------------------
// S9 / Lead review: blank edit targets and whitespace normalization.
// ---------------------------------------------------------------------------

#[test]
fn project_edit_blank_name_rejected() {
    let home = TempHome::new();
    add_project(&home, "alpha");
    use_project(&home, "alpha");

    for blank in ["", "   "] {
        let output = run_tam(&home, &["project", "edit", "alpha", blank]);
        assert_code(&output, 1);
        assert_stderr_contains(&output, "cannot be empty");
        assert_stdout_empty(&output);
    }

    // Project row, config and task target are untouched.
    let current = run_tam(&home, &["project", "current"]);
    assert_code(&current, 0);
    assert_stdout_contains(&current, "alpha");

    let list = run_tam(&home, &["project", "list"]);
    assert_code(&list, 0);
    let cells = table_row_cells(&stdout(&list), "alpha");
    assert_eq!(
        cells.len(),
        3,
        "expected exactly one non-empty project row, got: {cells:?}"
    );

    let added = run_tam(&home, &["task", "add", "still works"]);
    assert_code(&added, 0);
    let tasks = run_tam(&home, &["task", "list"]);
    assert_stdout_contains(&tasks, "still works");
}

#[test]
fn task_edit_blank_title_rejected() {
    let home = TempHome::new();
    add_project(&home, "alpha");
    use_project(&home, "alpha");
    add_task(&home, "original title");

    for blank in ["", "   "] {
        let output = run_tam(&home, &["task", "edit", "1", blank]);
        assert_code(&output, 1);
        assert_stderr_contains(&output, "cannot be empty");
        assert_stdout_empty(&output);
    }

    let list = run_tam(&home, &["task", "list"]);
    assert_code(&list, 0);
    let cells = table_row_cells(&stdout(&list), "original title");
    assert_eq!(cells.len(), 3, "task title must be unchanged: {cells:?}");
}

#[test]
fn project_names_trimmed_consistently() {
    let home = TempHome::new();

    // add stores the canonical name.
    let output = run_tam(&home, &["project", "add", "  alpha  "]);
    assert_code(&output, 0);
    let list = run_tam(&home, &["project", "list"]);
    assert_stdout_contains(&list, "alpha");
    assert!(
        !stdout(&list).contains("  alpha  "),
        "padded name leaked into the list:\n{}",
        stdout(&list)
    );

    // use tolerates padded input and stores the canonical name.
    let output = run_tam(&home, &["project", "use", " alpha "]);
    assert_code(&output, 0);
    let current = run_tam(&home, &["project", "current"]);
    assert_code(&current, 0);
    assert_stdout_contains(&current, "alpha");

    // edit tolerates padded source/target and stores the canonical name.
    let output = run_tam(&home, &["project", "edit", " alpha ", " beta "]);
    assert_code(&output, 0);
    let current = run_tam(&home, &["project", "current"]);
    assert_code(&current, 0);
    assert_stdout_contains(&current, "beta");
    assert!(
        !stdout(&current).contains(" beta "),
        "current project leaked padding:\n{}",
        stdout(&current)
    );
    let config = std::fs::read_to_string(home.tam_dir().join("tam.config.toml"))
        .expect("failed to read generated config");
    assert!(
        config.contains("current_project = \"beta\""),
        "config must store the canonical name:\n{config}"
    );

    // delete tolerates padded input and clears the canonical current name.
    let output = run_tam(&home, &["project", "del", " beta "]);
    assert_code(&output, 0);
    let current = run_tam(&home, &["project", "current"]);
    assert_code(&current, 1);
    assert_stderr_contains(&current, "No current project set");
}
