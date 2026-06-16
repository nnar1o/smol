/// Execution mode for running commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Wait for the command to finish (blocking).
    Sync,
    /// Wait a few seconds; if still running, switch to background (default).
    Auto,
    /// Launch immediately in background.
    Background,
    /// Run synchronously with live progress display (spinner + counters).
    Interactive,
}

impl Mode {
    pub fn from_str(s: &str) -> Self {
        match s {
            "sync" => Mode::Sync,
            "background" | "bg" => Mode::Background,
            "interactive" | "int" | "i" => Mode::Interactive,
            _ => Mode::Auto,
        }
    }
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Auto
    }
}

/// Parsed CLI command.
#[derive(Debug, Clone)]
pub enum CliCommand {
    Run { command: Vec<String>, mode: Mode },
    Status { task_id: String, tokens: bool },
    Log { task_id: String, errors: bool, warnings: bool, stats: bool },
    List { running: bool },
    Cancel { task_id: String },
    Clean { older: Option<String> },
    #[cfg(debug_assertions)]
    Mock {
        name: String,
        delay: Option<f64>,
        error_count: Option<usize>,
        warning_count: Option<usize>,
        file: Option<String>,
        stream: Option<String>,
    },
    Migrate { db_path: Option<String> },
    Setup { host: Option<String> },
    Uninstall { host: Option<String> },
    Completion { shell: String },
    Search { query: String },
    Export { task_id: Option<String>, format: Option<String> },
    Import { path: String },
    Parsers { action: String },
    #[cfg(debug_assertions)]
    Benchmark,
}

/// Parse CLI arguments. Returns a CliCommand.
pub fn parse_cli() -> CliCommand {
    let raw_args: Vec<String> = std::env::args().skip(1).collect();

    if raw_args.is_empty() {
        print_usage_and_exit();
    }

    // Handle --help / -h / help subcommand
    if raw_args.len() == 1 && (raw_args[0] == "--help" || raw_args[0] == "-h" || raw_args[0] == "help") {
        print_usage_and_exit();
    }

    let subcommand = raw_args[0].as_str();

    match subcommand {
        "status" => {
            let task_id = raw_args.get(1).cloned().unwrap_or_else(|| "last".to_string());
            let tokens = has_flag(&raw_args, "--tokens", "-t");
            CliCommand::Status { task_id, tokens }
        }
        "log" => {
            let task_id = raw_args.get(1).cloned().unwrap_or_else(|| {
                eprintln!("Error: task-id required");
                std::process::exit(1);
            });
            let errors = has_flag(&raw_args, "--errors", "-e");
            let warnings = has_flag(&raw_args, "--warnings", "-w");
            let stats = has_flag(&raw_args, "--stats", "-s");
            CliCommand::Log { task_id, errors, warnings, stats }
        }
        "list" => {
            let running = has_flag(&raw_args, "--running", "");
            CliCommand::List { running }
        }
        "cancel" => {
            let task_id = raw_args.get(1).cloned().unwrap_or_else(|| {
                eprintln!("Error: task-id required");
                std::process::exit(1);
            });
            CliCommand::Cancel { task_id }
        }
        "clean" => {
            let older = get_opt_value(&raw_args, "--older", "-o");
            CliCommand::Clean { older }
        }
        #[cfg(debug_assertions)]
        "mock-command" => {
            CliCommand::Mock {
                name: get_opt_value(&raw_args, "--name", "-n").unwrap_or_else(|| "maven_success".to_string()),
                delay: get_opt_value(&raw_args, "--delay", "").and_then(|s| s.parse::<f64>().ok()),
                error_count: get_opt_value(&raw_args, "--error-count", "").and_then(|s| s.parse::<usize>().ok()),
                warning_count: get_opt_value(&raw_args, "--warning-count", "").and_then(|s| s.parse::<usize>().ok()),
                file: get_opt_value(&raw_args, "--file", "-f"),
                stream: get_opt_value(&raw_args, "--stream", ""),
            }
        }
        "migrate" => {
            let db_path = get_opt_value(&raw_args, "--db", "");
            CliCommand::Migrate { db_path }
        }
        #[cfg(debug_assertions)]
        "benchmark" => {
            CliCommand::Benchmark
        }
        "setup" => {
            let host = raw_args.get(1).cloned();
            CliCommand::Setup { host }
        }
        "uninstall" => {
            let host = raw_args.get(1).cloned();
            CliCommand::Uninstall { host }
        }
        "search" => {
            let query = raw_args.get(1).cloned().unwrap_or_else(|| {
                eprintln!("Error: search query required");
                std::process::exit(1);
            });
            CliCommand::Search { query }
        }
        "export" => {
            let task_id = raw_args.get(1).cloned();
            let format = get_opt_value(&raw_args, "--format", "-f").unwrap_or_else(|| "json".to_string());
            CliCommand::Export { task_id, format: Some(format) }
        }
        "import" => {
            let path = raw_args.get(1).cloned().unwrap_or_else(|| {
                eprintln!("Error: import path required");
                std::process::exit(1);
            });
            CliCommand::Import { path }
        }
        "parsers" => {
            let action = raw_args.get(1).cloned().unwrap_or_else(|| "list".to_string());
            CliCommand::Parsers { action }
        }
        "completion" => {
            let shell = raw_args.get(1).cloned().unwrap_or_else(|| "bash".to_string());
            CliCommand::Completion { shell }
        }
        _ => {
            // Direct command: detect mode flags, rest is the command
            let mut mode = Mode::Auto;
            let mut cmd_args: Vec<String> = Vec::new();
            let mut i = 0;
            while i < raw_args.len() {
                let arg = &raw_args[i];
                match arg.as_str() {
                    "--sync" => mode = Mode::Sync,
                    "--bg" => mode = Mode::Background,
                    "--auto" => mode = Mode::Auto,
                    "--interactive" | "--int" | "-i" => mode = Mode::Interactive,
                    "--mode" => {
                        if let Some(val) = raw_args.get(i + 1) {
                            mode = Mode::from_str(val);
                            i += 1; // skip value
                        }
                    }
                    _ => cmd_args.push(arg.clone()),
                }
                i += 1;
            }

            CliCommand::Run { command: cmd_args, mode }
        }
    }
}

fn print_usage_and_exit() -> ! {
    eprintln!("smol — Smart Minimal Output Logger");
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  smol <command> [args...]           run a command");
    eprintln!("  smol status <task-id>              show task status");
    eprintln!("  smol log <task-id>                 show task log");
    eprintln!("  smol list                          list tasks");
    eprintln!("  smol cancel <task-id>              cancel a task");
    eprintln!("  smol clean                         clean old tasks");
    eprintln!("  smol search <query>                full-text search in task logs");
    eprintln!("  smol export [<task-id>]            export task(s) as JSON or markdown");
    eprintln!("  smol import <file>                 import a task from JSON");
    eprintln!("  smol parsers <action>              manage parsers (list, init, sync)");
    eprintln!("  smol setup [<host>]                install smol hooks for AI CLI (opencode, claude)");
    eprintln!("  smol uninstall [<host>]            remove smol hooks for AI CLI");
    eprintln!("  smol completion <shell>            generate shell completion script");
    eprintln!("  smol migrate [--db <path>]          migrate TOML tasks to SQLite");
    eprintln!();
    eprintln!("Modes:");
    eprintln!("  --sync       Wait for command to finish");
    eprintln!("  --auto       Wait a few seconds, then background if slow (default)");
    eprintln!("  --bg         Run in background immediately");
    eprintln!("  --interactive  Run with live progress display (spinner + counters)");
    #[cfg(debug_assertions)]
    eprintln!();
    #[cfg(debug_assertions)]
    eprintln!("  smol mock-command --name <name>    [test] mock command");
    #[cfg(debug_assertions)]
    eprintln!("  smol benchmark                    [test] run token benchmarks");
    std::process::exit(1);
}

fn has_flag(args: &[String], long: &str, short: &str) -> bool {
    args.iter().any(|a| a == long || (!short.is_empty() && a == short))
}

fn get_opt_value(args: &[String], long: &str, short: &str) -> Option<String> {
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg == long || (!short.is_empty() && arg == short) {
            return args.get(i + 1).cloned();
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode_from_str() {
        assert_eq!(Mode::from_str("sync"), Mode::Sync);
        assert_eq!(Mode::from_str("auto"), Mode::Auto);
        assert_eq!(Mode::from_str("bg"), Mode::Background);
        assert_eq!(Mode::from_str("background"), Mode::Background);
        assert_eq!(Mode::from_str("interactive"), Mode::Interactive);
        assert_eq!(Mode::from_str("int"), Mode::Interactive);
        assert_eq!(Mode::from_str("unknown"), Mode::Auto);
    }

    #[test]
    fn test_has_flag() {
        let args = vec!["--foo".into(), "-b".into(), "value".into()];
        assert!(has_flag(&args, "--foo", ""));
        assert!(has_flag(&args, "", "-b"));
        assert!(!has_flag(&args, "--bar", ""));
    }

    #[test]
    fn test_get_opt_value() {
        let args = vec!["--name".into(), "test".into(), "-f".into(), "file.txt".into()];
        assert_eq!(get_opt_value(&args, "--name", "-n"), Some("test".to_string()));
        assert_eq!(get_opt_value(&args, "--file", "-f"), Some("file.txt".to_string()));
        assert_eq!(get_opt_value(&args, "--not-found", ""), None);
    }
}
