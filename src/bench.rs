//! Benchmark module for testing token compression.
//!
//! This module provides a `run_benchmark()` function that exercises several
//! mock-output scenarios and measures token reduction and parse latency.
//!
//! Only available in debug builds (`#[cfg(debug_assertions)]`).

use std::time::Instant;

use crate::core::SmolError;
use crate::parse;
use crate::config;

/// Result of a single benchmark scenario.
#[derive(Debug, Clone)]
pub struct BenchResult {
    /// Human-readable scenario name.
    pub scenario: String,
    /// Estimated tokens in the raw output.
    pub input_tokens: usize,
    /// Estimated tokens in the formatted summary.
    pub output_tokens: usize,
    /// Reduction percentage (0.0–100.0).
    pub reduction_pct: f64,
    /// Wall-clock time to parse and summarize in milliseconds.
    pub latency_ms: u64,
}

/// Run all benchmark scenarios and return results.
pub fn run_benchmark() -> Result<Vec<BenchResult>, SmolError> {
    // Load built-in parsers (use a dummy directory so only built-ins are loaded)
    let parsers = config::load_all_parsers("/nonexistent")?;

    let large_output = generate_large_output();

    let scenarios: Vec<(&str, &str, &str, &str)> = vec![
        (
            "maven_success",
            "mvn clean install",
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
",
            "",
        ),
        (
            "maven_failure",
            "mvn compile",
            "\
[INFO] Scanning for projects...
[INFO] --- maven-compiler-plugin:3.8.1:compile (default-compile) @ my-app ---
[ERROR] /src/main/java/App.java:42: error: cannot find symbol
[ERROR] /src/main/java/App.java:43: error: cannot find symbol
[ERROR] /src/main/java/App.java:44: error: cannot find symbol
[WARNING] /src/main/java/App.java:10: unchecked cast
[WARNING] /src/main/java/App.java:11: unchecked cast
[WARNING] /src/main/java/App.java:12: unchecked cast
[WARNING] /src/main/java/App.java:13: unchecked cast
[WARNING] /src/main/java/App.java:14: unchecked cast
[INFO] ------------------------------------------------------------------------
[INFO] BUILD FAILURE
[INFO] ------------------------------------------------------------------------
",
            "",
        ),
        (
            "cargo_errors",
            "cargo build",
            "\
   Compiling my-app v0.1.0 (/home/user/my-app)
error[E0425]: cannot find value `foo` in this scope
  --> src/main.rs:10:9
   |
10 |     let x = foo;
   |         ^^^ not found in this scope

error[E0425]: cannot find value `bar` in this scope
  --> src/main.rs:20:9
   |
20 |     let y = bar;
   |         ^^^ not found in this scope

warning: unused variable `x`
  --> src/main.rs:15:5

error: could not compile `my-app` due to previous error
",
            "",
        ),
        (
            "gcc_errors",
            "gcc -c test.c",
            "\
test.c:10:5: error: expected ';' before 'return'
test.c:11:5: error: expected ';' before 'return'
test.c:15:3: warning: implicit declaration of function 'foo'
",
            "",
        ),
        (
            "large_output",
            "make all",
            &large_output,
            "",
        ),
    ];

    let mut results = Vec::new();

    for (name, command, stdout, stderr) in &scenarios {
        let start = Instant::now();

        let summary = parse::parse_output(
            command,
            stdout,
            stderr,
            &parsers,
            100,  // keep all errors
            100,  // keep all warnings
        )?;

        let elapsed_ms = start.elapsed().as_millis() as u64;

        // Use the formatted summary for output_tokens, but we can also
        // just read the already-set token fields from summary.
        let reduction_pct = if summary.input_tokens > 0 {
            ((1.0 - summary.output_tokens as f64 / summary.input_tokens as f64) * 100.0)
                .max(0.0)
        } else {
            0.0
        };

        results.push(BenchResult {
            scenario: name.to_string(),
            input_tokens: summary.input_tokens,
            output_tokens: summary.output_tokens,
            reduction_pct,
            latency_ms: elapsed_ms,
        });
    }

    Ok(results)
}

/// Generate a moderately large output to stress-test compression.
fn generate_large_output() -> String {
    let mut out = String::with_capacity(200_000);
    // Simulate a long build with repeated warnings/info lines
    for i in 0..2000 {
        out.push_str(&format!("[INFO] Processing file {} of 2000...\n", i + 1));
    }
    for i in 0..50 {
        out.push_str(&format!(
            "[WARNING] /src/foo.c:{}: unused variable 'x{}'\n",
            10 + i, i
        ));
    }
    for i in 0..20 {
        out.push_str(&format!(
            "[ERROR] /src/foo.c:{}: undefined reference to 'symbol_{}'\n",
            50 + i, i
        ));
    }
    out.push_str("[INFO] BUILD FAILED\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_runs_without_error() {
        let results = run_benchmark().unwrap();
        assert!(!results.is_empty(), "should have at least one scenario");
    }

    #[test]
    fn test_benchmark_all_scenarios_have_tokens() {
        let results = run_benchmark().unwrap();
        for r in &results {
            assert!(
                r.input_tokens > 0 || r.scenario == "maven_success",
                "{} should have input_tokens > 0",
                r.scenario
            );
            assert!(
                r.latency_ms < 1000,
                "{} took {}ms (>1s)",
                r.scenario,
                r.latency_ms
            );
        }
    }

    #[test]
    fn test_benchmark_maven_failure_reduces() {
        let results = run_benchmark().unwrap();
        let maven_res = results.iter().find(|r| r.scenario == "maven_failure").unwrap();
        // The summary should be smaller than the raw output
        assert!(
            maven_res.output_tokens <= maven_res.input_tokens,
            "maven_failure: output_tokens {} > input_tokens {}",
            maven_res.output_tokens,
            maven_res.input_tokens
        );
    }

    #[test]
    fn test_benchmark_large_output_significant_reduction() {
        let results = run_benchmark().unwrap();
        let large = results.iter().find(|r| r.scenario == "large_output").unwrap();
        // Large output should compress well (at least 50% reduction)
        assert!(
            large.reduction_pct > 50.0,
            "large_output reduction {}% should be >50%",
            large.reduction_pct
        );
    }

    #[test]
    fn test_generate_large_output_size() {
        let output = generate_large_output();
        assert!(output.len() > 50_000, "large output should be >50KB");
        assert!(output.contains("BUILD FAILED"));
    }
}
