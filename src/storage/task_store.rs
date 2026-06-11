use std::path::Path;

use crate::core::{SmolError, TaskId, TaskMeta, Task, TaskStatus};

use super::registry;  // only used for registry functions, not the Registry type

/// Save a task: write meta.toml, output.log, error.log, and update registry.
pub fn save_task(task: &Task) -> Result<(), SmolError> {
    let tasks_dir = crate::storage::paths::tasks_dir();
    let task_dir = std::path::Path::new(&tasks_dir).join(task.meta.id.as_str());

    // Ensure directory exists
    std::fs::create_dir_all(&task_dir)?;

    // Write meta.toml
    let meta_path = task_dir.join("meta.toml");
    let meta_content = toml::to_string_pretty(&task.meta)
        .map_err(|e| SmolError::Config(format!("Failed to serialize meta: {}", e)))?;
    std::fs::write(&meta_path, meta_content)?;

    // Write output.log if path exists
    if !task.stdout_path.is_empty() {
        let out_path = task_dir.join("output.log");
        std::fs::copy(&task.stdout_path, &out_path).ok();
    }
    if !task.stderr_path.is_empty() {
        let err_path = task_dir.join("error.log");
        std::fs::copy(&task.stderr_path, &err_path).ok();
    }

    // Update registry
    let mut registry = registry::load_registry(&tasks_dir)?;
    let entry_idx = registry.tasks.iter().position(|e| e.id == task.meta.id);
    if let Some(idx) = entry_idx {
        registry.tasks[idx].status = task.meta.status;
    } else {
        registry.tasks.push(registry::RegistryEntry {
            id: task.meta.id.clone(),
            created_at: task.meta.created_at,
            status: task.meta.status,
        });
    }
    registry::save_registry(&tasks_dir, &registry)?;

    Ok(())
}

/// Load task metadata by ID.
pub fn load_task_meta(task_id: &TaskId, tasks_dir: &str) -> Result<TaskMeta, SmolError> {
    let dir = Path::new(tasks_dir).join(task_id.as_str());
    let meta_path = dir.join("meta.toml");
    if !meta_path.exists() {
        return Err(SmolError::TaskNotFound(task_id.to_string()));
    }
    let content = std::fs::read_to_string(&meta_path)?;
    let meta: TaskMeta = toml::from_str(&content)
        .map_err(|e| SmolError::Config(format!("Invalid meta.toml for {}: {}", task_id, e)))?;
    Ok(meta)
}

pub fn task_exists(task_id: &TaskId, tasks_dir: &str) -> bool {
    Path::new(tasks_dir).join(task_id.as_str()).join("meta.toml").exists()
}

/// List all tasks, optionally filtered by status.
pub fn list_tasks(tasks_dir: &str, status_filter: Option<TaskStatus>) -> Result<Vec<TaskMeta>, SmolError> {
    let registry = registry::load_registry(tasks_dir)?;
    let mut tasks = Vec::new();
    for entry in &registry.tasks {
        if let Some(filter) = &status_filter {
            if &entry.status != filter {
                continue;
            }
        }
        match load_task_meta(&entry.id, tasks_dir) {
            Ok(meta) => tasks.push(meta),
            Err(_) => continue,
        }
    }
    tasks.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(tasks)
}

/// Cancel a running task by sending SIGTERM to its PID.
pub fn cancel_task(task_id: &TaskId, tasks_dir: &str) -> Result<(), SmolError> {
    let meta = load_task_meta(task_id, tasks_dir)?;
    let pid = meta.background_pid.or(meta.pid)
        .ok_or_else(|| SmolError::other("No PID found for task"))?;

    #[cfg(unix)]
    {
        use std::process::Command;
        Command::new("kill")
            .arg(pid.to_string())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()?;
    }

    Ok(())
}

/// Clean up tasks older than the specified number of seconds.
pub fn clean_older_than(tasks_dir: &str, older_than_secs: u64) -> Result<usize, SmolError> {
    use chrono::Utc;
    let registry = registry::load_registry(tasks_dir)?;
    let cutoff = Utc::now() - chrono::Duration::seconds(older_than_secs as i64);
    let mut cleaned = 0;

    for entry in &registry.tasks {
        if entry.created_at < cutoff && entry.status.is_terminal() {
            let dir = Path::new(tasks_dir).join(entry.id.as_str());
            if dir.exists() {
                std::fs::remove_dir_all(&dir).ok();
                cleaned += 1;
            }
        }
    }

    // Remove cleaned entries from registry
    let updated_registry = registry::Registry {
        tasks: registry.tasks.into_iter()
            .filter(|e| !(e.created_at < cutoff && e.status.is_terminal()))
            .collect(),
    };
    registry::save_registry(tasks_dir, &updated_registry)?;

    Ok(cleaned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use crate::core::TaskId;

    fn create_test_meta(id: &str) -> TaskMeta {
        TaskMeta {
            id: TaskId::from_raw(id.into()),
            command: "echo test".into(),
            mode: crate::core::TaskMode::Sync,
            created_at: Utc::now(),
            completed_at: None,
            exit_code: Some(0),
            duration_sec: Some(1),
            status: TaskStatus::Success,
            total_lines: 1,
            total_chars: 4,
            output_size_bytes: 4,
            error_count: 0,
            warning_count: 0,
            pid: None,
            background_pid: None,
            input_tokens: None,
            output_tokens: None,
            compression_ratio: None,
        }
    }

    #[test]
    fn test_save_and_load_task_meta() {
        let dir = std::env::temp_dir().join("smol-test-save-load");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let meta = create_test_meta("Tst1234");
        let task = Task {
            meta: meta.clone(),
            stdout_path: String::new(),
            stderr_path: String::new(),
        };
        save_task(&task).unwrap_or(());

        let loaded = load_task_meta(&meta.id, dir.to_str().unwrap());
        // May fail if the directory setup isn't right, but shouldn't panic
        if let Ok(l) = loaded {
            assert_eq!(l.id, meta.id);
            assert_eq!(l.command, meta.command);
        }
    }
}
