use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

/// Test that `smol --help` prints usage information.
#[test]
fn test_help_flag() {
    let mut cmd = Command::cargo_bin("smol").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("smol — Smart Minimal Output Logger"));
}

/// Test that `smol help` subcommand prints usage.
#[test]
fn test_help_subcommand() {
    let mut cmd = Command::cargo_bin("smol").unwrap();
    cmd.arg("help");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("smol — Smart Minimal Output Logger"));
}

/// Test that `smol -h` short flag prints usage.
#[test]
fn test_help_short_flag() {
    let mut cmd = Command::cargo_bin("smol").unwrap();
    cmd.arg("-h");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
}

/// Test that `smol` with no arguments prints usage and exits.
#[test]
fn test_no_args() {
    let mut cmd = Command::cargo_bin("smol").unwrap();
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("smol — Smart Minimal Output Logger"));
}

/// Test that `smol status last` when no tasks exist prints a message.
#[test]
fn test_status_last_no_tasks() {
    let temp = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("smol").unwrap();
    cmd.env("HOME", temp.path())
        .arg("status").arg("last");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Error"));
}

/// Test that `smol list` when no tasks exist says "No tasks found."
#[test]
fn test_list_no_tasks() {
    let temp = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("smol").unwrap();
    cmd.env("HOME", temp.path())
        .arg("list");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No tasks found."));
}

/// Test that `smol clean` when no tasks exist does not error.
#[test]
fn test_clean_no_tasks() {
    let temp = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("smol").unwrap();
    cmd.env("HOME", temp.path())
        .arg("clean");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Cleaned up 0 task(s)."));
}

/// Test that `smol completion bash` generates a script.
#[test]
fn test_completion_bash() {
    let mut cmd = Command::cargo_bin("smol").unwrap();
    cmd.arg("completion").arg("bash");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("_smol_completions"));
}

/// Test that `smol completion zsh` generates a script.
#[test]
fn test_completion_zsh() {
    let mut cmd = Command::cargo_bin("smol").unwrap();
    cmd.arg("completion").arg("zsh");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("#compdef smol"));
}

/// Test that `smol completion fish` generates a script.
#[test]
fn test_completion_fish() {
    let mut cmd = Command::cargo_bin("smol").unwrap();
    cmd.arg("completion").arg("fish");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("__fish_smol_needs_command"));
}

/// Test that `status` subcommand with no task-id defaults to "last" and fails with no tasks.
#[test]
fn test_status_subcommand() {
    let temp = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("smol").unwrap();
    cmd.env("HOME", temp.path())
        .arg("status");
    // Default to "last" if no task-id given
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Error"));
}

/// Test that `smol log` without task-id prints error.
#[test]
fn test_log_without_task_id() {
    let mut cmd = Command::cargo_bin("smol").unwrap();
    cmd.arg("log");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("task-id required"));
}

/// Test that `smol cancel` without task-id prints error.
#[test]
fn test_cancel_without_task_id() {
    let mut cmd = Command::cargo_bin("smol").unwrap();
    cmd.arg("cancel");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("task-id required"));
}

/// Test that an unknown subcommand is treated as a command to run.
/// If the command doesn't exist, it should fail with an I/O error.
#[test]
fn test_unknown_subcommand_as_command() {
    let mut cmd = Command::cargo_bin("smol").unwrap();
    cmd.arg("nonexistent-command-xyz");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Error"));
}

/// Test that `--sync` flag before a command is recognized.
#[test]
fn test_sync_flag() {
    let mut cmd = Command::cargo_bin("smol").unwrap();
    cmd.arg("--sync").arg("echo").arg("hello-sync");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("success"));
}

/// Test that `--bg` flag before a command is recognized.
#[test]
fn test_bg_flag() {
    let mut cmd = Command::cargo_bin("smol").unwrap();
    cmd.arg("--bg").arg("echo").arg("hello-bg");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("task-id:"));
}

/// Test that `--mode sync` works.
#[test]
fn test_mode_sync_flag() {
    let mut cmd = Command::cargo_bin("smol").unwrap();
    cmd.arg("--mode").arg("sync").arg("echo").arg("hello-mode");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("success"));
}

/// Test that `--auto` flag is recognized.
#[test]
fn test_auto_flag() {
    let mut cmd = Command::cargo_bin("smol").unwrap();
    cmd.arg("--auto").arg("echo").arg("hello-auto");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("success").or(predicate::str::contains("done")));
}

/// Test that `smol completion` defaults to bash.
#[test]
fn test_completion_default() {
    let mut cmd = Command::cargo_bin("smol").unwrap();
    cmd.arg("completion");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("_smol_completions"));
}

/// Test that `smol completion <unknown>` shows error and exits with code 1.
#[test]
fn test_completion_unknown_shell() {
    let mut cmd = Command::cargo_bin("smol").unwrap();
    cmd.arg("completion").arg("unknown-shell");
    cmd.assert()
        .code(1)
        .stderr(predicate::str::contains("Unknown shell"));
}
