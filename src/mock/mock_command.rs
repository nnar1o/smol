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
/// - `generic_output` — Simple mixed output
/// - `slow` — Output with delay (for testing auto/bg modes)
/// - `gradle_success` — Gradle BUILD SUCCESSFUL
/// - `gradle_failure` — Gradle BUILD FAILED with errors
/// - `npm_success` — npm install success
/// - `npm_failure` — npm install failure with ERR!
/// - `go_success` — Go build success (no output)
/// - `go_failure` — Go build with errors
/// - `tsc_success` — TypeScript compile success
/// - `tsc_failure` — TypeScript compile with TS errors
/// - `python_success` — Python script success
/// - `python_failure` — Python script with Traceback
/// - `make_success` — Make build success
/// - `make_failure` — Make build failure with ***
/// - `jest_success` — Jest test pass
/// - `jest_failure` — Jest test fail
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

        "gradle_success" => (
            "\
> Configure project :app
> Task :app:compileJava UP-TO-DATE
> Task :app:processResources UP-TO-DATE
> Task :app:classes UP-TO-DATE
> Task :app:jar UP-TO-DATE

BUILD SUCCESSFUL in 2s
3 actionable tasks: 3 up-to-date
".into(), 0),

        "gradle_failure" => {
            let mut out = String::from("\
> Configure project :app
> Task :app:compileJava FAILED
");
            for i in 1..=errors.max(2) {
                out.push_str(&format!("/src/main/java/App.java:{}: error: cannot find symbol\n", 20 + i));
            }
            out.push_str("\
BUILD FAILED in 5s
");
            (out, 1)
        },

        "npm_success" => (
            "\
npm notice created a lockfile as package-lock.json
added 123 packages from 456 contributors and audited 789 packages in 12.345s
found 0 vulnerabilities
".into(), 0),

        "npm_failure" => (
            "\
npm ERR! code ENOENT
npm ERR! syscall open
npm ERR! path /home/user/project/package.json
npm ERR! errno -2
npm ERR! enoent ENOENT: no such file or directory, open '/home/user/project/package.json'
npm ERR! enoent This is related to npm not being able to find a file.
npm ERR! enoent

npm ERR! A complete log of this run can be found in:
npm ERR!     /home/user/.npm/_logs/2024-01-01T00_00_00_000Z-debug-0.log
".into(), 1),

        "go_success" => (
            "".into(), 0),

        "go_failure" => {
            let mut out = String::new();
            for i in 1..=errors.max(1) {
                out.push_str(&format!("# github.com/user/project\nsrc/main.go:{}:2: undefined: x\n", 10 + i));
            }
            out.push_str("cannot find package \"example.com/pkg\" in any of:\n");
            (out, 1)
        },

        "tsc_success" => (
            "".into(), 0),

        "tsc_failure" => {
            let mut out = String::new();
            for i in 1..=errors.max(2) {
                out.push_str(&format!(
                    "src/app.ts({},5): error TS2322: Type 'number' is not assignable to type 'string'\n",
                    10 + i));
            }
            out.push_str("\nFound 2 errors.\n");
            (out, 2)
        },

        "python_success" => (
            "Hello, World!\n".into(), 0),

        "python_failure" => (
            "\
Traceback (most recent call last):
  File \"/home/user/script.py\", line 5, in <module>
    print(x)
NameError: name 'x' is not defined
".into(), 1),

        "make_success" => (
            "\
gcc -c -o main.o main.c
gcc -o program main.o
".into(), 0),

        "make_failure" => (
            "\
gcc -c -o main.o main.c
main.c:12:5: error: expected ';' before 'return'
make[1]: *** [Makefile:3: main.o] Error 1
make: *** [Makefile:2: all] Error 2
".into(), 2),

        "jest_success" => (
            "\
PASS src/App.test.js
  ✓ renders without crashing (12ms)

Test Suites: 1 passed, 1 total
Tests:       1 passed, 1 total
Snapshots:   0 total
Time:        1.234s
Ran all test suites.
".into(), 0),

        "jest_failure" => (
            "\
FAIL src/App.test.js
  ● renders without crashing

    expect(received).toBe(expected)

    Expected: true
    Received: false

      5 | test('renders without crashing', () => {
    > 6 |   expect(true).toBe(false);
        |               ^
      7 | });

  at Object.<anonymous> (src/App.test.js:6:15)

Test Suites: 1 failed, 1 total
Tests:       1 failed, 1 passed, 2 total
Snapshots:   0 total
Time:        1.456s
Ran all test suites.
".into(), 1),

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

    #[test]
    fn test_mock_gradle_success() {
        let (output, code) = generate_output("gradle_success", None, None);
        assert!(output.contains("BUILD SUCCESSFUL"));
        assert_eq!(code, 0);
    }

    #[test]
    fn test_mock_gradle_failure() {
        let (output, code) = generate_output("gradle_failure", Some(2), None);
        assert!(output.contains("BUILD FAILED"));
        assert!(output.contains("error:"));
        assert_eq!(code, 1);
    }

    #[test]
    fn test_mock_npm_success() {
        let (output, code) = generate_output("npm_success", None, None);
        assert!(output.contains("added"));
        assert_eq!(code, 0);
    }

    #[test]
    fn test_mock_npm_failure() {
        let (output, code) = generate_output("npm_failure", None, None);
        assert!(output.contains("npm ERR!"));
        assert_eq!(code, 1);
    }

    #[test]
    fn test_mock_go_success() {
        let (output, code) = generate_output("go_success", None, None);
        assert!(output.is_empty());
        assert_eq!(code, 0);
    }

    #[test]
    fn test_mock_go_failure() {
        let (output, code) = generate_output("go_failure", Some(1), None);
        assert!(output.contains("undefined:"));
        assert!(output.contains("cannot find package"));
        assert_eq!(code, 1);
    }

    #[test]
    fn test_mock_tsc_success() {
        let (output, code) = generate_output("tsc_success", None, None);
        assert!(output.is_empty());
        assert_eq!(code, 0);
    }

    #[test]
    fn test_mock_tsc_failure() {
        let (output, code) = generate_output("tsc_failure", Some(2), None);
        assert!(output.contains("error TS2322"));
        assert!(output.contains("Found 2 errors"));
        assert_eq!(code, 2);
    }

    #[test]
    fn test_mock_python_success() {
        let (output, code) = generate_output("python_success", None, None);
        assert!(output.contains("Hello, World!"));
        assert_eq!(code, 0);
    }

    #[test]
    fn test_mock_python_failure() {
        let (output, code) = generate_output("python_failure", None, None);
        assert!(output.contains("Traceback"));
        assert!(output.contains("NameError"));
        assert_eq!(code, 1);
    }

    #[test]
    fn test_mock_make_success() {
        let (output, code) = generate_output("make_success", None, None);
        assert!(output.contains("gcc"));
        assert_eq!(code, 0);
    }

    #[test]
    fn test_mock_make_failure() {
        let (output, code) = generate_output("make_failure", None, None);
        assert!(output.contains("***"));
        assert!(output.contains("Error 1"));
        assert_eq!(code, 2);
    }

    #[test]
    fn test_mock_jest_success() {
        let (output, code) = generate_output("jest_success", None, None);
        assert!(output.contains("PASS"));
        assert_eq!(code, 0);
    }

    #[test]
    fn test_mock_jest_failure() {
        let (output, code) = generate_output("jest_failure", None, None);
        assert!(output.contains("FAIL"));
        assert!(output.contains("Expected:"));
        assert!(output.contains("Received:"));
        assert_eq!(code, 1);
    }
}
