use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, Write};

use crate::config::{self, GlobalConfig};
use crate::core::{ParserConfig, SmolError, SummaryStatus, Task, TaskId, TaskMeta, TaskMode, TaskStatus};
use crate::exec;
use crate::mcp::protocol::{self, JsonRpcRequest, JsonRpcResponse};
use crate::parse;
use crate::storage;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Run the MCP server loop: read JSON-RPC 2.0 requests from stdin,
/// dispatch them, and write responses to stdout.
pub fn run() {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                let err = protocol::make_error(
                    None,
                    protocol::INTERNAL_ERROR,
                    format!("IO error reading stdin: {}", e),
                    None,
                );
                let _ = writeln!(stdout, "{}", serde_json::to_string(&err).unwrap());
                let _ = stdout.flush();
                continue;
            }
        };

        let trimmed = line.trim().to_string();
        if trimmed.is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(&trimmed) {
            Ok(req) => req,
            Err(e) => {
                let err = protocol::make_error(
                    None,
                    protocol::PARSE_ERROR,
                    format!("Parse error: {}", e),
                    None,
                );
                let _ = writeln!(stdout, "{}", serde_json::to_string(&err).unwrap());
                let _ = stdout.flush();
                continue;
            }
        };

        let is_notification = request.id.is_none() && request.method != "initialize";
        let (response, should_exit) = handle_request(&request);

        // Notifications (requests without an id) do not get a response.
        if is_notification && !should_exit {
            continue;
        }

        if !is_notification {
            let resp_json = serde_json::to_string(&response).unwrap();
            let _ = writeln!(stdout, "{}", resp_json);
            let _ = stdout.flush();
        }

        if should_exit {
            break;
        }
    }
}

// ---------------------------------------------------------------------------
// Request dispatching
// ---------------------------------------------------------------------------

fn handle_request(request: &JsonRpcRequest) -> (JsonRpcResponse, bool) {
    match request.method.as_str() {
        "initialize" => (handle_initialize(request), false),
        "tools/list" => (handle_tools_list(request), false),
        "tools/call" => (handle_tools_call(request), false),
        "shutdown" => {
            // Client asked to shut down — return success but stay alive
            // until the subsequent "exit" request per MCP spec.
            (protocol::make_response(request.id.clone(), json!({})), false)
        }
        "exit" => {
            // Follow the MCP lifecycle: respond then terminate.
            (protocol::make_response(request.id.clone(), json!({})), true)
        }
        _ => {
            // Ignore notifications and $-prefixed methods gracefully.
            if request.method.starts_with("notifications/") || request.method.starts_with("$/") {
                return (protocol::make_response(request.id.clone(), json!({})), false);
            }
            (protocol::make_error(
                request.id.clone(),
                protocol::METHOD_NOT_FOUND,
                format!("Method not found: {}", request.method),
                None,
            ), false)
        }
    }
}

// ---------------------------------------------------------------------------
// MCP lifecycle
// ---------------------------------------------------------------------------

fn handle_initialize(request: &JsonRpcRequest) -> JsonRpcResponse {
    protocol::make_response(
        request.id.clone(),
        json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "smol",
                "version": env!("CARGO_PKG_VERSION")
            }
        }),
    )
}

// ---------------------------------------------------------------------------
// Tool definitions
// ---------------------------------------------------------------------------

fn handle_tools_list(request: &JsonRpcRequest) -> JsonRpcResponse {
    let tools = json!([
        {
            "name": "smol_run",
            "description": "Execute a command and return a summarized analysis of the output. Use 'sync' mode to wait for completion (best for fast commands), 'auto' mode to wait briefly then fall back to background if the command takes too long (default), or 'bg' mode to run immediately in the background. Returns a summary with task_id, exit_code, and status. For background tasks, use smol_status to check progress and smol_log to retrieve output.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "command": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Command and arguments to execute"
                    },
                    "mode": {
                        "type": "string",
                        "enum": ["sync", "auto", "bg", "background"],
                        "description": "Execution mode: sync (wait for completion), auto (wait briefly then background), bg (immediate background)",
                        "default": "auto"
                    }
                },
                "required": ["command"]
            }
        },
        {
            "name": "smol_status",
            "description": "Get the status and metadata of a task by its task_id. Use 'last' as the task_id to query the most recently created task. Returns full task metadata including status (running/success/failed/cancelled/timed_out), exit_code, duration, error_count, warning_count, and test results if available.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "Task ID, or 'last' for the most recent task"
                    }
                },
                "required": ["task_id"]
            }
        },
        {
            "name": "smol_log",
            "description": "Retrieve the log output of a task. By default returns the full output log. Set 'errors' to true to filter for error lines only, or 'warnings' to true for warning lines only. Set 'stats' to true to return task metadata as JSON instead of log text. Use 'tail' to return only the last N lines and 'max_chars' to limit total output size.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "Task ID, or 'last' for the most recent task"
                    },
                    "errors": {
                        "type": "boolean",
                        "description": "Show only lines containing errors",
                        "default": false
                    },
                    "warnings": {
                        "type": "boolean",
                        "description": "Show only lines containing warnings",
                        "default": false
                    },
                    "stats": {
                        "type": "boolean",
                        "description": "Return task metadata as JSON instead of log text",
                        "default": false
                    },
                    "tail": {
                        "type": "integer",
                        "description": "Return only the last N lines of the log. 0 means no limit.",
                        "default": 0
                    },
                    "max_chars": {
                        "type": "integer",
                        "description": "Truncate output to at most this many characters. 0 means no limit.",
                        "default": 0
                    }
                },
                "required": ["task_id"]
            }
        },
        {
            "name": "smol_list",
            "description": "List all stored tasks, optionally filtered to show only running tasks. Set 'running' to true to see only currently executing tasks. Returns an array of task metadata objects.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "running": {
                        "type": "boolean",
                        "description": "If true, show only running tasks",
                        "default": false
                    }
                }
            }
        },
        {
            "name": "smol_cancel",
            "description": "Cancel a running task by sending SIGTERM to its process. The task_id must correspond to a currently running background task. Returns success status after sending the signal. Note: the process may take a moment to terminate after cancellation.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "Task ID, or 'last' for the most recent task"
                    }
                },
                "required": ["task_id"]
            }
        },
        {
            "name": "smol_wait",
            "description": "Wait for a background task to complete. Polls the task status at intervals until the task reaches a terminal state (success/failed/cancelled/timed_out) or the timeout is reached. Returns the final task metadata if completed, or the current metadata if the timeout expired.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "Task ID, or 'last' for the most recent task"
                    },
                    "timeout": {
                        "type": "integer",
                        "description": "Maximum seconds to wait (default 60)",
                        "default": 60
                    },
                    "interval": {
                        "type": "integer",
                        "description": "Seconds between status polls (default 2)",
                        "default": 2
                    }
                },
                "required": ["task_id"]
            }
        },
        {
            "name": "smol_clean",
            "description": "Remove old completed, failed, or cancelled tasks from storage to free disk space. The 'older' parameter accepts duration strings like '24h' (hours), '7d' (days), '30m' (minutes), '3600s' (seconds), or plain seconds. Only non-running tasks are removed. Default is '24h'.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "older": {
                        "type": "string",
                        "description": "Age threshold such as '24h', '7d', '30m', or plain seconds",
                        "default": "24h"
                    }
                }
            }
        }
    ]);

    protocol::make_response(request.id.clone(), json!({ "tools": tools }))
}

// ---------------------------------------------------------------------------
// Tool call dispatch
// ---------------------------------------------------------------------------

fn handle_tools_call(request: &JsonRpcRequest) -> JsonRpcResponse {
    let params = match &request.params {
        Some(p) => p,
        None => {
            return protocol::make_error(
                request.id.clone(),
                protocol::INVALID_PARAMS,
                "Missing params".into(),
                None,
            );
        }
    };

    let name = match params.get("name").and_then(|v| v.as_str()) {
        Some(n) => n,
        None => {
            return protocol::make_error(
                request.id.clone(),
                protocol::INVALID_PARAMS,
                "Missing 'name' in params".into(),
                None,
            );
        }
    };

    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

    let result = match name {
        "smol_run" => handle_smol_run(&arguments),
        "smol_status" => handle_smol_status(&arguments),
        "smol_log" => handle_smol_log(&arguments),
        "smol_list" => handle_smol_list(&arguments),
        "smol_cancel" => handle_smol_cancel(&arguments),
        "smol_clean" => handle_smol_clean(&arguments),
        "smol_wait" => handle_smol_wait(&arguments),
        _ => {
            return protocol::make_error(
                request.id.clone(),
                protocol::METHOD_NOT_FOUND,
                format!("Unknown tool: {}", name),
                None,
            );
        }
    };

    match result {
        Ok(value) => protocol::make_response(request.id.clone(), value),
        Err(e) => protocol::make_error(request.id.clone(), e.code, e.message, e.data),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn get_tasks_dir(cfg: &GlobalConfig) -> String {
    if cfg.tasks_dir.is_empty() {
        storage::paths::tasks_dir()
    } else {
        cfg.tasks_dir.clone()
    }
}

fn get_parsers_dir(cfg: &GlobalConfig) -> String {
    if cfg.parsers_dir.is_empty() {
        storage::paths::parsers_dir()
    } else {
        cfg.parsers_dir.clone()
    }
}

fn resolve_task_id(task_id: &str, tasks_dir: &str) -> Result<TaskId, SmolError> {
    if task_id == "last" {
        let tasks = storage::list_tasks(tasks_dir, None)?;
        tasks
            .into_iter()
            .next()
            .map(|t| t.id)
            .ok_or_else(|| SmolError::TaskNotFound("no tasks found".into()))
    } else {
        task_id
            .parse::<TaskId>()
            .map_err(|e| SmolError::InvalidTaskId(e))
    }
}

fn parse_duration(s: &str) -> Result<u64, SmolError> {
    if let Some(num) = s.strip_suffix('s') {
        let n: u64 = num
            .parse()
            .map_err(|_| SmolError::config("Invalid duration"))?;
        Ok(n)
    } else if let Some(num) = s.strip_suffix('h') {
        let n: u64 = num
            .parse()
            .map_err(|_| SmolError::config("Invalid duration"))?;
        Ok(n * 3600)
    } else if let Some(num) = s.strip_suffix('d') {
        let n: u64 = num
            .parse()
            .map_err(|_| SmolError::config("Invalid duration"))?;
        Ok(n * 86400)
    } else if let Some(num) = s.strip_suffix('m') {
        let n: u64 = num
            .parse()
            .map_err(|_| SmolError::config("Invalid duration"))?;
        Ok(n * 60)
    } else {
        let n: u64 = s.parse().map_err(|_| {
            SmolError::config("Invalid duration. Use e.g. '24h', '7d', '3600'")
        })?;
        Ok(n)
    }
}

/// Create a TaskMeta for a completed (sync / auto-fast) execution.
fn make_completed_meta(
    id: TaskId,
    command: &str,
    mode: TaskMode,
    result: &exec::RunResult,
    summary: &crate::core::Summary,
) -> TaskMeta {
    let now = chrono::Utc::now();
    TaskMeta {
        id,
        command: command.to_string(),
        mode,
        created_at: now,
        completed_at: Some(now),
        exit_code: result.exit_code,
        duration_sec: Some(result.duration_sec),
        status: summary_status_to_task_status(summary),
        total_lines: summary.total_lines,
        total_chars: summary.total_chars,
        output_size_bytes: (result.stdout.len() + result.stderr.len()) as u64,
        error_count: summary.error_count,
        warning_count: summary.warning_count,
        pid: result.pid,
        background_pid: None,
        input_tokens: None,
        output_tokens: None,
        compression_ratio: None,
        test_total: summary.tests.as_ref().map(|t| t.total),
        test_passed: summary.tests.as_ref().map(|t| t.passed),
        test_failed: summary.tests.as_ref().map(|t| t.failed),
        test_errors: summary.tests.as_ref().map(|t| t.errors),
        test_skipped: summary.tests.as_ref().map(|t| t.skipped),
    }
}

fn summary_status_to_task_status(summary: &crate::core::Summary) -> TaskStatus {
    match summary.status {
        SummaryStatus::Success => TaskStatus::Success,
        SummaryStatus::Failure => TaskStatus::Failed,
        SummaryStatus::Unknown => {
            if summary.error_count > 0 {
                TaskStatus::Failed
            } else {
                TaskStatus::Success
            }
        }
    }
}

fn write_task_output(tasks_dir: &str, task_id: &TaskId, stdout: &str, stderr: &str) {
    let task_dir = std::path::Path::new(tasks_dir).join(task_id.as_str());
    let _ = std::fs::create_dir_all(&task_dir);
    let _ = std::fs::write(task_dir.join("output.log"), stdout);
    let _ = std::fs::write(task_dir.join("error.log"), stderr);
}

// ---------------------------------------------------------------------------
// Tool handlers
// ---------------------------------------------------------------------------

fn handle_smol_run(args: &Value) -> Result<Value, protocol::JsonRpcErrorObj> {
    let command: Vec<String> = match args.get("command") {
        Some(Value::Array(arr)) => arr.iter().filter_map(|v| v.as_str().map(String::from)).collect(),
        _ => {
            return Err(protocol::JsonRpcErrorObj::new(
                protocol::INVALID_PARAMS,
                "Missing or invalid 'command' parameter (expected array of strings)",
            ));
        }
    };

    if command.is_empty() {
        return Err(protocol::JsonRpcErrorObj::new(
            protocol::INVALID_PARAMS,
            "Command array must not be empty",
        ));
    }

    let mode_str = args
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("auto");

    let cfg = config::load_global_config().map_err(|e| {
        protocol::JsonRpcErrorObj::new(protocol::INTERNAL_ERROR, format!("Failed to load config: {}", e))
    })?;
    let tasks_dir = get_tasks_dir(&cfg);
    storage::init(&tasks_dir).map_err(|e| {
        protocol::JsonRpcErrorObj::new(protocol::INTERNAL_ERROR, format!("Failed to init storage: {}", e))
    })?;
    let parsers_dir = get_parsers_dir(&cfg);
    let parsers = config::load_all_parsers(&parsers_dir).map_err(|e| {
        protocol::JsonRpcErrorObj::new(protocol::INTERNAL_ERROR, format!("Failed to load parsers: {}", e))
    })?;

    match mode_str {
        "sync" => run_sync_op(&command, &cfg, &parsers, &tasks_dir),
        "bg" | "background" => run_bg_op(&command, &tasks_dir, cfg.max_output_bytes),
        _ => run_auto_op(&command, &cfg, &parsers, &tasks_dir),
    }
}

fn run_sync_op(
    command: &[String],
    cfg: &GlobalConfig,
    parsers: &HashMap<String, ParserConfig>,
    tasks_dir: &str,
) -> Result<Value, protocol::JsonRpcErrorObj> {
    let result = exec::run_sync(command, cfg.max_output_bytes).map_err(|e| {
        protocol::JsonRpcErrorObj::new(protocol::INTERNAL_ERROR, format!("Command execution failed: {}", e))
    })?;

    let summary = parse::parse_output(
        &command.join(" "),
        &result.stdout,
        &result.stderr,
        parsers,
        cfg.max_errors,
        cfg.max_warnings,
    )
    .map_err(|e| {
        protocol::JsonRpcErrorObj::new(protocol::INTERNAL_ERROR, format!("Output parsing failed: {}", e))
    })?;

    let formatted = parse::summarizer::format_summary(&summary);
    let task_id = TaskId::new();

    let meta = make_completed_meta(task_id.clone(), &command.join(" "), TaskMode::Sync, &result, &summary);
    write_task_output(tasks_dir, &task_id, &result.stdout, &result.stderr);

    let task = Task {
        meta,
        stdout_path: format!("{}/{}/output.log", tasks_dir, task_id.as_str()),
        stderr_path: format!("{}/{}/error.log", tasks_dir, task_id.as_str()),
    };
    let _ = storage::save_task(&task);

    Ok(json!({
        "summary": formatted,
        "task_id": task_id.to_string(),
        "exit_code": result.exit_code,
        "status": summary_status_to_task_status(&summary).as_str(),
    }))
}

fn run_bg_op(command: &[String], tasks_dir: &str, max_output_bytes: u64) -> Result<Value, protocol::JsonRpcErrorObj> {
    let task_id = TaskId::new();
    let (meta, child) = exec::backgrounder::run_background(command, &task_id, tasks_dir, max_output_bytes)
        .map_err(|e| {
            protocol::JsonRpcErrorObj::new(
                protocol::INTERNAL_ERROR,
                format!("Failed to start background task: {}", e),
            )
        })?;

    // Detach — the child runs independently
    std::mem::drop(child);

    let task = Task {
        meta,
        stdout_path: format!("{}/{}/output.log", tasks_dir, task_id.as_str()),
        stderr_path: format!("{}/{}/error.log", tasks_dir, task_id.as_str()),
    };
    storage::save_task(&task).map_err(|e| {
        protocol::JsonRpcErrorObj::new(
            protocol::INTERNAL_ERROR,
            format!("Failed to save task: {}", e),
        )
    })?;

    Ok(json!({
        "task_id": task_id.to_string(),
        "status": "running",
    }))
}

fn run_auto_op(
    command: &[String],
    cfg: &GlobalConfig,
    parsers: &HashMap<String, ParserConfig>,
    tasks_dir: &str,
) -> Result<Value, protocol::JsonRpcErrorObj> {
    let options = exec::watcher::WatchOptions {
        max_bytes: cfg.max_output_bytes,
        timeout_secs: cfg.auto_wait_secs,
    };

    match exec::watcher::watch_command(command, &options).map_err(|e| {
        protocol::JsonRpcErrorObj::new(protocol::INTERNAL_ERROR, format!("Watch failed: {}", e))
    })? {
        exec::watcher::WatchResult::Completed(result) => {
            let summary = parse::parse_output(
                &command.join(" "),
                &result.stdout,
                &result.stderr,
                parsers,
                cfg.max_errors,
                cfg.max_warnings,
            )
            .map_err(|e| {
                protocol::JsonRpcErrorObj::new(
                    protocol::INTERNAL_ERROR,
                    format!("Output parsing failed: {}", e),
                )
            })?;

            let formatted = parse::summarizer::format_summary(&summary);
            let task_id = TaskId::new();
            let meta = make_completed_meta(task_id.clone(), &command.join(" "), TaskMode::Auto, &result, &summary);
            write_task_output(tasks_dir, &task_id, &result.stdout, &result.stderr);

            let task = Task {
                meta,
                stdout_path: format!("{}/{}/output.log", tasks_dir, task_id.as_str()),
                stderr_path: format!("{}/{}/error.log", tasks_dir, task_id.as_str()),
            };
            let _ = storage::save_task(&task);

            Ok(json!({
                "summary": formatted,
                "task_id": task_id.to_string(),
                "exit_code": result.exit_code,
                "status": summary_status_to_task_status(&summary).as_str(),
            }))
        }
        exec::watcher::WatchResult::NeedsBackground { .. } => run_bg_op(command, tasks_dir, cfg.max_output_bytes),
    }
}

fn handle_smol_status(args: &Value) -> Result<Value, protocol::JsonRpcErrorObj> {
    let task_id_str = args
        .get("task_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            protocol::JsonRpcErrorObj::new(protocol::INVALID_PARAMS, "Missing 'task_id' parameter")
        })?;

    let cfg = config::load_global_config().map_err(|e| {
        protocol::JsonRpcErrorObj::new(protocol::INTERNAL_ERROR, format!("Failed to load config: {}", e))
    })?;
    let tasks_dir = get_tasks_dir(&cfg);
    storage::init(&tasks_dir).map_err(|e| {
        protocol::JsonRpcErrorObj::new(protocol::INTERNAL_ERROR, format!("Storage init failed: {}", e))
    })?;

    let id = resolve_task_id(task_id_str, &tasks_dir).map_err(|e| {
        let msg = format!("{}", e);
        protocol::JsonRpcErrorObj::new(protocol::INTERNAL_ERROR, msg)
    })?;

    let meta = storage::load_task_meta(&id, &tasks_dir).map_err(|e| {
        let msg = format!("{}", e);
        protocol::JsonRpcErrorObj::new(protocol::INTERNAL_ERROR, msg)
    })?;

    Ok(serde_json::to_value(&meta).unwrap_or_default())
}

fn handle_smol_log(args: &Value) -> Result<Value, protocol::JsonRpcErrorObj> {
    let task_id_str = args
        .get("task_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            protocol::JsonRpcErrorObj::new(protocol::INVALID_PARAMS, "Missing 'task_id' parameter")
        })?;

    let errors = args.get("errors").and_then(|v| v.as_bool()).unwrap_or(false);
    let warnings = args.get("warnings").and_then(|v| v.as_bool()).unwrap_or(false);
    let stats = args.get("stats").and_then(|v| v.as_bool()).unwrap_or(false);
    let tail = args.get("tail").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let max_chars = args.get("max_chars").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

    let cfg = config::load_global_config().map_err(|e| {
        protocol::JsonRpcErrorObj::new(protocol::INTERNAL_ERROR, format!("Failed to load config: {}", e))
    })?;
    let tasks_dir = get_tasks_dir(&cfg);
    storage::init(&tasks_dir).map_err(|e| {
        protocol::JsonRpcErrorObj::new(protocol::INTERNAL_ERROR, format!("Storage init failed: {}", e))
    })?;

    let id = resolve_task_id(task_id_str, &tasks_dir).map_err(|e| {
        let msg = format!("{}", e);
        protocol::JsonRpcErrorObj::new(protocol::INTERNAL_ERROR, msg)
    })?;

    // If stats requested, return the full metadata as JSON
    if stats {
        let meta = storage::load_task_meta(&id, &tasks_dir).map_err(|e| {
            let msg = format!("{}", e);
            protocol::JsonRpcErrorObj::new(protocol::INTERNAL_ERROR, msg)
        })?;
        return Ok(serde_json::to_value(&meta).unwrap_or_default());
    }

    // Read the log file
    let log_path = std::path::Path::new(&tasks_dir)
        .join(id.as_str())
        .join("output.log");
    let output = if log_path.exists() {
        std::fs::read_to_string(&log_path).unwrap_or_default()
    } else {
        String::new()
    };

    let filtered: String = if errors {
        output
            .lines()
            .filter(|l| {
                l.contains("error")
                    || l.contains("ERROR")
                    || l.contains("Error")
            })
            .collect::<Vec<_>>()
            .join("\n")
    } else if warnings {
        output
            .lines()
            .filter(|l| {
                l.contains("warning")
                    || l.contains("WARNING")
                    || l.contains("Warning")
            })
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        output
    };

    let filtered = if tail > 0 {
        let lines: Vec<&str> = filtered.lines().collect();
        let start = lines.len().saturating_sub(tail);
        lines[start..].join("\n")
    } else {
        filtered
    };

    let filtered = if max_chars > 0 && filtered.len() > max_chars {
        let mut truncated = filtered[..max_chars].to_string();
        // Avoid splitting in the middle of a line
        if let Some(pos) = truncated.rfind('\n') {
            truncated.truncate(pos);
        }
        truncated.push_str(&format!("\n... [truncated, {} of {} chars shown]", max_chars, filtered.len()));
        truncated
    } else {
        filtered
    };

    Ok(json!({ "log": filtered }))
}

fn handle_smol_list(args: &Value) -> Result<Value, protocol::JsonRpcErrorObj> {
    let running_only = args.get("running").and_then(|v| v.as_bool()).unwrap_or(false);

    let cfg = config::load_global_config().map_err(|e| {
        protocol::JsonRpcErrorObj::new(protocol::INTERNAL_ERROR, format!("Failed to load config: {}", e))
    })?;
    let tasks_dir = get_tasks_dir(&cfg);
    storage::init(&tasks_dir).map_err(|e| {
        protocol::JsonRpcErrorObj::new(protocol::INTERNAL_ERROR, format!("Storage init failed: {}", e))
    })?;

    let mut tasks = storage::list_tasks(&tasks_dir, None).map_err(|e| {
        let msg = format!("{}", e);
        protocol::JsonRpcErrorObj::new(protocol::INTERNAL_ERROR, msg)
    })?;

    if running_only {
        tasks.retain(|t| t.status == TaskStatus::Running);
    }

    Ok(serde_json::to_value(&tasks).unwrap_or_default())
}

fn handle_smol_cancel(args: &Value) -> Result<Value, protocol::JsonRpcErrorObj> {
    let task_id_str = args
        .get("task_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            protocol::JsonRpcErrorObj::new(protocol::INVALID_PARAMS, "Missing 'task_id' parameter")
        })?;

    let cfg = config::load_global_config().map_err(|e| {
        protocol::JsonRpcErrorObj::new(protocol::INTERNAL_ERROR, format!("Failed to load config: {}", e))
    })?;
    let tasks_dir = get_tasks_dir(&cfg);
    storage::init(&tasks_dir).map_err(|e| {
        protocol::JsonRpcErrorObj::new(protocol::INTERNAL_ERROR, format!("Storage init failed: {}", e))
    })?;

    let id = resolve_task_id(task_id_str, &tasks_dir).map_err(|e| {
        let msg = format!("{}", e);
        protocol::JsonRpcErrorObj::new(protocol::INTERNAL_ERROR, msg)
    })?;

    storage::cancel_task(&id, &tasks_dir).map_err(|e| {
        let msg = format!("{}", e);
        protocol::JsonRpcErrorObj::new(protocol::INTERNAL_ERROR, msg)
    })?;

    Ok(json!({
        "success": true,
        "task_id": id.to_string(),
        "message": format!("Task {} cancelled.", id),
    }))
}

fn handle_smol_clean(args: &Value) -> Result<Value, protocol::JsonRpcErrorObj> {
    let older = args
        .get("older")
        .and_then(|v| v.as_str())
        .unwrap_or("24h");

    let cfg = config::load_global_config().map_err(|e| {
        protocol::JsonRpcErrorObj::new(protocol::INTERNAL_ERROR, format!("Failed to load config: {}", e))
    })?;
    let tasks_dir = get_tasks_dir(&cfg);
    storage::init(&tasks_dir).map_err(|e| {
        protocol::JsonRpcErrorObj::new(protocol::INTERNAL_ERROR, format!("Storage init failed: {}", e))
    })?;

    let secs = parse_duration(older).map_err(|e| {
        let msg = format!("{}", e);
        protocol::JsonRpcErrorObj::new(protocol::INVALID_PARAMS, msg)
    })?;

    let count = storage::clean_older_than(&tasks_dir, secs).map_err(|e| {
        let msg = format!("{}", e);
        protocol::JsonRpcErrorObj::new(protocol::INTERNAL_ERROR, msg)
    })?;

    Ok(json!({
        "cleaned": count,
        "message": format!("Cleaned up {} task(s).", count),
    }))
}

fn handle_smol_wait(args: &Value) -> Result<Value, protocol::JsonRpcErrorObj> {
    let task_id_str = args
        .get("task_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            protocol::JsonRpcErrorObj::new(protocol::INVALID_PARAMS, "Missing 'task_id' parameter")
        })?;

    let timeout_secs: u64 = args
        .get("timeout")
        .and_then(|v| v.as_i64())
        .unwrap_or(60) as u64;
    let interval_secs: u64 = args
        .get("interval")
        .and_then(|v| v.as_i64())
        .unwrap_or(2) as u64;

    if interval_secs == 0 {
        return Err(protocol::JsonRpcErrorObj::new(
            protocol::INVALID_PARAMS,
            "Interval must be greater than 0",
        ));
    }

    let cfg = config::load_global_config().map_err(|e| {
        protocol::JsonRpcErrorObj::new(protocol::INTERNAL_ERROR, format!("Failed to load config: {}", e))
    })?;
    let tasks_dir = get_tasks_dir(&cfg);
    storage::init(&tasks_dir).map_err(|e| {
        protocol::JsonRpcErrorObj::new(protocol::INTERNAL_ERROR, format!("Storage init failed: {}", e))
    })?;

    let id = resolve_task_id(task_id_str, &tasks_dir).map_err(|e| {
        let msg = format!("{}", e);
        protocol::JsonRpcErrorObj::new(protocol::INTERNAL_ERROR, msg)
    })?;

    let start = std::time::Instant::now();
    let timeout_dur = std::time::Duration::from_secs(timeout_secs);

    loop {
        let meta = storage::load_task_meta(&id, &tasks_dir).map_err(|e| {
            let msg = format!("{}", e);
            protocol::JsonRpcErrorObj::new(protocol::INTERNAL_ERROR, msg)
        })?;

        if meta.status.is_terminal() {
            let mut value = serde_json::to_value(&meta).unwrap_or_default();
            if let Some(obj) = value.as_object_mut() {
                obj.insert("waited".into(), json!(true));
            }
            return Ok(value);
        }

        if start.elapsed() >= timeout_dur {
            let mut value = serde_json::to_value(&meta).unwrap_or_default();
            if let Some(obj) = value.as_object_mut() {
                obj.insert("waited".into(), json!(false));
                obj.insert("wait_timeout".into(), json!(true));
            }
            return Ok(value);
        }

        std::thread::sleep(std::time::Duration::from_secs(interval_secs));
    }
}
