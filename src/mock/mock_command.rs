use std::fs;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

use crate::core::SmolError;

/// Run a mock command with deterministic output for testing.
///
/// Supported `name` values:
/// - `maven_success` — Maven BUILD SUCCESS output
/// - `maven_failure` — Maven BUILD FAILURE with errors and warnings
/// - `cargo_success` — Cargo build success
/// - `cargo_errors` — Cargo build with rustc errors
/// - `gcc_errors` — GCC compilation errors
/// - `generic_input` — Simple mixed output
/// - `slow` — Output with delay (for testing auto/bg modes)
pub fn run_mock_command(
    name: &str,
    delay: Option<f64>,
    error_count: Option<usize>,
    warning_count: Option<usize>,
    file: Option<&str>,
    stream: Option<&str>,
) -> Result<i32, SmolError> {
    // Optional delay before output
    if let Some(d) = delay {
        let ms = (d * 1000.0) as u64;
        thread::sleep(Duration::from_millis(ms));
    }

    // If a file is specified, read and output its contents
    if let Some(path) = file {
        let content = fs::read_to_string(path)
            .map_err(|e| SmolError::other(format!("Failed to read {}: {}", path, e)))?;
        match stream.unwrap_or("stdout") {
            "stderr" => eprint!("{}", content),
            _ => print!("{}", content),
        }
        io::stdout().flush().ok();
        return Ok(0);
    }

    // Generate output based on name
    let (output, exit_code) = generate_output(name, error_count, warning_count);

    match stream.unwrap_or("stdout") {
        "stderr" => eprint!("{}", output),
        _ => print!("{}", output),
    }
    io::stdout().flush().ok();

    Ok(exit_code)
}

fn generate_output(name: &str, error_count: Option<usize>, warning_count: Option<usize>) -> (String, i32) {
    let errors = error_count.unwrap_or(0);
    let warnings = warning_count.unwrap_or(0);

    match name {
        "maven_success" => (
            "\
[INFO] Scanning for projects...
[INFO] 
[INFO] --- maven-compiler-plugin:3.8.1:compile (default-compile) @ my-app ---
[INFO] Compiling 15 source files to /target/classes
[INFO] 
[INFO] --- maven-jar-plugin:3.1.0:jar (default-jar) @ my-app ---
[INFO] Building jar: /target/my-app-1.0.jar
[INFO] 
[INFO] --- maven-install-plugin:2.5.2:install (default-install) @ my-app ---
[INFO] Installing /target/my-app-1.0.jar to /home/user/.m2/repository
[INFO] 
[INFO] BUILD SUCCESS
[INFO] ------------------------------------------------------------------------
[INFO] Total time:  2.456 s
[INFO] ------------------------------------------------------------------------
".into(), 0),

        "maven_failure" => {
            let mut out = String::from("\
[INFO] Scanning for projects...
[INFO] 
[INFO] --- maven-compiler-plugin:3.8.1:compile (default-compile) @ my-app ---
[INFO] Compiling 15 source files to /target/classes
");
            for i in 1..=errors.max(3) {
                out.push_str(&format!("[ERROR] /src/main/java/App.java:{}: error: cannot find symbol\n", 40 + i));
            }
            for i in 1..=warnings.max(5) {
                out.push_str(&format!("[WARNING] /src/main/java/App.java:{}: unchecked cast\n", 10 + i));
            }
            out.push_str("\
[INFO] 
[INFO] ------------------------------------------------------------------------
[INFO] BUILD FAILURE
[INFO] ------------------------------------------------------------------------
");
            (out, 1)
        },

        "cargo_success" => (
            "\
   Compiling my-app v0.1.0 (/home/user/my-app)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.42s
".into(), 0),

        "cargo_errors" => {
            let mut out = String::from("\
   Compiling my-app v0.1.0 (/home/user/my-app)
");
            for i in 1..=errors.max(2) {
                out.push_str(&format!(
                    "error[E0425]: cannot find value `foo` in this scope\n\
                    --> src/main.rs:{0}:9\n\
                     |\n\
                    {0} |     let x = foo;\n\
                     |         ^^^ not found in this scope\n", 10 + i));
            }
            if warnings > 0 {
                out.push_str(&format!(
                    "warning: unused variable `x`\n\
                    --> src/main.rs:{0}:5\n", 15));
            }
            out.push_str("error: could not compile `my-app` due to previous error\n");
            (out, 1)
        },

        "gcc_errors" => {
            let mut out = String::new();
            for i in 1..=errors.max(2) {
                out.push_str(&format!(
                    "test.c:{}:5: error: expected ';' before 'return'\n", 10 + i));
            }
            if warnings > 0 {
                out.push_str("test.c:15:3: warning: implicit declaration of function 'foo'\n");
            }
            (out, 1)
        },

        "generic_output" => {
            let mut out = String::new();
            for i in 1..=errors.max(0) {
                out.push_str(&format!("ERROR: something failed at step {}\n", i));
            }
            for i in 1..=warnings.max(0) {
                out.push_str(&format!("WARNING: something suspicious at step {}\n", i));
            }
            if errors > 0 {
                out.push_str("FAIL: build did not complete\n");
            } else {
                out.push_str("DONE: all steps completed\n");
            }
            (out, if errors > 0 { 1 } else { 0 })
        },

        _ => {
            // Default: just echo the name
            (format!("mock-command: {}\n", name), 0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_maven_success() {
        let (output, code) = generate_output("maven_success", None, None);
        assert!(output.contains("BUILD SUCCESS"));
        assert_eq!(code, 0);
    }

    #[test]
    fn test_mock_maven_failure() {
        let (output, code) = generate_output("maven_failure", Some(2), Some(3));
        assert!(output.contains("BUILD FAILURE"));
        assert!(output.contains("[ERROR]"));
        assert!(output.contains("[WARNING]"));
        assert_eq!(code, 1);
    }

    #[test]
    fn test_mock_cargo_errors() {
        let (output, code) = generate_output("cargo_errors", Some(1), Some(1));
        assert!(output.contains("error[E0425]"));
        assert!(output.contains("warning:"));
        assert_eq!(code, 1);
    }

    #[test]
    fn test_mock_gcc_errors() {
        let (output, code) = generate_output("gcc_errors", Some(3), Some(1));
        assert!(output.contains("test.c"));
        assert!(output.contains("error:"));
        assert_eq!(code, 1);
    }
}
