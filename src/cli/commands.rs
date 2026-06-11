use crate::cli::args::{CliCommand, Mode};
use crate::core::SmolError;
use crate::config;
use crate::storage;
use crate::parse;
use crate::exec;
use crate::exec::signal;
#[cfg(debug_assertions)]
use crate::mock;

/// Execute a parsed CLI command and return the exit code.
pub fn execute_command(cmd: CliCommand) -> Result<i32, SmolError> {
    match cmd {
        CliCommand::Run { command, mode } => {
            execute_run(command, mode)
        }
        CliCommand::Status { task_id } => {
            execute_status(task_id)
        }
        CliCommand::Log { task_id, errors, warnings, stats } => {
            execute_log(task_id, errors, warnings, stats)
        }
        CliCommand::List { running } => {
            execute_list(running)
        }
        CliCommand::Cancel { task_id } => {
            execute_cancel(task_id)
        }
        CliCommand::Clean { older } => {
            execute_clean(older)
        }
        CliCommand::Setup { host } => {
            execute_setup(host)
        }
        CliCommand::Uninstall { host } => {
            execute_uninstall(host)
        }
        CliCommand::Migrate { db_path } => {
            execute_migrate(db_path)
        }
        #[cfg(debug_assertions)]
        CliCommand::Mock { name, delay, error_count, warning_count, file, stream } => {
            execute_mock(name, delay, error_count, warning_count, file, stream)
        }
        #[cfg(debug_assertions)]
        CliCommand::Benchmark => {
            execute_benchmark()
        }
        CliCommand::Search { query } => {
            execute_search(query)
        }
        CliCommand::Export { task_id, format } => {
            execute_export(task_id, format)
        }
        CliCommand::Import { path } => {
            execute_import(path)
        }
        CliCommand::Parsers { action } => {
            execute_parsers_action(action)
        }
        CliCommand::Completion { shell } => {
            execute_completion(shell)
        }
    }
}

fn execute_search(_query: String) -> Result<i32, SmolError> {
    Err(SmolError::other("search is not yet implemented"))
}

fn execute_export(_task_id: Option<String>, _format: Option<String>) -> Result<i32, SmolError> {
    Err(SmolError::other("export is not yet implemented"))
}

fn execute_import(_path: String) -> Result<i32, SmolError> {
    Err(SmolError::other("import is not yet implemented"))
}

fn execute_parsers_action(_action: String) -> Result<i32, SmolError> {
    Err(SmolError::other("parsers is not yet implemented"))
}

fn execute_run(command: Vec<String>, mode: Mode) -> Result<i32, SmolError> {
    // Initialize signal handlers so we can respond to SIGINT/SIGTERM.
    // In background mode, signal handlers are still registered but the
    // spawned child runs in its own session (via setsid) and won't be
    // affected by terminal-originated signals.
    signal::init();

    let global_config = config::load_global_config()?;
    let tasks_dir = if global_config.tasks_dir.is_empty() {
        storage::paths::tasks_dir()
    } else {
        global_config.tasks_dir.clone()
    };

    // Initialize storage
    storage::init(&tasks_dir)?;

    // Load parsers
    let parsers_dir = if global_config.parsers_dir.is_empty() {
        storage::paths::parsers_dir()
    } else {
        global_config.parsers_dir.clone()
    };
    let parsers = config::load_all_parsers(&parsers_dir)?;

    match mode {
        Mode::Sync => {
            // Check for cancellation before starting
            if signal::is_cancelled() {
                return Ok(130); // 128 + SIGINT(2)
            }

            let result = exec::run_sync(&command, global_config.max_output_bytes)?;

            // If a cancellation signal was received during execution,
            // return a non-zero exit code (128 + SIGINT).
            if signal::is_cancelled() {
                return Ok(130);
            }
            let summary = parse::parse_output(
                &command.join(" "),
                &result.stdout,
                &result.stderr,
                &parsers,
                global_config.max_errors,
                global_config.max_warnings,
            )?;

            let formatted = parse::summarizer::format_summary(&summary);
            println!("{}", formatted);
            Ok(result.exit_code.unwrap_or(1))
        }
        Mode::Interactive => {
            if signal::is_cancelled() {
                return Ok(130);
            }

            let result = exec::run_interactive(&command, global_config.max_output_bytes)?;

            if signal::is_cancelled() {
                return Ok(130);
            }
            let summary = parse::parse_output(
                &command.join(" "),
                &result.stdout,
                &result.stderr,
                &parsers,
                global_config.max_errors,
                global_config.max_warnings,
            )?;

            let formatted = parse::summarizer::format_summary(&summary);
            println!("{}", formatted);
            Ok(result.exit_code.unwrap_or(1))
        }
        Mode::Auto => {
            let options = exec::watcher::WatchOptions {
                max_bytes: global_config.max_output_bytes,
                timeout_secs: global_config.auto_wait_secs,
            };
            match exec::watcher::watch_command(&command, &options)? {
                exec::watcher::WatchResult::Completed(result) => {
                    let summary = parse::parse_output(
                        &command.join(" "),
                        &result.stdout,
                        &result.stderr,
                        &parsers,
                        global_config.max_errors,
                        global_config.max_warnings,
                    )?;
                    let formatted = parse::summarizer::format_summary(&summary);
                    println!("{}", formatted);
                    Ok(result.exit_code.unwrap_or(1))
                }
                exec::watcher::WatchResult::NeedsBackground { .. } => {
                    spawn_background_task(&command, &tasks_dir)
                }
            }
        }
        Mode::Background => {
            spawn_background_task(&command, &tasks_dir)
        }
    }
}

fn execute_status(task_id: String) -> Result<i32, SmolError> {
    let tasks_dir = storage::paths::tasks_dir();
    storage::init(&tasks_dir)?;

    let id = resolve_task_id(&task_id, &tasks_dir)?;

    let mut meta = storage::load_task_meta(&id, &tasks_dir)?;
    let _ = update_completed_task(&mut meta, &tasks_dir);

    // Line 1: id  status  command
    print!("{}  {}  {}", meta.id, meta.status.as_str(), meta.command);
    if let Some(d) = meta.duration_sec {
        print!("  [{}s]", d);
    }
    println!();

    // Line 2: err/warn + optional fields
    let mut parts: Vec<String> = Vec::new();
    parts.push(format!("err:{}", meta.error_count));
    parts.push(format!("warn:{}", meta.warning_count));
    if let Some(code) = meta.exit_code {
        parts.push(format!("exit:{}", code));
    }
    if let Some(in_tok) = meta.input_tokens {
        if let Some(out_tok) = meta.output_tokens {
            let reduction = if in_tok > 0 {
                ((1.0 - out_tok as f64 / in_tok as f64) * 100.0).round() as i64
            } else {
                0
            };
            parts.push(format!("tok:{}→{}({}%)", in_tok, out_tok, reduction));
        }
    }
    if let Some(total) = meta.test_total {
        if let Some(passed) = meta.test_passed {
            let failed = meta.test_failed.unwrap_or(0);
            let errors = meta.test_errors.unwrap_or(0);
            let skipped = meta.test_skipped.unwrap_or(0);
            parts.push(format!("tests:{}/ {}/ {}/ {}/{}", total, passed, failed, errors, skipped));
        }
    }
    println!("  {}", parts.join("  "));
    Ok(0)
}

fn resolve_task_id(task_id: &str, tasks_dir: &str) -> Result<crate::core::TaskId, SmolError> {
    if task_id == "last" {
        let tasks = storage::list_tasks(tasks_dir, None)?;
        tasks.into_iter().next()
            .ok_or_else(|| SmolError::TaskNotFound("no tasks found".into()))
            .map(|t| t.id)
    } else {
        task_id.parse::<crate::core::TaskId>()
            .map_err(|e| SmolError::InvalidTaskId(e))
    }
}

fn execute_log(task_id: String, errors: bool, warnings: bool, stats: bool) -> Result<i32, SmolError> {
    let tasks_dir = storage::paths::tasks_dir();
    storage::init(&tasks_dir)?;

    let id = resolve_task_id(&task_id, &tasks_dir)?;

    // Lazily update task status
    if let Ok(mut meta) = storage::load_task_meta(&id, &tasks_dir) {
        let _ = update_completed_task(&mut meta, &tasks_dir);
    }

    if stats {
        let mut meta = storage::load_task_meta(&id, &tasks_dir)?;
        let _ = update_completed_task(&mut meta, &tasks_dir);
        println!("{}", serde_json::to_string_pretty(&meta)
            .map_err(|e| SmolError::other(format!("JSON error: {}", e)))?);
        return Ok(0);
    }

    let log_path = std::path::Path::new(&tasks_dir)
        .join(id.as_str())
        .join("output.log");
    let output = if log_path.exists() {
        std::fs::read_to_string(&log_path)?
    } else {
        String::new()
    };

    if errors {
        for line in output.lines() {
            if line.contains("error") || line.contains("ERROR") || line.contains("Error") {
                println!("{}", line);
            }
        }
    } else if warnings {
        for line in output.lines() {
            if line.contains("warning") || line.contains("WARNING") || line.contains("Warning") {
                println!("{}", line);
            }
        }
    } else {
        print!("{}", output);
    }

    Ok(0)
}

fn execute_list(running: bool) -> Result<i32, SmolError> {
    let tasks_dir = storage::paths::tasks_dir();
    storage::init(&tasks_dir)?;

    let mut tasks = storage::list_tasks(&tasks_dir, None)?;
    // Lazily update running tasks that may have completed
    for task in tasks.iter_mut() {
        let _ = update_completed_task(task, &tasks_dir);
    }
    // Apply filter after status update
    if running {
        tasks.retain(|t| t.status == crate::core::TaskStatus::Running);
    }
    for task in &tasks {
        println!("{}  {}  {}  errors:{}  warnings:{}",
            task.id,
            task.status.as_str(),
            task.created_at.format("%Y-%m-%d %H:%M:%S"),
            task.error_count,
            task.warning_count,
        );
    }
    if tasks.is_empty() {
        println!("No tasks found.");
    }
    Ok(0)
}

fn execute_cancel(task_id: String) -> Result<i32, SmolError> {
    let tasks_dir = storage::paths::tasks_dir();
    storage::init(&tasks_dir)?;

    let id = resolve_task_id(&task_id, &tasks_dir)?;
    storage::cancel_task(&id, &tasks_dir)?;
    println!("Task {} cancelled.", id);
    Ok(0)
}

fn execute_clean(older: Option<String>) -> Result<i32, SmolError> {
    let tasks_dir = storage::paths::tasks_dir();
    storage::init(&tasks_dir)?;

    let secs = match older {
        Some(dur) => parse_duration(&dur)?,
        None => 86400, // default 24h
    };
    let count = storage::clean_older_than(&tasks_dir, secs)?;
    println!("Cleaned up {} task(s).", count);
    Ok(0)
}

/// Check if a PID is still alive (Unix: kill -0).
fn is_pid_alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// If a task is still marked "Running" but its process has exited,
/// parse the output and update the metadata accordingly.
fn update_completed_task(
    meta: &mut crate::core::TaskMeta,
    tasks_dir: &str,
) -> Result<(), SmolError> {
    if meta.status != crate::core::TaskStatus::Running {
        return Ok(());
    }

    // Check if PID is still alive
    let pid = meta.background_pid.or(meta.pid);
    if let Some(pid) = pid {
        if is_pid_alive(pid) {
            return Ok(()); // Still running
        }
    }

    // Process has exited — read output and compute final status
    let task_dir = std::path::Path::new(tasks_dir).join(meta.id.as_str());
    let stdout = std::fs::read_to_string(task_dir.join("output.log")).unwrap_or_default();
    let stderr = std::fs::read_to_string(task_dir.join("error.log")).unwrap_or_default();

    // Load parsers
    let parsers_dir = crate::storage::paths::parsers_dir();
    let parsers = crate::config::load_all_parsers(&parsers_dir)?;

    // Parse output
    let summary = crate::parse::parse_output(
        &meta.command,
        &stdout,
        &stderr,
        &parsers,
        100, // keep all for background tasks
        100,
    )?;

    let now = chrono::Utc::now();
    meta.completed_at = Some(now);
    // Compute approximate duration from created_at to completed_at
    if let Some(duration) = now.signed_duration_since(meta.created_at).num_seconds().try_into().ok() {
        meta.duration_sec = Some(duration);
    }
    meta.error_count = summary.error_count;
    meta.warning_count = summary.warning_count;
    meta.input_tokens = Some(summary.input_tokens);
    meta.output_tokens = Some(summary.output_tokens);
    meta.compression_ratio = Some(summary.compression_ratio);
    if let Some(ref tests) = summary.tests {
        meta.test_total = Some(tests.total);
        meta.test_passed = Some(tests.passed);
        meta.test_failed = Some(tests.failed);
        meta.test_errors = Some(tests.errors);
        meta.test_skipped = Some(tests.skipped);
    }
    meta.status = match summary.status {
        crate::core::SummaryStatus::Success => {
            meta.exit_code = Some(0);
            crate::core::TaskStatus::Success
        }
        crate::core::SummaryStatus::Failure => {
            meta.exit_code = Some(1);
            crate::core::TaskStatus::Failed
        }
        crate::core::SummaryStatus::Unknown => {
            if summary.error_count > 0 {
                meta.exit_code = Some(1);
                crate::core::TaskStatus::Failed
            } else {
                meta.exit_code = Some(0);
                crate::core::TaskStatus::Success
            }
        }
    };

    // Write updated meta
    let meta_path = task_dir.join("meta.toml");
    let new_content = toml::to_string_pretty(&meta)
        .map_err(|e| SmolError::Config(format!("Failed to serialize meta: {}", e)))?;
    std::fs::write(&meta_path, new_content)?;

    // Update registry
    if let Ok(mut registry) = crate::storage::registry::load_registry(tasks_dir) {
        if let Some(entry) = registry.tasks.iter_mut().find(|e| e.id == meta.id) {
            entry.status = meta.status;
        }
        let _ = crate::storage::registry::save_registry(tasks_dir, &registry);
    }

    Ok(())
}

/// Spawn a task in background. The process runs independently; status will be
/// lazily updated when queried via status/log/list commands.
fn spawn_background_task(
    command: &[String],
    tasks_dir: &str,
) -> Result<i32, SmolError> {
    let task_id = crate::core::TaskId::new();
    let (meta, child) = exec::backgrounder::run_background(
        command,
        &task_id,
        tasks_dir,
        10 * 1024 * 1024,
    )?;
    println!("task-id:{}", task_id);

    // Don't wait for the child — it runs independently.
    // We detach it by dropping the Child handle.
    std::mem::drop(child);

    // Save initial task metadata (status = Running)
    let task = crate::core::Task {
        meta,
        stdout_path: format!("{}/{}/output.log", tasks_dir, task_id),
        stderr_path: format!("{}/{}/error.log", tasks_dir, task_id),
    };
    storage::save_task(&task).ok();
    Ok(0)
}

#[cfg(debug_assertions)]
fn execute_mock(
    name: String,
    delay: Option<f64>,
    error_count: Option<usize>,
    warning_count: Option<usize>,
    file: Option<String>,
    stream: Option<String>,
) -> Result<i32, SmolError> {
    mock::run_mock_command(&name, delay, error_count, warning_count, file.as_deref(), stream.as_deref())
}

#[cfg(debug_assertions)]
fn execute_benchmark() -> Result<i32, SmolError> {
    let results = crate::bench::run_benchmark()?;
    println!("{:<30} {:>12} {:>12} {:>14} {:>12}", "Scenario", "Input Tokens", "Output Tokens", "Reduction %", "Latency ms");
    println!("{}", "-".repeat(84));
    for r in &results {
        println!("{:<30} {:>12} {:>12} {:>13.1}% {:>12}",
            r.scenario,
            r.input_tokens,
            r.output_tokens,
            r.reduction_pct,
            r.latency_ms,
        );
    }
    Ok(0)
}

fn execute_setup(host: Option<String>) -> Result<i32, SmolError> {
    let host_str = host.as_deref().unwrap_or("all");
    if host_str == "all" || host_str == "opencode" {
        match crate::hook::HookManager::setup("opencode") {
            Ok(msg) => println!("{}", msg),
            Err(e) => eprintln!("Warning: opencode setup failed: {}", e),
        }
    }
    if host_str == "all" || host_str == "claude" {
        match crate::hook::HookManager::setup("claude") {
            Ok(msg) => println!("{}", msg),
            Err(e) => eprintln!("Warning: claude setup failed: {}", e),
        }
    }
    if host_str != "all" && host_str != "opencode" && host_str != "claude" {
        eprintln!("Unknown host: {}. Supported: opencode, claude", host_str);
        return Ok(1);
    }
    Ok(0)
}

fn execute_uninstall(host: Option<String>) -> Result<i32, SmolError> {
    match crate::hook::HookManager::uninstall(host.as_deref()) {
        Ok(msg) => println!("{}", msg),
        Err(e) => eprintln!("Warning: uninstall failed: {}", e),
    }
    Ok(0)
}

fn execute_completion(shell: String) -> Result<i32, SmolError> {
    match shell.as_str() {
        "bash" => print!("{}", crate::completions::generate_bash()),
        "zsh" => print!("{}", crate::completions::generate_zsh()),
        "fish" => print!("{}", crate::completions::generate_fish()),
        _ => {
            eprintln!("Unknown shell: {}. Supported: bash, zsh, fish", shell);
            return Ok(1);
        }
    }
    Ok(0)
}

/// Migrate tasks from TOML file storage to SQLite.
fn execute_migrate(db_path: Option<String>) -> Result<i32, SmolError> {
    let tasks_dir = storage::paths::tasks_dir();
    storage::init(&tasks_dir)?;

    let db_path = db_path.unwrap_or_else(|| {
        format!("{}/smol.db", storage::paths::smol_dir())
    });

    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let sqlite = storage::sqlite::SqliteStorage::new(&db_path)?;
    sqlite.init()?;

    // List all tasks from TOML storage
    let tasks = storage::list_tasks(&tasks_dir, None)?;
    if tasks.is_empty() {
        println!("No tasks found to migrate.");
        return Ok(0);
    }

    let mut migrated = 0u64;
    for meta in &tasks {
        sqlite.save_task(meta)?;
        migrated += 1;
    }

    println!("Migrated {} task(s) to SQLite database at: {}", migrated, db_path);
    Ok(0)
}

/// Parse a duration string like "24h", "7d", "3600" into seconds.
fn parse_duration(s: &str) -> Result<u64, SmolError> {
    if let Some(num) = s.strip_suffix('s') {
        let n: u64 = num.parse().map_err(|_| SmolError::config("Invalid duration"))?;
        Ok(n)
    } else if let Some(num) = s.strip_suffix('h') {
        let n: u64 = num.parse().map_err(|_| SmolError::config("Invalid duration"))?;
        Ok(n * 3600)
    } else if let Some(num) = s.strip_suffix('d') {
        let n: u64 = num.parse().map_err(|_| SmolError::config("Invalid duration"))?;
        Ok(n * 86400)
    } else if let Some(num) = s.strip_suffix('m') {
        let n: u64 = num.parse().map_err(|_| SmolError::config("Invalid duration"))?;
        Ok(n * 60)
    } else {
        let n: u64 = s.parse().map_err(|_| {
            SmolError::config("Invalid duration. Use e.g. '24h', '7d', '3600'")
        })?;
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration_hours() {
        assert_eq!(parse_duration("24h").unwrap(), 86400);
        assert_eq!(parse_duration("1h").unwrap(), 3600);
    }

    #[test]
    fn test_parse_duration_days() {
        assert_eq!(parse_duration("7d").unwrap(), 604800);
        assert_eq!(parse_duration("1d").unwrap(), 86400);
    }

    #[test]
    fn test_parse_duration_minutes() {
        assert_eq!(parse_duration("30m").unwrap(), 1800);
    }

    #[test]
    fn test_parse_duration_seconds() {
        assert_eq!(parse_duration("3600").unwrap(), 3600);
        assert_eq!(parse_duration("30s").unwrap(), 30);
    }

    #[test]
    fn test_parse_duration_invalid() {
        assert!(parse_duration("xyz").is_err());
    }
}
