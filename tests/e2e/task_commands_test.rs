use assert_cmd::Command;
use predicates::prelude::*;
use std::time::Duration;
use tempfile::TempDir;

/// Helper to run a background command and capture the task id from output.
fn run_background_task(temp: &TempDir, args: &[&str]) -> String {
    let mut cmd = Command::cargo_bin("smol").unwrap();
    cmd.env("HOME", temp.path());
    cmd.arg("--bg");
    for arg in args {
        cmd.arg(arg);
    }
    let output = cmd.output().unwrap();
    assert!(
        output.status.success(),
        "bg command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().next().expect("Expected task-id line");
    assert!(line.starts_with("task-id:"), "Expected task-id:, got: {:?}", line);
    line.trim_start_matches("task-id:").to_string()
}

/// Helper to run a smol command and get its stdout as string.
/// Panics if the command fails, showing stderr.
fn smol_output(temp: &TempDir, args: &[&str]) -> String {
    let mut cmd = Command::cargo_bin("smol").unwrap();
    cmd.env("HOME", temp.path());
    for arg in args {
        cmd.arg(arg);
    }
    let output = cmd.output().unwrap();
    if !output.status.success() {
        panic!(
            "smol command failed: {:?}\nstdout: {}\nstderr: {}",
            args,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Test the full status/log/list/clean workflow.
#[test]
fn test_task_lifecycle() {
    let temp = TempDir::new().unwrap();

    // 1. Run a background task
    let task_id = run_background_task(&temp, &["echo", "lifecycle-test-123"]);
    assert_eq!(task_id.len(), 8, "task-id should be 8 chars");

    // Give time for the task to complete and write output
    std::thread::sleep(Duration::from_millis(300));

    // 2. Check status shows correct fields (compact format)
    let status_out = smol_output(&temp, &["status", &task_id]);
    assert!(status_out.contains(&task_id), "Status should contain task id");
    assert!(status_out.contains("lifecycle-test-123"), "Status should contain command");
    assert!(status_out.contains("success"), "Status should show success status");
    assert!(status_out.contains("err:"), "Status should have err field");
    assert!(status_out.contains("warn:"), "Status should have warn field");

    // 3. Check status last works
    let status_last = smol_output(&temp, &["status", "last"]);
    assert!(status_last.contains(&task_id), "status last should show our task");

    // 4. Check list shows the task
    let list_out = smol_output(&temp, &["list"]);
    assert!(list_out.contains(&task_id), "list should contain our task id: {}", list_out);

    // 5. Check log shows output
    let log_out = smol_output(&temp, &["log", &task_id]);
    assert!(log_out.contains("lifecycle-test-123"), "log should contain command output");

    // 6. Check log --stats shows JSON
    let stats_out = smol_output(&temp, &["log", &task_id, "--stats"]);
    assert!(stats_out.contains("\"id\""), "stats should contain id field");
    assert!(stats_out.contains("\"command\""), "stats should contain command field");
    assert!(stats_out.contains("\"status\""), "stats should contain status field");

    // 7. Clean old tasks (0 seconds = all completed tasks)
    let clean_out = smol_output(&temp, &["clean", "--older", "0s"]);
    assert!(clean_out.contains("Cleaned up"), "clean should report cleanup count");

    // 8. List should now show no tasks
    let list_after = smol_output(&temp, &["list"]);
    assert!(list_after.contains("No tasks found."), "After clean, there should be no tasks");
}

/// Test that `smol clean` without --older defaults to 24h and doesn't remove fresh tasks.
#[test]
fn test_clean_default() {
    let temp = TempDir::new().unwrap();
    let task_id = run_background_task(&temp, &["echo", "default-clean"]);
    std::thread::sleep(Duration::from_millis(200));

    // Default clean (24h) should not remove fresh tasks
    let clean_out = smol_output(&temp, &["clean"]);
    assert!(clean_out.contains("Cleaned up 0 task(s)."));

    // Task should still exist
    let status_out = smol_output(&temp, &["status", &task_id]);
    assert!(status_out.contains(&task_id), "Task should still exist after default clean");
}

/// Test `status` for a task that does not exist.
#[test]
fn test_status_nonexistent_task() {
    let temp = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("smol").unwrap();
    cmd.env("HOME", temp.path())
        .arg("status")
        .arg("BadId00");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Error"));
}

/// Test `list --running` when no tasks are running.
#[test]
fn test_list_running_no_tasks() {
    let temp = TempDir::new().unwrap();
    let list_out = smol_output(&temp, &["list", "--running"]);
    assert!(list_out.contains("No tasks found.") || list_out.trim().is_empty());
}

/// Test `list` shows multiple tasks sorted by date.
#[test]
fn test_list_multiple_tasks() {
    let temp = TempDir::new().unwrap();

    // Create two tasks
    let id1 = run_background_task(&temp, &["echo", "task-one"]);
    let id2 = run_background_task(&temp, &["echo", "task-two"]);
    std::thread::sleep(Duration::from_millis(300));

    let list_out = smol_output(&temp, &["list"]);
    assert!(list_out.contains(&id1), "List should contain first task id");
    assert!(list_out.contains(&id2), "List should contain second task id");
}

/// Test that `log` for a task with output shows the output content.
#[test]
fn test_log_shows_output() {
    let temp = TempDir::new().unwrap();
    let task_id = run_background_task(&temp, &["echo", "specific-output-content"]);
    std::thread::sleep(Duration::from_millis(200));

    let log_out = smol_output(&temp, &["log", &task_id]);
    assert!(log_out.contains("specific-output-content"));
}

/// Test that `clean --older 24h` doesn't remove recently created tasks.
#[test]
fn test_clean_keeps_recent() {
    let temp = TempDir::new().unwrap();
    let task_id = run_background_task(&temp, &["echo", "keep-me"]);
    std::thread::sleep(Duration::from_millis(200));

    // Clean with 24h threshold - should NOT remove our just-created task
    let clean_out = smol_output(&temp, &["clean", "--older", "24h"]);
    assert!(clean_out.contains("Cleaned up 0 task(s)."));

    // Task should still exist
    let status_out = smol_output(&temp, &["status", &task_id]);
    assert!(status_out.contains(&task_id), "Task should still exist after clean with 24h threshold");
}

/// Test that `log` with `--errors` flag works.
/// The --errors flag filters to lines containing "error" (case-insensitive).
#[test]
fn test_log_errors_flag() {
    let temp = TempDir::new().unwrap();
    let task_id = run_background_task(&temp, &["echo", "clean-pass-no-issues"]);
    std::thread::sleep(Duration::from_millis(300));

    let log_out = smol_output(&temp, &["log", &task_id, "--errors"]);
    // Clean output has no errors, so filtered output should be empty
    assert!(log_out.is_empty(), "--errors output should be empty for clean output, got: {:?}", log_out);
}

/// Test that `log` with `--warnings` flag works.
/// The --warnings flag filters to lines containing "warning" (case-insensitive).
#[test]
fn test_log_warnings_flag() {
    let temp = TempDir::new().unwrap();
    let task_id = run_background_task(&temp, &["echo", "clean-pass-no-issues"]);
    std::thread::sleep(Duration::from_millis(300));

    let log_out = smol_output(&temp, &["log", &task_id, "--warnings"]);
    // Clean output has no warnings, so filtered output should be empty
    assert!(log_out.is_empty(), "--warnings output should be empty for clean output, got: {:?}", log_out);
}
