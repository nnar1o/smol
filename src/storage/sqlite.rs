use rusqlite::{params, Connection};

use crate::core::{ErrorLine, SmolError, TaskId, TaskMeta, TaskMode, TaskStatus, WarningLine};
use chrono::{DateTime, Utc};

/// SQLite-backed storage for smol tasks.
pub struct SqliteStorage {
    conn: Connection,
}

fn parse_mode(s: &str) -> TaskMode {
    match s {
        "sync" => TaskMode::Sync,
        "background" | "bg" => TaskMode::Background,
        _ => TaskMode::Auto,
    }
}

fn parse_status(s: &str) -> TaskStatus {
    match s {
        "running" => TaskStatus::Running,
        "success" => TaskStatus::Success,
        "failed" => TaskStatus::Failed,
        "cancelled" => TaskStatus::Cancelled,
        "timeout" => TaskStatus::TimedOut,
        _ => TaskStatus::Running,
    }
}

impl SqliteStorage {
    /// Open (or create) a SQLite database at the given path.
    pub fn new(db_path: &str) -> Result<Self, SmolError> {
        let conn = Connection::open(db_path)
            .map_err(|e| SmolError::other(format!("Failed to open SQLite database: {}", e)))?;
        Ok(Self { conn })
    }

    /// Create the schema tables and indexes.
    pub fn init(&self) -> Result<(), SmolError> {
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS tasks (
                    id TEXT PRIMARY KEY,
                    command TEXT NOT NULL,
                    mode TEXT NOT NULL DEFAULT 'auto',
                    status TEXT NOT NULL DEFAULT 'running',
                    created_at TEXT NOT NULL,
                    completed_at TEXT,
                    exit_code INTEGER,
                    duration_sec INTEGER,
                    error_count INTEGER DEFAULT 0,
                    warning_count INTEGER DEFAULT 0,
                    pid INTEGER,
                    background_pid INTEGER,
                    input_tokens INTEGER,
                    output_tokens INTEGER,
                    compression_ratio REAL
                );

                CREATE TABLE IF NOT EXISTS errors (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    task_id TEXT NOT NULL REFERENCES tasks(id),
                    line_number INTEGER NOT NULL,
                    file TEXT,
                    file_line INTEGER,
                    column INTEGER,
                    content TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS warnings (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    task_id TEXT NOT NULL REFERENCES tasks(id),
                    line_number INTEGER NOT NULL,
                    file TEXT,
                    file_line INTEGER,
                    content TEXT NOT NULL
                );

                CREATE INDEX IF NOT EXISTS idx_errors_task_id ON errors(task_id);
                CREATE INDEX IF NOT EXISTS idx_warnings_task_id ON warnings(task_id);
                CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
                CREATE INDEX IF NOT EXISTS idx_tasks_created ON tasks(created_at);",
            )
            .map_err(|e| SmolError::other(format!("Failed to initialize SQLite schema: {}", e)))?;
        Ok(())
    }

    /// Insert or replace a task's metadata.
    pub fn save_task(&self, meta: &TaskMeta) -> Result<(), SmolError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO tasks
                 (id, command, mode, status, created_at, completed_at,
                  exit_code, duration_sec, error_count, warning_count,
                  pid, background_pid, input_tokens, output_tokens, compression_ratio)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![
                    meta.id.as_str(),
                    meta.command,
                    meta.mode.as_str(),
                    meta.status.as_str(),
                    meta.created_at.to_rfc3339(),
                    meta.completed_at.map(|t| t.to_rfc3339()),
                    meta.exit_code,
                    meta.duration_sec.map(|d| d as i64),
                    meta.error_count as i64,
                    meta.warning_count as i64,
                    meta.pid.map(|p| p as i64),
                    meta.background_pid.map(|p| p as i64),
                    meta.input_tokens.map(|t| t as i64),
                    meta.output_tokens.map(|t| t as i64),
                    meta.compression_ratio,
                ],
            )
            .map_err(|e| SmolError::other(format!("Failed to save task: {}", e)))?;
        Ok(())
    }

    /// Load a single task by ID.
    pub fn load_task(&self, id: &TaskId) -> Result<TaskMeta, SmolError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, command, mode, status, created_at, completed_at,
                        exit_code, duration_sec, error_count, warning_count,
                        pid, background_pid, input_tokens, output_tokens, compression_ratio
                 FROM tasks WHERE id = ?1",
            )
            .map_err(|e| SmolError::other(format!("Failed to prepare query: {}", e)))?;

        let result = stmt.query_row(params![id.as_str()], |row| {
            let mode_str: String = row.get(2)?;
            let status_str: String = row.get(3)?;

            let created_at_str: String = row.get(4)?;
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            let completed_at_str: Option<String> = row.get(5)?;
            let completed_at = completed_at_str.and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&Utc))
                    .ok()
            });

            Ok(TaskMeta {
                id: TaskId::from_raw(row.get::<_, String>(0)?),
                command: row.get(1)?,
                mode: parse_mode(&mode_str),
                status: parse_status(&status_str),
                created_at,
                completed_at,
                exit_code: row.get(6)?,
                duration_sec: row.get::<_, Option<i64>>(7)?.map(|d| d as u64),
                total_lines: 0,
                total_chars: 0,
                output_size_bytes: 0,
                error_count: row.get::<_, i64>(8)? as u32,
                warning_count: row.get::<_, i64>(9)? as u32,
                pid: row.get::<_, Option<i64>>(10)?.map(|p| p as u32),
                background_pid: row.get::<_, Option<i64>>(11)?.map(|p| p as u32),
                input_tokens: row.get::<_, Option<i64>>(12)?.map(|t| t as usize),
                output_tokens: row.get::<_, Option<i64>>(13)?.map(|t| t as usize),
                compression_ratio: row.get(14)?,
            })
        });

        match result {
            Ok(meta) => Ok(meta),
            Err(rusqlite::Error::QueryReturnedNoRows) => Err(SmolError::TaskNotFound(id.to_string())),
            Err(e) => Err(SmolError::other(format!("Failed to load task: {}", e))),
        }
    }

    /// List all tasks, optionally filtering to running ones only.
    pub fn list_tasks(&self, running_only: bool) -> Result<Vec<TaskMeta>, SmolError> {
        let (sql, status_param): (&str, Option<&str>) = if running_only {
            ("SELECT id, command, mode, status, created_at, completed_at,
                     exit_code, duration_sec, error_count, warning_count,
                     pid, background_pid, input_tokens, output_tokens, compression_ratio
              FROM tasks WHERE status = ?1 ORDER BY created_at DESC",
             Some("running"))
        } else {
            ("SELECT id, command, mode, status, created_at, completed_at,
                     exit_code, duration_sec, error_count, warning_count,
                     pid, background_pid, input_tokens, output_tokens, compression_ratio
              FROM tasks ORDER BY created_at DESC",
             None)
        };

        let mut stmt = self
            .conn
            .prepare(sql)
            .map_err(|e| SmolError::other(format!("Failed to prepare list query: {}", e)))?;

        let rows = if let Some(status) = status_param {
            stmt.query_map(params![status], row_to_meta)
        } else {
            stmt.query_map([], row_to_meta)
        }
        .map_err(|e| SmolError::other(format!("Failed to list tasks: {}", e)))?;

        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(
                row.map_err(|e| SmolError::other(format!("Failed to read task row: {}", e)))?,
            );
        }
        Ok(tasks)
    }

    /// Update a task's final status (used when a running task completes).
    pub fn update_task_status(
        &self,
        id: &TaskId,
        status: &TaskStatus,
        completed_at: &DateTime<Utc>,
        exit_code: Option<i32>,
        error_count: u32,
        warning_count: u32,
    ) -> Result<(), SmolError> {
        self.conn
            .execute(
                "UPDATE tasks SET status = ?1, completed_at = ?2, exit_code = ?3,
                                  error_count = ?4, warning_count = ?5
                 WHERE id = ?6",
                params![
                    status.as_str(),
                    completed_at.to_rfc3339(),
                    exit_code,
                    error_count as i64,
                    warning_count as i64,
                    id.as_str(),
                ],
            )
            .map_err(|e| SmolError::other(format!("Failed to update task status: {}", e)))?;
        Ok(())
    }

    /// Cancel a running task: kill its PID and mark as cancelled.
    pub fn cancel_task(&self, id: &TaskId) -> Result<(), SmolError> {
        let meta = self.load_task(id)?;
        let pid = meta
            .background_pid
            .or(meta.pid)
            .ok_or_else(|| SmolError::other("No PID found for task"))?;

        #[cfg(unix)]
        {
            std::process::Command::new("kill")
                .arg(pid.to_string())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map_err(|e| SmolError::other(format!("Failed to kill process: {}", e)))?;
        }

        self.conn
            .execute(
                "UPDATE tasks SET status = ?1 WHERE id = ?2",
                params!["cancelled", id.as_str()],
            )
            .map_err(|e| SmolError::other(format!("Failed to cancel task: {}", e)))?;

        Ok(())
    }

    /// Delete tasks older than `secs` that are not running.
    /// Returns the number of tasks deleted.
    pub fn clean_older_than(&self, secs: u64) -> Result<u64, SmolError> {
        let cutoff = Utc::now() - chrono::Duration::seconds(secs as i64);
        let cutoff_str = cutoff.to_rfc3339();

        // Delete matching errors/warnings first (foreign key not enforced by default)
        self.conn
            .execute(
                "DELETE FROM errors WHERE task_id IN (SELECT id FROM tasks WHERE created_at < ?1 AND status != 'running')",
                params![cutoff_str],
            )
            .ok();
        self.conn
            .execute(
                "DELETE FROM warnings WHERE task_id IN (SELECT id FROM tasks WHERE created_at < ?1 AND status != 'running')",
                params![cutoff_str],
            )
            .ok();

        let deleted = self
            .conn
            .execute(
                "DELETE FROM tasks WHERE created_at < ?1 AND status != 'running'",
                params![cutoff_str],
            )
            .map_err(|e| SmolError::other(format!("Failed to clean old tasks: {}", e)))?;

        Ok(deleted as u64)
    }

    /// Search tasks by command (LIKE query).
    pub fn search_tasks(&self, query: &str) -> Result<Vec<TaskMeta>, SmolError> {
        let pattern = format!("%{}%", query);
        let sql =
            "SELECT id, command, mode, status, created_at, completed_at,
                    exit_code, duration_sec, error_count, warning_count,
                    pid, background_pid, input_tokens, output_tokens, compression_ratio
             FROM tasks WHERE command LIKE ?1 ORDER BY created_at DESC";

        let mut stmt = self
            .conn
            .prepare(sql)
            .map_err(|e| SmolError::other(format!("Failed to prepare search query: {}", e)))?;

        let rows = stmt
            .query_map(params![pattern], row_to_meta)
            .map_err(|e| SmolError::other(format!("Failed to search tasks: {}", e)))?;

        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(
                row.map_err(|e| SmolError::other(format!("Failed to read task row: {}", e)))?,
            );
        }
        Ok(tasks)
    }

    // ── Error storage ───────────────────────────────────────────

    /// Store error lines for a task (replaces all previous errors for that task).
    pub fn save_errors(&self, task_id: &TaskId, errors: &[ErrorLine]) -> Result<(), SmolError> {
        self.conn
            .execute("DELETE FROM errors WHERE task_id = ?1", params![task_id.as_str()])
            .ok();

        let mut stmt = self
            .conn
            .prepare(
                "INSERT INTO errors (task_id, line_number, file, file_line, column, content)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )
            .map_err(|e| SmolError::other(format!("Failed to prepare error insert: {}", e)))?;

        for err in errors {
            stmt.execute(params![
                task_id.as_str(),
                err.line_num as i64,
                err.file,
                err.file_line.map(|l| l as i64),
                err.column.map(|c| c as i64),
                err.content,
            ])
            .map_err(|e| SmolError::other(format!("Failed to insert error: {}", e)))?;
        }

        Ok(())
    }

    /// Store warning lines for a task (replaces all previous warnings for that task).
    pub fn save_warnings(&self, task_id: &TaskId, warnings: &[WarningLine]) -> Result<(), SmolError> {
        self.conn
            .execute("DELETE FROM warnings WHERE task_id = ?1", params![task_id.as_str()])
            .ok();

        let mut stmt = self
            .conn
            .prepare(
                "INSERT INTO warnings (task_id, line_number, file, file_line, content)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )
            .map_err(|e| SmolError::other(format!("Failed to prepare warning insert: {}", e)))?;

        for warn in warnings {
            stmt.execute(params![
                task_id.as_str(),
                warn.line_num as i64,
                warn.file,
                warn.file_line.map(|l| l as i64),
                warn.content,
            ])
            .map_err(|e| SmolError::other(format!("Failed to insert warning: {}", e)))?;
        }

        Ok(())
    }

    /// Load error lines for a task.
    pub fn load_errors(&self, task_id: &TaskId) -> Result<Vec<ErrorLine>, SmolError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT line_number, file, file_line, column, content
                 FROM errors WHERE task_id = ?1 ORDER BY line_number",
            )
            .map_err(|e| SmolError::other(format!("Failed to prepare error load: {}", e)))?;

        let rows = stmt
            .query_map(params![task_id.as_str()], |row| {
                Ok(ErrorLine {
                    line_num: row.get::<_, i64>(0)? as u64,
                    content: row.get(4)?,
                    file: row.get(1)?,
                    file_line: row.get::<_, Option<i64>>(2)?.map(|l| l as u64),
                    column: row.get::<_, Option<i64>>(3)?.map(|c| c as u64),
                })
            })
            .map_err(|e| SmolError::other(format!("Failed to query errors: {}", e)))?;

        let mut errors = Vec::new();
        for row in rows {
            errors.push(row.map_err(|e| SmolError::other(format!("Failed to read error row: {}", e)))?);
        }
        Ok(errors)
    }

    /// Load warning lines for a task.
    pub fn load_warnings(&self, task_id: &TaskId) -> Result<Vec<WarningLine>, SmolError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT line_number, file, file_line, content
                 FROM warnings WHERE task_id = ?1 ORDER BY line_number",
            )
            .map_err(|e| SmolError::other(format!("Failed to prepare warning load: {}", e)))?;

        let rows = stmt
            .query_map(params![task_id.as_str()], |row| {
                Ok(WarningLine {
                    line_num: row.get::<_, i64>(0)? as u64,
                    content: row.get(3)?,
                    file: row.get(1)?,
                    file_line: row.get::<_, Option<i64>>(2)?.map(|l| l as u64),
                })
            })
            .map_err(|e| SmolError::other(format!("Failed to query warnings: {}", e)))?;

        let mut warnings = Vec::new();
        for row in rows {
            warnings.push(row.map_err(|e| SmolError::other(format!("Failed to read warning row: {}", e)))?);
        }
        Ok(warnings)
    }
}

/// Helper to map a SQLite row to a TaskMeta.
fn row_to_meta(row: &rusqlite::Row) -> rusqlite::Result<TaskMeta> {
    let mode_str: String = row.get(2)?;
    let status_str: String = row.get(3)?;

    let created_at_str: String = row.get(4)?;
    let created_at = DateTime::parse_from_rfc3339(&created_at_str)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());

    let completed_at_str: Option<String> = row.get(5)?;
    let completed_at = completed_at_str.and_then(|s| {
        DateTime::parse_from_rfc3339(&s)
            .map(|dt| dt.with_timezone(&Utc))
            .ok()
    });

    Ok(TaskMeta {
        id: TaskId::from_raw(row.get::<_, String>(0)?),
        command: row.get(1)?,
        mode: parse_mode(&mode_str),
        status: parse_status(&status_str),
        created_at,
        completed_at,
        exit_code: row.get(6)?,
        duration_sec: row.get::<_, Option<i64>>(7)?.map(|d| d as u64),
        total_lines: 0,
        total_chars: 0,
        output_size_bytes: 0,
        error_count: row.get::<_, i64>(8)? as u32,
        warning_count: row.get::<_, i64>(9)? as u32,
        pid: row.get::<_, Option<i64>>(10)?.map(|p| p as u32),
        background_pid: row.get::<_, Option<i64>>(11)?.map(|p| p as u32),
        input_tokens: row.get::<_, Option<i64>>(12)?.map(|t| t as usize),
        output_tokens: row.get::<_, Option<i64>>(13)?.map(|t| t as usize),
        compression_ratio: row.get(14)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::TaskId;
    use crate::core::TaskMode;

    fn create_test_db() -> SqliteStorage {
        let storage = SqliteStorage::new(":memory:").unwrap();
        storage.init().unwrap();
        storage
    }

    fn create_test_meta(id: &str) -> TaskMeta {
        TaskMeta {
            id: TaskId::from_raw(id.to_string()),
            command: "echo hello".to_string(),
            mode: TaskMode::Auto,
            created_at: Utc::now(),
            completed_at: None,
            exit_code: Some(0),
            duration_sec: Some(1),
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
        }
    }

    #[test]
    fn test_init_creates_tables() {
        let storage = SqliteStorage::new(":memory:").unwrap();
        storage.init().unwrap();

        // Verify tables exist
        let count: i64 = storage
            .conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('tasks', 'errors', 'warnings')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_save_and_load_task() {
        let storage = create_test_db();
        let meta = create_test_meta("SvLd1234");
        storage.save_task(&meta).unwrap();

        let loaded = storage.load_task(&meta.id).unwrap();
        assert_eq!(loaded.id.as_str(), meta.id.as_str());
        assert_eq!(loaded.command, meta.command);
        assert_eq!(loaded.status, meta.status);
        assert_eq!(loaded.mode, meta.mode);
    }

    #[test]
    fn test_load_nonexistent_task() {
        let storage = create_test_db();
        let id = TaskId::from_raw("NoSuchId".to_string());
        let result = storage.load_task(&id);
        assert!(result.is_err());
        match result {
            Err(SmolError::TaskNotFound(_)) => {}
            _ => panic!("Expected TaskNotFound error"),
        }
    }

    #[test]
    fn test_list_tasks_all() {
        let storage = create_test_db();
        let meta1 = create_test_meta("Tsk00001");
        let meta2 = create_test_meta("Tsk00002");
        storage.save_task(&meta1).unwrap();
        storage.save_task(&meta2).unwrap();

        let tasks = storage.list_tasks(false).unwrap();
        assert_eq!(tasks.len(), 2);
    }

    #[test]
    fn test_list_tasks_running_only() {
        let storage = create_test_db();
        let mut meta1 = create_test_meta("Tsk00001");
        meta1.status = TaskStatus::Running;
        let meta2 = create_test_meta("Tsk00002");
        storage.save_task(&meta1).unwrap();
        storage.save_task(&meta2).unwrap();

        let tasks = storage.list_tasks(true).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id.as_str(), "Tsk00001");
    }

    #[test]
    fn test_update_task_status() {
        let storage = create_test_db();
        let mut meta = create_test_meta("Updt1234");
        meta.status = TaskStatus::Running;
        storage.save_task(&meta).unwrap();

        let now = Utc::now();
        storage
            .update_task_status(&meta.id, &TaskStatus::Success, &now, Some(0), 0, 0)
            .unwrap();

        let loaded = storage.load_task(&meta.id).unwrap();
        assert_eq!(loaded.status, TaskStatus::Success);
        assert!(loaded.completed_at.is_some());
    }

    #[test]
    fn test_search_tasks() {
        let storage = create_test_db();
        let mut meta1 = create_test_meta("Srch0001");
        meta1.command = "npm build".to_string();
        let mut meta2 = create_test_meta("Srch0002");
        meta2.command = "cargo test".to_string();
        storage.save_task(&meta1).unwrap();
        storage.save_task(&meta2).unwrap();

        let results = storage.search_tasks("npm").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].command, "npm build");
    }

    #[test]
    fn test_clean_older_than() {
        let storage = create_test_db();
        let mut meta = create_test_meta("OldT1234");
        meta.status = TaskStatus::Success;
        // Set created_at to 100 seconds ago
        meta.created_at = Utc::now() - chrono::Duration::seconds(100);
        storage.save_task(&meta).unwrap();

        let deleted = storage.clean_older_than(50).unwrap();
        assert_eq!(deleted, 1);

        let tasks = storage.list_tasks(false).unwrap();
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_save_and_load_errors() {
        let storage = create_test_db();
        let meta = create_test_meta("ErrT1234");
        storage.save_task(&meta).unwrap();

        let errors = vec![
            ErrorLine {
                line_num: 10,
                content: "undefined variable".to_string(),
                file: Some("src/main.rs".to_string()),
                file_line: Some(42),
                column: Some(5),
            },
            ErrorLine {
                line_num: 20,
                content: "type mismatch".to_string(),
                file: None,
                file_line: None,
                column: None,
            },
        ];

        storage.save_errors(&meta.id, &errors).unwrap();
        let loaded = storage.load_errors(&meta.id).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].content, "undefined variable");
        assert_eq!(loaded[0].file.as_deref(), Some("src/main.rs"));
        assert_eq!(loaded[0].column, Some(5));
        assert_eq!(loaded[1].content, "type mismatch");
    }

    #[test]
    fn test_save_and_load_warnings() {
        let storage = create_test_db();
        let meta = create_test_meta("WrnT1234");
        storage.save_task(&meta).unwrap();

        let warnings = vec![
            WarningLine {
                line_num: 5,
                content: "unused import".to_string(),
                file: Some("src/lib.rs".to_string()),
                file_line: Some(3),
            },
        ];

        storage.save_warnings(&meta.id, &warnings).unwrap();
        let loaded = storage.load_warnings(&meta.id).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].content, "unused import");
    }

    #[test]
    fn test_save_errors_replaces_old() {
        let storage = create_test_db();
        let meta = create_test_meta("Rplc1234");
        storage.save_task(&meta).unwrap();

        let old_errors = vec![ErrorLine {
            line_num: 1,
            content: "old error".to_string(),
            file: None,
            file_line: None,
            column: None,
        }];
        storage.save_errors(&meta.id, &old_errors).unwrap();

        let new_errors = vec![ErrorLine {
            line_num: 2,
            content: "new error".to_string(),
            file: None,
            file_line: None,
            column: None,
        }];
        storage.save_errors(&meta.id, &new_errors).unwrap();

        let loaded = storage.load_errors(&meta.id).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].content, "new error");
    }
}
