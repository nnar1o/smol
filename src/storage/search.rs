use crate::core::{SmolError, TaskId, TaskMeta};

/// Search results from task logs
pub struct SearchResult {
    pub task_id: TaskId,
    pub command: String,
    pub line_number: usize,
    pub line_content: String,
    pub created_at: String,
}

/// Search through all task output logs for a query string.
/// Performs case-insensitive matching against both stdout and stderr.
pub fn search_in_logs(
    tasks_dir: &str,
    query: &str,
    max_results: usize,
) -> Result<Vec<SearchResult>, SmolError> {
    let mut results = Vec::new();
    let tasks_path = std::path::Path::new(tasks_dir);

    if !tasks_path.exists() {
        return Ok(results);
    }

    let query_lower = query.to_lowercase();

    for entry in std::fs::read_dir(tasks_path)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let task_id_str = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        // Parse task ID — skip directories that aren't valid task dirs
        let task_id = match task_id_str.parse::<TaskId>() {
            Ok(id) => id,
            Err(_) => continue,
        };

        // Read meta
        let meta_path = path.join("meta.toml");
        let meta = match std::fs::read_to_string(&meta_path) {
            Ok(content) => match toml::from_str::<TaskMeta>(&content) {
                Ok(m) => m,
                Err(_) => continue,
            },
            Err(_) => continue,
        };

        // Search in output.log (stdout)
        let output_path = path.join("output.log");
        if let Ok(content) = std::fs::read_to_string(&output_path) {
            for (i, line) in content.lines().enumerate() {
                if line.to_lowercase().contains(&query_lower) {
                    results.push(SearchResult {
                        task_id: task_id.clone(),
                        command: meta.command.clone(),
                        line_number: i + 1,
                        line_content: line.to_string(),
                        created_at: meta.created_at.to_rfc3339(),
                    });
                    if results.len() >= max_results {
                        return Ok(results);
                    }
                }
            }
        }

        // Search in error.log (stderr)
        let error_path = path.join("error.log");
        if let Ok(content) = std::fs::read_to_string(&error_path) {
            for (i, line) in content.lines().enumerate() {
                if line.to_lowercase().contains(&query_lower) {
                    results.push(SearchResult {
                        task_id: task_id.clone(),
                        command: meta.command.clone(),
                        line_number: i + 1,
                        line_content: format!("[stderr] {}", line),
                        created_at: meta.created_at.to_rfc3339(),
                    });
                    if results.len() >= max_results {
                        return Ok(results);
                    }
                }
            }
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{TaskId, TaskMeta, TaskMode, TaskStatus};
    use chrono::Utc;

    fn create_test_task(task_dir: &std::path::Path, id: &TaskId, command: &str, stdout: &str, stderr: &str) {
        std::fs::create_dir_all(task_dir).unwrap();

        let meta = TaskMeta {
            id: id.clone(),
            command: command.to_string(),
            mode: TaskMode::Sync,
            created_at: Utc::now(),
            completed_at: None,
            exit_code: Some(0),
            duration_sec: None,
            status: TaskStatus::Success,
            total_lines: 0,
            total_chars: 0,
            output_size_bytes: 0,
            error_count: 0,
            warning_count: 0,
            pid: None,
            background_pid: None,
            input_tokens: None,
            output_tokens: None,
            compression_ratio: None,
            test_total: None,
            test_passed: None,
            test_failed: None,
            test_errors: None,
            test_skipped: None,
        };

        let meta_content = toml::to_string_pretty(&meta).unwrap();
        std::fs::write(task_dir.join("meta.toml"), meta_content).unwrap();

        std::fs::write(task_dir.join("output.log"), stdout).unwrap();
        std::fs::write(task_dir.join("error.log"), stderr).unwrap();
    }

    #[test]
    fn test_search_in_logs_found() {
        let dir = tempfile::tempdir().unwrap();
        let tasks_dir = dir.path().join("tasks");
        let id = TaskId::from_raw("TestId01".into());
        let task_dir = tasks_dir.join(id.as_str());

        create_test_task(
            &task_dir,
            &id,
            "echo hello",
            "hello world\nhow are you",
            "",
        );

        let results = search_in_logs(tasks_dir.to_str().unwrap(), "hello", 10).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].task_id, id);
        assert_eq!(results[0].command, "echo hello");
        assert_eq!(results[0].line_content, "hello world");
    }

    #[test]
    fn test_search_in_logs_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let tasks_dir = dir.path().join("tasks");
        let id = TaskId::from_raw("TestId02".into());
        let task_dir = tasks_dir.join(id.as_str());

        create_test_task(&task_dir, &id, "echo hi", "hello world", "");

        let results = search_in_logs(tasks_dir.to_str().unwrap(), "nonexistent", 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_in_logs_max_results() {
        let dir = tempfile::tempdir().unwrap();
        let tasks_dir = dir.path().join("tasks");

        // Create two tasks, each with matching lines
        let id1 = TaskId::from_raw("TestId03".into());
        let id2 = TaskId::from_raw("TestId04".into());

        create_test_task(&tasks_dir.join(id1.as_str()), &id1, "cmd1", "match one\nmatch two", "");
        create_test_task(&tasks_dir.join(id2.as_str()), &id2, "cmd2", "match three", "");

        let results = search_in_logs(tasks_dir.to_str().unwrap(), "match", 2).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_search_in_logs_no_tasks_dir() {
        let results = search_in_logs("/tmp/nonexistent_smol_tasks_dir_xyz", "test", 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_in_logs_case_insensitive() {
        let dir = tempfile::tempdir().unwrap();
        let tasks_dir = dir.path().join("tasks");
        let id = TaskId::from_raw("TestId05".into());

        create_test_task(&tasks_dir.join(id.as_str()), &id, "build", "ERROR: compilation failed", "");

        let results = search_in_logs(tasks_dir.to_str().unwrap(), "error", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].line_content.contains("ERROR"));
    }
}
