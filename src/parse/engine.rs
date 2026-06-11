use regex::Regex;

use crate::core::{ParserConfig, Summary, SummaryStatus, ErrorLine, WarningLine, InfoLine, TestFailure, TestResult, SmolError};

/// Run a parser config against the combined output.
/// Returns a Summary with all matched errors, warnings, info lines.
pub fn run_parser(config: &ParserConfig, output: &str) -> Result<Summary, SmolError> {
    let mut summary = Summary::new();

    // Pre-compile all regexes
    let ignore_regexes: Vec<Regex> = config.ignore_patterns.iter()
        .filter_map(|p| Regex::new(p).ok())
        .collect();
    let error_regexes: Vec<(Regex, &crate::core::PatternEntry)> = config.error_patterns.iter()
        .filter_map(|p| Regex::new(&p.regex).ok().map(|r| (r, p)))
        .collect();
    let warning_regexes: Vec<(Regex, &crate::core::PatternEntry)> = config.warning_patterns.iter()
        .filter_map(|p| Regex::new(&p.regex).ok().map(|r| (r, p)))
        .collect();
    let info_regexes: Vec<(Regex, &crate::core::PatternEntry)> = config.info_patterns.iter()
        .filter_map(|p| Regex::new(&p.regex).ok().map(|r| (r, p)))
        .collect();
    let success_regexes: Vec<Regex> = config.status_patterns.success.iter()
        .filter_map(|p| Regex::new(&p.regex).ok())
        .collect();
    let failure_regexes: Vec<Regex> = config.status_patterns.failure.iter()
        .filter_map(|p| Regex::new(&p.regex).ok())
        .collect();

    let mut has_success = false;
    let mut has_failure = false;

    for (line_num, line) in output.lines().enumerate() {
        let line_num = line_num as u64 + 1;

        // Check ignore patterns first
        if ignore_regexes.iter().any(|r| r.is_match(line)) {
            continue;
        }

        // Check error patterns
        let mut matched = false;
        for (re, pattern) in &error_regexes {
            if let Some(caps) = re.captures(line) {
                let content = caps.name(&pattern.group).map_or(line, |m| m.as_str());
                let file = caps.name(&pattern.file_group).map(|m| m.as_str().to_string());
                let file_line = caps.name(&pattern.line_group).and_then(|m| m.as_str().parse::<u64>().ok());
                let column = caps.name(&pattern.column_group).and_then(|m| m.as_str().parse::<u64>().ok());

                summary.error_count += 1;
                summary.errors.push(ErrorLine {
                    line_num,
                    content: content.to_string(),
                    file,
                    file_line,
                    column,
                });
                matched = true;
                if pattern.is_fatal {
                    has_failure = true;
                }
                break;
            }
        }
        if matched { continue; }

        // Check warning patterns
        for (re, pattern) in &warning_regexes {
            if let Some(caps) = re.captures(line) {
                let content = caps.name(&pattern.group).map_or(line, |m| m.as_str());
                let file = caps.name(&pattern.file_group).map(|m| m.as_str().to_string());
                let file_line = caps.name(&pattern.line_group).and_then(|m| m.as_str().parse::<u64>().ok());

                summary.warning_count += 1;
                summary.warnings.push(WarningLine {
                    line_num,
                    content: content.to_string(),
                    file,
                    file_line,
                });
                matched = true;
                break;
            }
        }
        if matched { continue; }

        // Check info patterns
        for (re, _pattern) in &info_regexes {
            if re.is_match(line) {
                summary.info_count += 1;
                summary.infos.push(InfoLine {
                    line_num,
                    content: line.to_string(),
                });
                matched = true;
                break;
            }
        }
        if matched { continue; }

        // Check status patterns
        if !has_success && success_regexes.iter().any(|r| r.is_match(line)) {
            has_success = true;
        }
        if !has_failure && failure_regexes.iter().any(|r| r.is_match(line)) {
            has_failure = true;
        }
    }

    // Determine final status
    summary.status = if has_failure {
        SummaryStatus::Failure
    } else if has_success {
        SummaryStatus::Success
    } else if summary.error_count > 0 {
        SummaryStatus::Failure
    } else {
        SummaryStatus::Success
    };

    summary.total_lines = output.lines().count() as u64;
    summary.total_chars = output.len() as u64;

    // Extract test results if test_patterns are configured
    if !config.test_patterns.is_empty() {
        summary.tests = extract_test_results(output, config);
    }

    Ok(summary)
}

/// Extract test results from output by matching test patterns.
/// Returns `Some(TestResult)` if any summary pattern matched,
/// `None` if no test output was detected.
pub fn extract_test_results(output: &str, config: &ParserConfig) -> Option<TestResult> {
    let mut total: u32 = 0;
    let mut passed: u32 = 0;
    let mut failed: u32 = 0;
    let mut errors: u32 = 0;
    let mut skipped: u32 = 0;
    let mut failures = Vec::new();
    let mut found_test = false;

    for pattern in &config.test_patterns {
        let summary_re = match Regex::new(&pattern.summary_regex) {
            Ok(r) => r,
            Err(_) => continue,
        };

        // Extract test counts from each matching summary line
        for caps in summary_re.captures_iter(output) {
            found_test = true;

            if let Some(t) = caps.name("total").and_then(|m| m.as_str().parse::<u32>().ok()) {
                total = total.max(t);
            }
            if let Some(p) = caps.name("passed").and_then(|m| m.as_str().parse::<u32>().ok()) {
                passed = passed.max(p);
            }
            if let Some(f) = caps.name("failures").and_then(|m| m.as_str().parse::<u32>().ok()) {
                failed = failed.max(f);
            }
            if let Some(e) = caps.name("errors").and_then(|m| m.as_str().parse::<u32>().ok()) {
                errors = errors.max(e);
            }
            if let Some(s) = caps.name("skipped").and_then(|m| m.as_str().parse::<u32>().ok()) {
                skipped = skipped.max(s);
            }
        }

        // If total is not captured but we have passed + failed + skipped,
        // compute total as their sum.
        if total == 0 && (passed > 0 || failed > 0 || skipped > 0) {
            total = passed + failed + skipped;
        }

        // Extract individual failures
        if let Some(failure_re_opt) = &pattern.failure_regex {
            if let Ok(failure_re) = Regex::new(failure_re_opt) {
                for caps in failure_re.captures_iter(output) {
                    let suite = caps.name("suite").map_or("", |m| m.as_str());
                    let test = caps.name("test").map(|m| m.as_str());
                    let message = caps.name("message").map_or("", |m| m.as_str());

                    if !suite.is_empty() {
                        failures.push(TestFailure {
                            suite: suite.to_string(),
                            test: test.map(|s| s.to_string()),
                            message: message.to_string(),
                        });
                    }
                }
            }
        }
    }

    if found_test {
        Some(TestResult {
            total,
            passed,
            failed,
            errors,
            skipped,
            failures,
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::PatternType;

    fn maven_config() -> ParserConfig {
        ParserConfig {
            name: "maven".into(),
            detection: crate::core::DetectionConfig {
                command_prefix: Some("mvn".into()),
                heuristic: None,
            },
            ignore_patterns: vec![
                "^\\[INFO\\] ---".to_string(),
                "^Downloading".to_string(),
            ],
            error_patterns: vec![
                crate::core::PatternEntry {
                    regex: "^\\[ERROR\\]\\s+(.*)$".into(),
                    severity: PatternType::Error,
                    is_fatal: false,
                    group: "message".into(),
                    file_group: "file".into(),
                    line_group: "line".into(),
                    column_group: "column".into(),
                },
                crate::core::PatternEntry {
                    regex: "BUILD FAILURE".into(),
                    severity: PatternType::Error,
                    is_fatal: true,
                    group: "message".into(),
                    file_group: "file".into(),
                    line_group: "line".into(),
                    column_group: "column".into(),
                },
            ],
            warning_patterns: vec![
                crate::core::PatternEntry {
                    regex: "^\\[WARNING\\]\\s+(.*)$".into(),
                    severity: PatternType::Warning,
                    is_fatal: false,
                    group: "message".into(),
                    file_group: "file".into(),
                    line_group: "line".into(),
                    column_group: "column".into(),
                },
            ],
            info_patterns: vec![],
            status_patterns: crate::core::StatusPatterns {
                success: vec![
                    crate::core::StatusPattern { regex: "BUILD SUCCESS".into(), group: "message".into() },
                ],
                failure: vec![
                    crate::core::StatusPattern { regex: "BUILD FAILURE".into(), group: "message".into() },
                ],
            },
            summary: crate::core::SummaryConfig {
                max_errors: 3,
                max_warnings: 5,
                show_error_lines: true,
            },
            test_patterns: vec![],
        }
    }

    #[test]
    fn test_parse_maven_success() {
        let config = maven_config();
        let output = "\
[INFO] Scanning for projects...
[INFO] --- maven-compiler-plugin ---
[INFO] BUILD SUCCESS
";
        let summary = run_parser(&config, output).unwrap();
        assert_eq!(summary.status, SummaryStatus::Success);
        assert_eq!(summary.error_count, 0);
        assert_eq!(summary.warning_count, 0);
    }

    #[test]
    fn test_parse_maven_failure() {
        let config = maven_config();
        let output = "\
[INFO] Scanning for projects...
[ERROR] Failed to execute goal...
[WARNING] Some deprecated API
[ERROR] Compilation failure
[WARNING] Another warning
BUILD FAILURE
";
        let summary = run_parser(&config, output).unwrap();
        assert_eq!(summary.status, SummaryStatus::Failure);
        assert_eq!(summary.error_count, 3);  // 2x [ERROR] + 1x BUILD FAILURE
        assert_eq!(summary.warning_count, 2);
    }

    #[test]
    fn test_parse_ignore_patterns() {
        let config = maven_config();
        let output = "\
[INFO] --- maven-compiler-plugin ---
Downloading from central: https://...
[INFO] BUILD SUCCESS
";
        let summary = run_parser(&config, output).unwrap();
        // Ignored lines should not appear anywhere
        assert_eq!(summary.status, SummaryStatus::Success);
    }

    // ── Test result extraction tests ─────────────────────────────

    fn maven_test_config() -> ParserConfig {
        let mut cfg = maven_config();
        cfg.test_patterns = vec![
            crate::core::TestPattern {
                summary_regex: "Tests run:\\s*(?P<total>\\d+),\\s*Failures:\\s*(?P<failures>\\d+),\\s*Errors:\\s*(?P<errors>\\d+),\\s*Skipped:\\s*(?P<skipped>\\d+)".into(),
                failure_regex: Some("(?P<suite>[\\w.]+)\\s+>\\s+(?P<test>\\w+)".into()),
            },
        ];
        cfg
    }

    fn cargo_test_config() -> ParserConfig {
        let mut cfg = maven_config();
        cfg.name = "cargo".into();
        cfg.test_patterns = vec![
            crate::core::TestPattern {
                summary_regex: "test result:\\s*\\w+\\.\\s*(?P<passed>\\d+) passed;\\s*(?P<failures>\\d+) failed;\\s*(?P<skipped>\\d+) (?:ignored|skipped)".into(),
                failure_regex: Some("test\\s+(?P<suite>\\S+)\\s+\\.\\.\\.\\s+FAILED".into()),
            },
        ];
        cfg
    }

    fn jest_test_config() -> ParserConfig {
        let mut cfg = maven_config();
        cfg.name = "jest".into();
        cfg.test_patterns = vec![
            crate::core::TestPattern {
                summary_regex: "Tests:\\s*(?:(?P<failures>\\d+)\\s+failed)?,?\\s*(?:(?P<passed>\\d+)\\s+passed)?,?\\s*(?:(?P<total>\\d+)\\s+total)?".into(),
                failure_regex: Some("FAIL\\s+(?P<suite>\\S+)".into()),
            },
        ];
        cfg
    }

    #[test]
    fn test_extract_maven_surefire_success() {
        let config = maven_test_config();
        let output = "\
[INFO] --- maven-surefire-plugin ---
[INFO] Running com.example.MyTest
[INFO] Tests run: 10, Failures: 0, Errors: 0, Skipped: 2, Time elapsed: 0.123s
[INFO] BUILD SUCCESS
";
        let summary = run_parser(&config, output).unwrap();
        let tests = summary.tests.expect("test results should be present");
        assert_eq!(tests.total, 10);
        assert_eq!(tests.passed, 0);
        assert_eq!(tests.failed, 0);
        assert_eq!(tests.errors, 0);
        assert_eq!(tests.skipped, 2);
        assert!(tests.failures.is_empty());
    }

    #[test]
    fn test_extract_maven_surefire_failures() {
        let config = maven_test_config();
        let output = "\
-------------------------------------------------------
 T E S T S
-------------------------------------------------------
Running com.example.MyTest
Tests run: 3, Failures: 1, Errors: 0, Skipped: 0, Time elapsed: 0.045s
Running com.example.OtherTest
Tests run: 7, Failures: 1, Errors: 0, Skipped: 1, Time elapsed: 0.078s
BUILD FAILURE

Failed tests:
  com.example.MyTest > testMethod
  com.example.OtherTest > otherMethod
";
        let summary = run_parser(&config, output).unwrap();
        let tests = summary.tests.expect("test results should be present");
        // Uses max across all summary lines: total=7, failures=1, errors=0, skipped=1
        assert_eq!(tests.total, 7);
        assert_eq!(tests.failed, 1);
        assert_eq!(tests.errors, 0);
        assert_eq!(tests.skipped, 1);
        // Two individual failures captured
        assert_eq!(tests.failures.len(), 2);
        assert_eq!(tests.failures[0].suite, "com.example.MyTest");
        assert_eq!(tests.failures[0].test.as_deref(), Some("testMethod"));
        assert_eq!(tests.failures[1].suite, "com.example.OtherTest");
        assert_eq!(tests.failures[1].test.as_deref(), Some("otherMethod"));
    }

    #[test]
    fn test_extract_cargo_test_success() {
        let config = cargo_test_config();
        let output = "\
   Compiling my-app v0.1.0 (/home/user/my-app)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.42s
     Running unittests src/lib.rs

running 10 tests
test test_foo ... ok
test test_bar ... ok
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
";
        let summary = run_parser(&config, output).unwrap();
        let tests = summary.tests.expect("test results should be present");
        assert_eq!(tests.total, 10);
        assert_eq!(tests.passed, 10);
        assert_eq!(tests.failed, 0);
        assert_eq!(tests.errors, 0);
        assert_eq!(tests.skipped, 0);
        assert!(tests.failures.is_empty());
    }

    #[test]
    fn test_extract_cargo_test_failure() {
        let config = cargo_test_config();
        let output = "\
   Compiling my-app v0.1.0 (/home/user/my-app)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.42s
     Running unittests src/lib.rs

running 3 tests
test test_foo ... ok
test test_bar ... FAILED
test test_baz ... ok
test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
";
        let summary = run_parser(&config, output).unwrap();
        let tests = summary.tests.expect("test results should be present");
        assert_eq!(tests.total, 3);
        assert_eq!(tests.passed, 2);
        assert_eq!(tests.failed, 1);
        assert_eq!(tests.errors, 0);
        assert_eq!(tests.skipped, 0);
        assert_eq!(tests.failures.len(), 1);
        assert_eq!(tests.failures[0].suite, "test_bar");
    }

    #[test]
    fn test_extract_jest_test_results() {
        let config = jest_test_config();
        let output = "\
FAIL src/App.test.js
  ● renders without crashing
  expect(received).toBe(expected)

  Expected: true
  Received: false

PASS src/utils.test.js
Test Suites: 1 failed, 1 passed, 2 total
Tests:       1 failed, 2 passed, 3 total
Snapshots:   0 total
Time:        2.456s
";
        let summary = run_parser(&config, output).unwrap();
        let tests = summary.tests.expect("test results should be present");
        assert_eq!(tests.total, 3);
        assert_eq!(tests.passed, 2);
        assert_eq!(tests.failed, 1);
        assert_eq!(tests.errors, 0);
        assert_eq!(tests.skipped, 0);
        assert_eq!(tests.failures.len(), 1);
        assert_eq!(tests.failures[0].suite, "src/App.test.js");
    }

    #[test]
    fn test_extract_no_test_output_returns_none() {
        let mut config = maven_config();
        config.test_patterns = vec![
            crate::core::TestPattern {
                summary_regex: "Tests run:\\s*(?P<total>\\d+),\\s*Failures:\\s*(?P<failures>\\d+),\\s*Errors:\\s*(?P<errors>\\d+),\\s*Skipped:\\s*(?P<skipped>\\d+)".into(),
                failure_regex: None,
            },
        ];
        let output = "\
[INFO] Scanning for projects...
[INFO] BUILD SUCCESS
";
        let summary = run_parser(&config, output).unwrap();
        assert!(summary.tests.is_none(), "expected no test results for non-test output");
    }

    #[test]
    fn test_extract_test_results_no_patterns_returns_none() {
        let config = maven_config(); // no test_patterns set
        let output = "\
Tests run: 10, Failures: 0, Errors: 0, Skipped: 0
";
        let summary = run_parser(&config, output).unwrap();
        assert!(summary.tests.is_none(), "expected no test results when no patterns configured");
    }
}
