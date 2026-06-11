use crate::core::SmolError;
use super::RunResult;

/// Options for the watcher when running in auto mode.
#[derive(Debug, Clone)]
pub struct WatchOptions {
    pub max_bytes: u64,
    pub timeout_secs: u64,
}

impl Default for WatchOptions {
    fn default() -> Self {
        Self {
            max_bytes: 10 * 1024 * 1024,
            timeout_secs: 5,
        }
    }
}

/// Result from the watcher — either the command finished, or it needs backgrounding.
#[derive(Debug)]
pub enum WatchResult {
    Completed(RunResult),
    NeedsBackground {
        /// Partial output collected so far.
        partial_stdout: String,
        partial_stderr: String,
        pid: u32,
        /// The command string for backgrounding.
        command: Vec<String>,
    },
}

/// Watch a command for auto-mode: wait up to timeout_secs.
/// If it finishes, return Completed. Otherwise return NeedsBackground.
pub fn watch_command(
    cmd: &[String],
    options: &WatchOptions,
) -> Result<WatchResult, SmolError> {
    match super::run_with_timeout(cmd, options.max_bytes, options.timeout_secs) {
        Ok(result) => Ok(WatchResult::Completed(result)),
        Err(SmolError::CommandTimedOut) => Ok(WatchResult::NeedsBackground {
            partial_stdout: String::new(),
            partial_stderr: String::new(),
            pid: 0,
            command: cmd.to_vec(),
        }),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watch_completed() {
        let options = WatchOptions { max_bytes: 1024 * 1024, timeout_secs: 5 };
        let result = watch_command(&["echo".into(), "done".into()], &options).unwrap();
        match result {
            WatchResult::Completed(r) => assert!(r.stdout.contains("done")),
            _ => panic!("Expected Completed"),
        }
    }
}
