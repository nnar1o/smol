pub mod spawner;
pub mod watcher;
pub mod backgrounder;

use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crate::core::SmolError;

/// Result of running a command.
#[derive(Debug)]
pub struct RunResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub pid: Option<u32>,
    pub duration_sec: u64,
}

/// Spawn a command and return the child process with piped output.
pub fn spawn_command(cmd: &[String]) -> Result<(Child, std::process::ChildStdout, std::process::ChildStderr), SmolError> {
    if cmd.is_empty() {
        return Err(SmolError::other("Empty command"));
    }

    let program = &cmd[0];
    let args = &cmd[1..];

    let mut command = Command::new(program);
    command.args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command.spawn()?;
    let stdout = child.stdout.take()
        .ok_or_else(|| SmolError::other("Failed to capture stdout"))?;
    let stderr = child.stderr.take()
        .ok_or_else(|| SmolError::other("Failed to capture stderr"))?;

    Ok((child, stdout, stderr))
}

/// Read all output from the child process into strings.
pub fn read_all_output(
    child: &mut Child,
    stdout: std::process::ChildStdout,
    stderr: std::process::ChildStderr,
    max_bytes: u64,
) -> Result<(String, String), SmolError> {
    let (tx, rx) = mpsc::channel();

    // Thread for stdout
    let tx_stdout = tx.clone();
    let handle_stdout = thread::spawn(move || {
        let mut reader = stdout;
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).ok();
        tx_stdout.send(("stdout", buf)).ok();
    });

    // Thread for stderr
    let handle_stderr = thread::spawn(move || {
        let mut reader = stderr;
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).ok();
        tx.send(("stderr", buf)).ok();
    });

    // Wait for both threads
    handle_stdout.join().ok();
    handle_stderr.join().ok();

    // Collect results
    let mut stdout_str = String::new();
    let mut stderr_str = String::new();
    for (stream, data) in rx.iter() {
        // Truncate if too large
        let max = max_bytes as usize;
        let truncated = if data.len() > max {
            &data[..max]
        } else {
            &data[..]
        };
        let s = String::from_utf8_lossy(truncated).to_string();
        match stream {
            "stdout" => stdout_str = s,
            "stderr" => stderr_str = s,
            _ => {}
        }
    }

    // Wait for child
    let _ = child.wait();

    Ok((stdout_str, stderr_str))
}

/// Run a command synchronously and return the output.
pub fn run_sync(cmd: &[String], max_bytes: u64) -> Result<RunResult, SmolError> {
    let start = std::time::Instant::now();
    let (mut child, stdout, stderr) = spawn_command(cmd)?;
    let pid = child.id();
    let (stdout_str, stderr_str) = read_all_output(&mut child, stdout, stderr, max_bytes)?;
    let exit_code = child.wait()?.code();
    let duration = start.elapsed().as_secs();

    Ok(RunResult {
        stdout: stdout_str,
        stderr: stderr_str,
        exit_code,
        pid: Some(pid),
        duration_sec: duration,
    })
}

/// Run a command with a timeout (for auto mode).
/// If the command finishes before timeout, returns RunResult.
/// If it's still running after timeout, returns the child for backgrounding.
pub fn run_with_timeout(
    cmd: &[String],
    max_bytes: u64,
    timeout_secs: u64,
) -> Result<RunResult, SmolError> {
    let start = std::time::Instant::now();
    let (mut child, stdout_pipe, stderr_pipe) = spawn_command(cmd)?;
    let pid = child.id();

    // Read output in a thread with timeout
    let (tx, rx) = mpsc::channel();
    let tx_out = tx.clone();

    thread::spawn(move || {
        let mut stdout_reader = stdout_pipe;
        let mut stderr_reader = stderr_pipe;
        let mut stdout_buf = Vec::new();
        let mut stderr_buf = Vec::new();
        stdout_reader.read_to_end(&mut stdout_buf).ok();
        stderr_reader.read_to_end(&mut stderr_buf).ok();
        tx_out.send((stdout_buf, stderr_buf)).ok();
    });

    // Wait for either completion or timeout
    let timeout = Duration::from_secs(timeout_secs);
    let elapsed = start.elapsed();

    let (stdout_buf, stderr_buf) = if elapsed < timeout {
        // Try to receive with remaining timeout
        let remaining = timeout - elapsed;
        match rx.recv_timeout(remaining) {
            Ok(data) => data,
            Err(_) => {
                // Timed out — kill child and return what we have
                let _ = child.kill();
                let _ = child.wait();
                return Err(SmolError::CommandTimedOut);
            }
        }
    } else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(SmolError::CommandTimedOut);
    };

    let exit_code = child.wait()?.code();
    let duration = start.elapsed().as_secs();

    // Apply max_bytes limit
    let truncate = |buf: Vec<u8>| -> String {
        let max = max_bytes as usize;
        let data = if buf.len() > max { &buf[..max] } else { &buf[..] };
        String::from_utf8_lossy(data).to_string()
    };

    Ok(RunResult {
        stdout: truncate(stdout_buf),
        stderr: truncate(stderr_buf),
        exit_code,
        pid: Some(pid),
        duration_sec: duration,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_empty_command() {
        let result = spawn_command(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_run_sync_echo() {
        let result = run_sync(&["echo".into(), "hello smol".into()], 1024 * 1024).unwrap();
        assert!(result.stdout.contains("hello smol"));
        assert_eq!(result.exit_code, Some(0));
    }

    #[test]
    fn test_run_sync_false() {
        let result = run_sync(&["false".into()], 1024 * 1024).unwrap();
        assert_eq!(result.exit_code, Some(1));
    }

    #[test]
    fn test_run_sync_with_output_truncation() {
        // Generate 100KB of output
        let result = run_sync(&["bash".into(), "-c".into(), "for i in {1..10000}; do echo 'line '$i; done".into()], 1024).unwrap();
        // Should be truncated but still have some content
        assert!(result.stdout.len() <= 1024);
        assert!(result.exit_code == Some(0));
    }

    #[test]
    fn test_run_with_timeout_completes() {
        let result = run_with_timeout(&["echo".into(), "quick".into()], 1024 * 1024, 5).unwrap();
        assert!(result.stdout.contains("quick"));
    }

    #[test]
    fn test_run_with_timeout_exceeded() {
        let result = run_with_timeout(&["sleep".into(), "10".into()], 1024 * 1024, 1);
        assert!(result.is_err());
    }
}
