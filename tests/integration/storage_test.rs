use chrono::{Utc, Duration};
use smol::core::{TaskId, TaskMeta, TaskStatus, TaskMode, SmolError};
use smol::storage::{self, task_store, sqlite};
use tempfile::TempDir;

/// Helper to create test metadata.
fn test_meta(id: &str, command: &str) -> TaskMeta {
    TaskMeta {
        id: TaskId::from_raw(id.to_string()),
        command: command.to_string(),
        mode: TaskMode::Sync,
        created_at: Utc::now(),
        completed_at: Some(Utc::now()),
        exit_code: Some(0),
        duration_sec: Some(1),
        status: TaskStatus::Success,
        total_lines: 5,
        total_chars: 100,
        output_size_bytes: 100,
        error_count: 0,
        warning_count: 0,
        pid: None,
        background_pid: None,
        input_tokens: Some(25),
        output_tokens: Some(5),
        compression_ratio: Some(0.2),
        test_total: None,
        test_passed: None,
        test_failed: None,
        test_errors: None,
        test_skipped: None,
    }
}

/// Helper to manually save a task's meta and registry entry to a specific directory.
/// (We cannot use task_store::save_task because it writes to HOME/.smol/tasks.)
fn save_task_to_dir(meta: &TaskMeta, tasks_dir: &str) {
    // Create task directory
    let task_dir = std::path::Path::new(tasks_dir).join(meta.id.as_str());
    std::fs::create_dir_all(&task_dir).unwrap();

    // Write meta.toml
    let meta_content = toml::to_string_pretty(meta).unwrap();
    std::fs::write(task_dir.join("meta.toml"), meta_content).unwrap();

    // Update registry
    use smol::storage::registry;
    let mut reg = registry::load_registry(tasks_dir).unwrap_or_default();
    if !reg.tasks.iter().any(|e| e.id == meta.id) {
        reg.tasks.push(smol::storage::registry::RegistryEntry {
            id: meta.id.clone(),
            created_at: meta.created_at,
            status: meta.status,
        });
        registry::save_registry(tasks_dir, &reg).unwrap();
    }
}

// ── TOML Storage Tests ──────────────────────────────────────────

/// Test saving and loading a task meta via TOML storage.
#[test]
fn test_toml_save_and_load() {
    let temp = TempDir::new().unwrap();
    storage::init(temp.path().to_str().unwrap()).unwrap();
    let meta = test_meta("Tst12345", "echo hello");

    save_task_to_dir(&meta, temp.path().to_str().unwrap());

    // Load
    let loaded = task_store::load_task_meta(&meta.id, temp.path().to_str().unwrap()).unwrap();
    assert_eq!(loaded.id, meta.id);
    assert_eq!(loaded.command, meta.command);
    assert_eq!(loaded.status, meta.status);
}

/// Test that loading a non-existent task returns TaskNotFound error.
#[test]
fn test_toml_load_nonexistent() {
    let temp = TempDir::new().unwrap();
    let id = TaskId::from_raw("NoExist1".to_string());
    let result = task_store::load_task_meta(&id, temp.path().to_str().unwrap());
    assert!(result.is_err());
    match result {
        Err(SmolError::TaskNotFound(_)) => {} // expected
        _ => panic!("Expected TaskNotFound error"),
    }
}

/// Test task_exists returns true/false correctly.
#[test]
fn test_toml_task_exists() {
    let temp = TempDir::new().unwrap();
    storage::init(temp.path().to_str().unwrap()).unwrap();
    let meta = test_meta("Exist123", "echo hi");

    save_task_to_dir(&meta, temp.path().to_str().unwrap());

    assert!(task_store::task_exists(&meta.id, temp.path().to_str().unwrap()));
    let fake_id = TaskId::from_raw("Fake1234".to_string());
    assert!(!task_store::task_exists(&fake_id, temp.path().to_str().unwrap()));
}

/// Test listing tasks sorted by date (most recent first).
#[test]
fn test_toml_list_tasks_sorted() {
    let temp = TempDir::new().unwrap();
    storage::init(temp.path().to_str().unwrap()).unwrap();

    // Create two tasks with different creation times
    let mut meta1 = test_meta("LstTsk01", "first command");
    meta1.created_at = Utc::now() - Duration::hours(2);
    save_task_to_dir(&meta1, temp.path().to_str().unwrap());

    let mut meta2 = test_meta("LstTsk02", "second command");
    meta2.created_at = Utc::now() - Duration::hours(1);
    save_task_to_dir(&meta2, temp.path().to_str().unwrap());

    // List should return most recent first
    let tasks = task_store::list_tasks(temp.path().to_str().unwrap(), None).unwrap();
    assert_eq!(tasks.len(), 2);
    assert_eq!(tasks[0].id, meta2.id, "Most recent task should be first");
}

/// Test cleaning tasks older than a threshold.
#[test]
fn test_toml_clean_older_than() {
    let temp = TempDir::new().unwrap();
    storage::init(temp.path().to_str().unwrap()).unwrap();

    // Create old completed task
    let mut old_meta = test_meta("OldTask01", "old command");
    old_meta.created_at = Utc::now() - Duration::hours(2);
    old_meta.status = TaskStatus::Success;
    save_task_to_dir(&old_meta, temp.path().to_str().unwrap());

    // Create recent task
    let mut new_meta = test_meta("NewTask01", "new command");
    new_meta.created_at = Utc::now();
    save_task_to_dir(&new_meta, temp.path().to_str().unwrap());

    // Clean older than 1 hour
    let cleaned = task_store::clean_older_than(temp.path().to_str().unwrap(), 3600).unwrap();
    assert_eq!(cleaned, 1, "Should clean exactly 1 old task");

    // Only the recent task should remain
    let remaining = task_store::list_tasks(temp.path().to_str().unwrap(), None).unwrap();
    assert_eq!(remaining.len(), 1, "Only 1 task should remain");
    assert_eq!(remaining[0].id, new_meta.id);
}

/// Test that running tasks are not cleaned.
#[test]
fn test_toml_clean_skips_running() {
    let temp = TempDir::new().unwrap();
    storage::init(temp.path().to_str().unwrap()).unwrap();

    let mut running_meta = test_meta("Runnin01", "running task");
    running_meta.created_at = Utc::now() - Duration::hours(2);
    running_meta.status = TaskStatus::Running; // running tasks should not be cleaned
    save_task_to_dir(&running_meta, temp.path().to_str().unwrap());

    let cleaned = task_store::clean_older_than(temp.path().to_str().unwrap(), 3600).unwrap();
    assert_eq!(cleaned, 0, "Running tasks should not be cleaned");
}

// ── SQLite Storage Tests ────────────────────────────────────────

/// Helper to create an in-memory SQLite storage.
fn sqlite_storage() -> sqlite::SqliteStorage {
    let storage = sqlite::SqliteStorage::new(":memory:").unwrap();
    storage.init().unwrap();
    storage
}

/// Test SQLite save and load.
#[test]
fn test_sqlite_save_and_load() {
    let storage = sqlite_storage();
    let meta = test_meta("SqlTst01", "cargo build");
    storage.save_task(&meta).unwrap();

    let loaded = storage.load_task(&meta.id).unwrap();
    assert_eq!(loaded.id, meta.id);
    assert_eq!(loaded.command, meta.command);
    assert_eq!(loaded.status, meta.status);
    assert_eq!(loaded.mode, meta.mode);
    assert_eq!(loaded.error_count, meta.error_count);
    assert_eq!(loaded.warning_count, meta.warning_count);
}

/// Test SQLite load non-existent task.
#[test]
fn test_sqlite_load_nonexistent() {
    let storage = sqlite_storage();
    let id = TaskId::from_raw("NoSqlId1".to_string());
    let result = storage.load_task(&id);
    assert!(result.is_err());
    match result {
        Err(SmolError::TaskNotFound(_)) => {}
        _ => panic!("Expected TaskNotFound"),
    }
}

/// Test SQLite list all tasks.
#[test]
fn test_sqlite_list_tasks() {
    let storage = sqlite_storage();
    let meta1 = test_meta("SqlLst01", "first");
    let meta2 = test_meta("SqlLst02", "second");
    storage.save_task(&meta1).unwrap();
    storage.save_task(&meta2).unwrap();

    let tasks = storage.list_tasks(false).unwrap();
    assert_eq!(tasks.len(), 2);
}

/// Test SQLite list running tasks only.
#[test]
fn test_sqlite_list_running() {
    let storage = sqlite_storage();
    let mut meta1 = test_meta("SqlRun01", "running");
    meta1.status = TaskStatus::Running;
    let meta2 = test_meta("SqlRun02", "completed");
    storage.save_task(&meta1).unwrap();
    storage.save_task(&meta2).unwrap();

    let running = storage.list_tasks(true).unwrap();
    assert_eq!(running.len(), 1);
    assert_eq!(running[0].id, meta1.id);
}

/// Test SQLite search by command.
#[test]
fn test_sqlite_search_tasks() {
    let storage = sqlite_storage();
    let meta1 = test_meta("SrchSql1", "npm build");
    let meta2 = test_meta("SrchSql2", "cargo test");
    storage.save_task(&meta1).unwrap();
    storage.save_task(&meta2).unwrap();

    let results = storage.search_tasks("npm").unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].command, "npm build");

    let all = storage.search_tasks("").unwrap();
    assert_eq!(all.len(), 2);
}

/// Test SQLite update task status.
#[test]
fn test_sqlite_update_task_status() {
    let storage = sqlite_storage();
    let mut meta = test_meta("UpdSql01", "test command");
    meta.status = TaskStatus::Running;
    storage.save_task(&meta).unwrap();

    let now = Utc::now();
    storage.update_task_status(&meta.id, &TaskStatus::Success, &now, Some(0), 0, 0).unwrap();

    let loaded = storage.load_task(&meta.id).unwrap();
    assert_eq!(loaded.status, TaskStatus::Success);
}

/// Test SQLite clean older than.
#[test]
fn test_sqlite_clean_older_than() {
    let storage = sqlite_storage();
    let mut meta = test_meta("ClnSql01", "old task");
    meta.created_at = Utc::now() - Duration::seconds(100);
    meta.status = TaskStatus::Success;
    storage.save_task(&meta).unwrap();

    let deleted = storage.clean_older_than(50).unwrap();
    assert_eq!(deleted, 1);

    let tasks = storage.list_tasks(false).unwrap();
    assert!(tasks.is_empty());
}

/// Test SQLite save and load errors.
#[test]
fn test_sqlite_save_and_load_errors() {
    let storage = sqlite_storage();
    let meta = test_meta("ErrSql01", "compile");
    storage.save_task(&meta).unwrap();

    let errors = vec![
        smol::core::ErrorLine {
            line_num: 10,
            content: "undefined variable".to_string(),
            file: Some("src/main.rs".to_string()),
            file_line: Some(42),
            column: Some(5),
        },
    ];

    storage.save_errors(&meta.id, &errors).unwrap();
    let loaded = storage.load_errors(&meta.id).unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].content, "undefined variable");
}

/// Test SQLite save and load warnings.
#[test]
fn test_sqlite_save_and_load_warnings() {
    let storage = sqlite_storage();
    let meta = test_meta("WrnSql01", "build");
    storage.save_task(&meta).unwrap();

    let warnings = vec![
        smol::core::WarningLine {
            line_num: 5,
            content: "unused import".to_string(),
            file: Some("src/lib.rs".to_string()),
            file_line: Some(3),
        },
    ];

    storage.save_warnings(&meta.id, &warnings).unwrap();
    let loaded = storage.load_warnings(&meta.id).unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].content, "unused import");
}

/// Test SQLite init creates schema.
#[test]
fn test_sqlite_init_creates_tables() {
    let storage = sqlite_storage();
    // Just verify it doesn't panic - successfully created tables
    let meta = test_meta("InitSql01", "init test");
    storage.save_task(&meta).unwrap();
    let loaded = storage.load_task(&meta.id).unwrap();
    assert_eq!(loaded.id, meta.id);
}
