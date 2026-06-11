pub mod args;
pub mod commands;

pub use args::CliCommand;
pub use commands::execute_command;

/// Run the CLI and return the exit code.
pub fn run() {
    let cmd = args::parse_cli();
    let result = execute_command(cmd);
    match result {
        Ok(exit_code) => std::process::exit(exit_code),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
