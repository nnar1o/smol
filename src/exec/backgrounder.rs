use std::process::{Command, Stdio};
use std::fs::{self, File};
use std::path::Path;

use crate::core::{SmolError, TaskId, TaskMeta, TaskStatus, TaskMode};

/// Launch a command in the background.
/// Returns the task ID and spawned process info.
///
/// The child process is fully detached from the parent:
/// - stdio is redirected to files
/// - on Unix, `setsid()` creates a new session (no terminal, separate process group)
/// - stdin is connected to `/dev/null`
pub fn run_background(
    cmd: &[String],
    task_id: &TaskId,
    tasks_dir: &str,
    _max_bytes: u64,
) -> Result<(TaskMeta, std::process::Child), SmolError> {
    // Create task directory
    let task_dir = Path::new(tasks_dir).join(task_id.as_str());
    fs::create_dir_all(&task_dir)?;

    // Open log files
    let stdout_path = task_dir.join("output.log");
    let stderr_path = task_dir.join("error.log");
    let stdout_file = File::create(&stdout_path)?;
    let stderr_file = File::create(&stderr_path)?;

    // Build command with redirected stdio
    let mut command = Command::new(&cmd[0]);
    command
        .args(&cmd[1..])
        .stdout(
            stdout_file
                .try_clone()
                .map_err(|_| SmolError::other("Failed to clone stdout file"))?,
        )
        .stderr(stderr_file)
        .stdin(Stdio::null());

    // Daemonize on Unix: create a new session so the child is
    // detached from the parent's terminal and process group.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // SAFETY: pre_exec runs in the child after fork but before exec.
        // libc::setsid() is safe to call here and is the standard way
        // to daemonize a process.
        unsafe {
            command.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
    }

    let child = command.spawn()?;

    let meta = TaskMeta {
        id: task_id.clone(),
        command: cmd.join(" "),
        mode: TaskMode::Background,
        created_at: chrono::Utc::now(),
        completed_at: None,
        exit_code: None,
        duration_sec: None,
        status: TaskStatus::Running,
        total_lines: 0,
        total_chars: 0,
        output_size_bytes: 0,
        error_count: 0,
        warning_count: 0,
        pid: Some(child.id()),
        background_pid: Some(child.id()),
        input_tokens: None,
        output_tokens: None,
        compression_ratio: None,
    };

    Ok((meta, child))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_background_creates_task() {
        let dir = std::env::temp_dir().join("smol-test-bg");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let task_id = TaskId::new();
        let result = run_background(
            &["echo".into(), "bg test".into()],
            &task_id,
            dir.to_str().unwrap(),
            1024 * 1024,
        );

        assert!(result.is_ok());
        let (meta, mut child) = result.unwrap();
        assert_eq!(meta.id, task_id);
        assert_eq!(meta.status, TaskStatus::Running);

        // Wait for child
        let status = child.wait().unwrap();
        assert!(status.success());

        // Check that log files exist
        let task_dir = dir.join(task_id.as_str());
        assert!(task_dir.join("output.log").exists());
        assert!(task_dir.join("error.log").exists());

        let _ = fs::remove_dir_all(&dir);
    }
}
