pub mod registry;
pub mod task_store;
pub mod paths;
pub mod sqlite;
pub mod search;
pub mod export;

use crate::core::{SmolError, TaskId, TaskMeta, Task, TaskStatus};

/// Initialize the storage system.
pub fn init(tasks_dir: &str) -> Result<(), SmolError> {
    std::fs::create_dir_all(tasks_dir)?;
    Ok(())
}

/// Save a task's metadata and output.
pub fn save_task(task: &Task) -> Result<(), SmolError> {
    task_store::save_task(task)
}

/// Load task metadata by ID.
pub fn load_task_meta(task_id: &TaskId, tasks_dir: &str) -> Result<TaskMeta, SmolError> {
    task_store::load_task_meta(task_id, tasks_dir)
}

/// Check if a task exists.
pub fn task_exists(task_id: &TaskId, tasks_dir: &str) -> bool {
    task_store::task_exists(task_id, tasks_dir)
}

/// List all tasks, optionally filtered by status.
pub fn list_tasks(tasks_dir: &str, status_filter: Option<TaskStatus>) -> Result<Vec<TaskMeta>, SmolError> {
    task_store::list_tasks(tasks_dir, status_filter)
}

/// Cancel a running task by PID.
pub fn cancel_task(task_id: &TaskId, tasks_dir: &str) -> Result<(), SmolError> {
    task_store::cancel_task(task_id, tasks_dir)
}

/// Clean up old tasks.
pub fn clean_older_than(tasks_dir: &str, older_than_secs: u64) -> Result<usize, SmolError> {
    task_store::clean_older_than(tasks_dir, older_than_secs)
}
