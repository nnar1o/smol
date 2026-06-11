use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::task_id::TaskId;

/// How the task was launched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskMode {
    Sync,
    Auto,
    Background,
}

impl TaskMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskMode::Sync => "sync",
            TaskMode::Auto => "auto",
            TaskMode::Background => "background",
        }
    }
}

/// Current status of a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Running,
    Success,
    Failed,
    Cancelled,
    TimedOut,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Running => "running",
            TaskStatus::Success => "success",
            TaskStatus::Failed => "failed",
            TaskStatus::Cancelled => "cancelled",
            TaskStatus::TimedOut => "timeout",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, TaskStatus::Success | TaskStatus::Failed | TaskStatus::Cancelled | TaskStatus::TimedOut)
    }
}

/// Metadata for a single task, persisted in `meta.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMeta {
    pub id: TaskId,
    pub command: String,
    pub mode: TaskMode,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub exit_code: Option<i32>,
    pub duration_sec: Option<u64>,
    pub status: TaskStatus,
    pub total_lines: u64,
    pub total_chars: u64,
    pub output_size_bytes: u64,
    pub error_count: u32,
    pub warning_count: u32,
    pub pid: Option<u32>,
    pub background_pid: Option<u32>,
    /// Estimated input tokens at parse time.
    pub input_tokens: Option<usize>,
    /// Estimated output tokens in the summary.
    pub output_tokens: Option<usize>,
    /// Compression ratio (output/input) if input > 0.
    pub compression_ratio: Option<f64>,
}

/// A runtime task handle (includes the actual process if still running).
#[derive(Debug)]
pub struct Task {
    pub meta: TaskMeta,
    pub stdout_path: String,
    pub stderr_path: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_status_terminal() {
        assert!(TaskStatus::Success.is_terminal());
        assert!(TaskStatus::Failed.is_terminal());
        assert!(TaskStatus::Cancelled.is_terminal());
        assert!(TaskStatus::TimedOut.is_terminal());
        assert!(!TaskStatus::Running.is_terminal());
    }

    #[test]
    fn test_task_mode_as_str() {
        assert_eq!(TaskMode::Sync.as_str(), "sync");
        assert_eq!(TaskMode::Auto.as_str(), "auto");
        assert_eq!(TaskMode::Background.as_str(), "background");
    }

    #[test]
    fn test_task_status_as_str() {
        assert_eq!(TaskStatus::Running.as_str(), "running");
        assert_eq!(TaskStatus::Success.as_str(), "success");
        assert_eq!(TaskStatus::Failed.as_str(), "failed");
    }
}
