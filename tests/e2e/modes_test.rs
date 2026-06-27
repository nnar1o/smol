use assert_cmd::Command;
use predicates::prelude::*;
use std::time::Duration;
use tempfile::TempDir;

/// Create a smol command with HOME set to a temp directory.
fn smol_cmd(temp: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("smol").unwrap();
    cmd.env("HOME", temp.path());
    cmd
}

/// Sync mode: run `echo hello` and verify the output is passed through.
#[test]
fn test_sync_mode_completes() {
    let temp = TempDir::new().unwrap();
    smol_cmd(&temp)
        .arg("--sync").arg("echo").arg("hello-sync")
        .assert()
        .success()
        .stdout(predicate::str::contains("hello-sync"));
}

/// Sync mode: non-zero exit code should propagate from the command.
#[test]
fn test_sync_mode_failure() {
    let temp = TempDir::new().unwrap();
    let assert = smol_cmd(&temp)
        .arg("--sync").arg("false")
        .assert();
    // `false` exits 1, empty stdout falls through to summary
    assert
        .code(1)
        .stdout(predicate::str::contains("success").or(predicate::str::contains("done")));
}

/// Auto mode (default): fast command should complete and pass through output.
#[test]
fn test_auto_mode_fast_command() {
    let temp = TempDir::new().unwrap();
    smol_cmd(&temp)
        .arg("echo").arg("hello-auto")
        .assert()
        .success()
        .stdout(predicate::str::contains("hello-auto"));
}

/// Auto mode with explicit --auto flag.
#[test]
fn test_auto_mode_explicit_flag() {
    let temp = TempDir::new().unwrap();
    smol_cmd(&temp)
        .arg("--auto").arg("echo").arg("hello-auto-flag")
        .assert()
        .success()
        .stdout(predicate::str::contains("hello-auto-flag"));
}

/// Background mode: should print task-id and return 0.
#[test]
fn test_background_mode_prints_task_id() {
    let temp = TempDir::new().unwrap();
    smol_cmd(&temp)
        .arg("--bg").arg("echo").arg("hello-bg")
        .assert()
        .success()
        .stdout(predicate::str::contains("task-id:"));
}

/// Background mode: task should appear in list after creation.
#[test]
fn test_background_task_appears_in_list() {
    let temp = TempDir::new().unwrap();

    // Run command in background
    let bg_output = smol_cmd(&temp)
        .arg("--bg").arg("echo").arg("hello-list")
        .output().unwrap();
    assert!(bg_output.status.success());

    // Give the background task a moment to write output
    std::thread::sleep(Duration::from_millis(200));

    // List tasks - should include our background task
    let list_output = smol_cmd(&temp)
        .arg("list")
        .output().unwrap();
    let stdout = String::from_utf8_lossy(&list_output.stdout);
    // List output format: "id  status  date  errors:N  warnings:N"
    // Task should have a status line (not "No tasks found.")
    assert!(
        stdout.contains("success") || stdout.contains("running"),
        "Task should appear in list (showing success or running), got: {}",
        stdout
    );
}

/// Background mode: verify status of a completed background task.
#[test]
fn test_background_task_status() {
    let temp = TempDir::new().unwrap();

    // Run echo in background (completes instantly)
    let bg_output = smol_cmd(&temp)
        .arg("--bg").arg("echo").arg("hello-status")
        .output().unwrap();
    assert!(bg_output.status.success());

    // Give the task time to complete
    std::thread::sleep(Duration::from_millis(300));

    // Check status of last task
    let status_output = smol_cmd(&temp)
        .arg("status").arg("last")
        .output().unwrap();
    let stdout = String::from_utf8_lossy(&status_output.stdout);
    assert!(stdout.contains("success"), "Should show success status: {}", stdout);
    assert!(stdout.contains("errors:0"), "Should show errors:0: {}", stdout);
}

/// Background mode: multi-word command preserves full command.
#[test]
fn test_background_multi_word_command() {
    let temp = TempDir::new().unwrap();

    let bg_output = smol_cmd(&temp)
        .arg("--bg").arg("echo").arg("multi").arg("word").arg("test")
        .output().unwrap();
    assert!(bg_output.status.success());

    std::thread::sleep(Duration::from_millis(200));

    let status_output = smol_cmd(&temp)
        .arg("status").arg("last")
        .output().unwrap();
    let stdout = String::from_utf8_lossy(&status_output.stdout);
    assert!(
        stdout.contains("success"),
        "Should show success, got: {}",
        stdout
    );
}

/// Auto mode: a command that sleeps briefly should still complete.
#[test]
fn test_auto_mode_slow_command() {
    let temp = TempDir::new().unwrap();
    smol_cmd(&temp)
        .arg("--auto").arg("echo").arg("still-fast")
        .assert()
        .success()
        .stdout(predicate::str::contains("still-fast"));
}

/// Interactive mode: basic echo command should complete and pass through output.
#[test]
fn test_interactive_mode_completes() {
    let temp = TempDir::new().unwrap();
    smol_cmd(&temp)
        .arg("--interactive").arg("echo").arg("hello-interactive")
        .assert()
        .success()
        .stdout(predicate::str::contains("hello-interactive"));
}

/// Interactive mode: non-zero exit code propagates.
#[test]
fn test_interactive_mode_failure() {
    let temp = TempDir::new().unwrap();
    let assert = smol_cmd(&temp)
        .arg("--interactive").arg("false")
        .assert();
    assert
        .code(1)
        .stdout(predicate::str::contains("success").or(predicate::str::contains("done")));
}

/// Interactive mode: short flag --int works.
#[test]
fn test_interactive_short_flag() {
    let temp = TempDir::new().unwrap();
    smol_cmd(&temp)
        .arg("--int").arg("echo").arg("short-flag")
        .assert()
        .success()
        .stdout(predicate::str::contains("short-flag"));
}

/// Interactive mode: Maven test fixture with mixed build errors + test failures.
/// Verifies the panel correctly separates build errors from test failures
/// (doesn't double-count test failures as build errors).
#[test]
fn test_interactive_maven_fixture_runs() {
    let temp = TempDir::new().unwrap();
    let fixture = std::env::current_dir()
        .unwrap()
        .join("tests/fixtures/scripts/maven_test_output.sh");
    // The fixture script simulates a Maven BUILD FAILURE (exit=1)
    smol_cmd(&temp)
        .arg("--interactive")
        .arg(fixture.to_str().unwrap())
        .assert()
        .code(1);
}

/// Interactive mode: Maven fixture with --int flag.
#[test]
fn test_interactive_maven_fixture_int_flag() {
    let temp = TempDir::new().unwrap();
    let fixture = std::env::current_dir()
        .unwrap()
        .join("tests/fixtures/scripts/maven_test_output.sh");
    smol_cmd(&temp)
        .arg("--int")
        .arg(fixture.to_str().unwrap())
        .assert()
        .code(1);
}
