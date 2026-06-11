use crate::core::{SmolError, TaskId, TaskMeta};

/// Export format
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExportedTask {
    pub meta: TaskMeta,
    pub stdout: String,
    pub stderr: String,
}

/// Export a single task to the given format.
pub fn export_task(
    tasks_dir: &str,
    task_id: &TaskId,
    format: &str,
) -> Result<String, SmolError> {
    let task_dir = std::path::Path::new(tasks_dir).join(task_id.as_str());

    let meta_path = task_dir.join("meta.toml");
    let meta_content = std::fs::read_to_string(&meta_path)?;
    let meta: TaskMeta = toml::from_str(&meta_content)?;

    let stdout = std::fs::read_to_string(task_dir.join("output.log")).unwrap_or_default();
    let stderr = std::fs::read_to_string(task_dir.join("error.log")).unwrap_or_default();

    match format {
        "json" => {
            let exported = ExportedTask { meta, stdout, stderr };
            serde_json::to_string_pretty(&exported)
                .map_err(|e| SmolError::other(format!("JSON error: {}", e)))
        }
        "markdown" | "md" => {
            let mut md = String::new();
            md.push_str(&format!("# Task: {}\n\n", meta.id));
            md.push_str(&format!("**Command**: `{}`\n\n", meta.command));
            md.push_str(&format!("**Status**: {}\n\n", meta.status.as_str()));
            md.push_str(&format!("**Created**: {}\n\n", meta.created_at));
            if let Some(t) = meta.completed_at {
                md.push_str(&format!("**Completed**: {}\n\n", t));
            }
            md.push_str(&format!(
                "**Errors**: {}  **Warnings**: {}\n\n",
                meta.error_count, meta.warning_count
            ));

            if !stdout.is_empty() {
                md.push_str("## stdout\n\n```\n");
                md.push_str(&stdout);
                md.push_str("\n```\n\n");
            }
            if !stderr.is_empty() {
                md.push_str("## stderr\n\n```\n");
                md.push_str(&stderr);
                md.push_str("\n```\n");
            }
            Ok(md)
        }
        _ => Err(SmolError::config(format!(
            "Unknown format: {}. Use 'json' or 'markdown'",
            format
        ))),
    }
}

/// Export all tasks as vector of formatted strings.
pub fn export_all(tasks_dir: &str, format: &str) -> Result<Vec<String>, SmolError> {
    let tasks_path = std::path::Path::new(tasks_dir);
    if !tasks_path.exists() {
        return Ok(Vec::new());
    }

    let mut exports = Vec::new();
    for entry in std::fs::read_dir(tasks_path)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if let Ok(id) = name.parse::<TaskId>() {
            if let Ok(export) = export_task(tasks_dir, &id, format) {
                exports.push(export);
            }
        }
    }
    Ok(exports)
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
    fn test_export_task_json() {
        let dir = tempfile::tempdir().unwrap();
        let tasks_dir = dir.path().join("tasks");
        let id = TaskId::from_raw("ExpTst01".into());
        let task_dir = tasks_dir.join(id.as_str());

        create_test_task(&task_dir, &id, "echo test", "hello", "warning: stuff");

        let result = export_task(tasks_dir.to_str().unwrap(), &id, "json").unwrap();
        assert!(result.contains(r#""meta""#));
        assert!(result.contains(r#""stdout""#));
        assert!(result.contains("hello"));
    }

    #[test]
    fn test_export_task_markdown() {
        let dir = tempfile::tempdir().unwrap();
        let tasks_dir = dir.path().join("tasks");
        let id = TaskId::from_raw("ExpTst02".into());
        let task_dir = tasks_dir.join(id.as_str());

        create_test_task(&task_dir, &id, "cargo build", "compiling...", "");

        let result = export_task(tasks_dir.to_str().unwrap(), &id, "markdown").unwrap();
        assert!(result.contains("# Task:"));
        assert!(result.contains("**Command**:"));
        assert!(result.contains("cargo build"));
        assert!(result.contains("## stdout"));
    }

    #[test]
    fn test_export_task_invalid_format() {
        let dir = tempfile::tempdir().unwrap();
        let tasks_dir = dir.path().join("tasks");
        let id = TaskId::from_raw("ExpTst03".into());
        let task_dir = tasks_dir.join(id.as_str());

        create_test_task(&task_dir, &id, "echo hi", "", "");

        let result = export_task(tasks_dir.to_str().unwrap(), &id, "xml");
        assert!(result.is_err());
    }

    #[test]
    fn test_export_all() {
        let dir = tempfile::tempdir().unwrap();
        let tasks_dir = dir.path().join("tasks");

        let id1 = TaskId::from_raw("ExpAll01".into());
        let id2 = TaskId::from_raw("ExpAll02".into());

        create_test_task(&tasks_dir.join(id1.as_str()), &id1, "cmd1", "output1", "");
        create_test_task(&tasks_dir.join(id2.as_str()), &id2, "cmd2", "output2", "");

        let results = export_all(tasks_dir.to_str().unwrap(), "json").unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_export_no_tasks_dir() {
        let results = export_all("/tmp/nonexistent_smol_export_dir", "json").unwrap();
        assert!(results.is_empty());
    }
}
