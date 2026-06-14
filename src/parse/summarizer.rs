use crate::core::{Summary, SummaryStatus};

/// Trim the errors and warnings lists to the specified maximums.
pub fn trim_summary(mut summary: Summary, max_errors: usize, max_warnings: usize) -> Summary {
    if summary.errors.len() > max_errors {
        summary.errors.truncate(max_errors);
    }
    if summary.warnings.len() > max_warnings {
        summary.warnings.truncate(max_warnings);
    }
    summary
}

/// Format a summary into a human-readable string.
pub fn format_summary(summary: &Summary) -> String {
    let mut parts = Vec::new();

    match summary.status {
        SummaryStatus::Success => parts.push("success".to_string()),
        SummaryStatus::Failure => parts.push("failure".to_string()),
        SummaryStatus::Unknown => parts.push("done".to_string()),
    }

    if summary.error_count > 0 {
        parts.push(format!("errors:{}", summary.error_count));
    }
    if summary.warning_count > 0 {
        parts.push(format!("warnings:{}", summary.warning_count));
    }

    let mut result = parts.join(" ");

    // Append test results if present
    if let Some(ref tests) = summary.tests {
        // Build a readable test summary showing only non-zero counts
        let mut test_parts: Vec<String> = Vec::new();
        if tests.passed > 0 {
            test_parts.push(format!("{} passed", tests.passed));
        }
        if tests.failed > 0 {
            test_parts.push(format!("{} failed", tests.failed));
        }
        if tests.errors > 0 {
            test_parts.push(format!("{} errors", tests.errors));
        }
        if tests.skipped > 0 {
            test_parts.push(format!("{} skipped", tests.skipped));
        }
        if test_parts.is_empty() && tests.total > 0 {
            test_parts.push(format!("{} total", tests.total));
        }

        if !test_parts.is_empty() {
            result.push_str(&format!(
                "\ntests: {} of {}",
                test_parts.join(", "),
                tests.total
            ));
        }

        // Show individual test failures
        if !tests.failures.is_empty() {
            for failure in &tests.failures {
                let test_name = failure.test.as_deref().unwrap_or("");
                if !test_name.is_empty() {
                    result.push_str(&format!(
                        "\n  FAIL {}::{}",
                        failure.suite,
                        test_name,
                    ));
                } else if !failure.message.is_empty() {
                    result.push_str(&format!(
                        "\n  FAIL {}: {}",
                        failure.suite,
                        failure.message,
                    ));
                } else {
                    result.push_str(&format!(
                        "\n  FAIL {}",
                        failure.suite,
                    ));
                }
            }
        }
    }

    // Add first few error lines if present
    if !summary.errors.is_empty() {
        result.push('\n');
        for err in &summary.errors {
            if let Some(ref file) = err.file {
                if let Some(line) = err.file_line {
                    result.push_str(&format!("\n  {}:{}: {}", file, line, err.content));
                } else {
                    result.push_str(&format!("\n  {}: {}", file, err.content));
                }
            } else {
                result.push_str(&format!("\n  - {}", err.content));
            }
        }
    }

    result
}

/// Format summary for JSON output (stats mode).
pub fn format_summary_json(summary: &Summary) -> String {
    serde_json::to_string_pretty(summary).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{ErrorLine, WarningLine};

    fn sample_summary() -> Summary {
        Summary {
            status: SummaryStatus::Failure,
            error_count: 5,
            warning_count: 3,
            info_count: 0,
            total_lines: 100,
            total_chars: 5000,
            errors: vec![
                ErrorLine { line_num: 10, content: "undefined variable".into(), file: Some("main.rs".into()), file_line: Some(42), column: None },
                ErrorLine { line_num: 20, content: "type mismatch".into(), file: Some("main.rs".into()), file_line: Some(55), column: Some(10) },
                ErrorLine { line_num: 30, content: "missing semicolon".into(), file: None, file_line: None, column: None },
            ],
            warnings: vec![
                WarningLine { line_num: 5, content: "unused variable".into(), file: Some("lib.rs".into()), file_line: Some(15) },
            ],
            infos: vec![],
            truncated: false,
            truncated_count: None,
            input_tokens: 0,
            output_tokens: 0,
            compression_ratio: 1.0,
            tests: None,
        }
    }

    #[test]
    fn test_format_summary_success() {
        let s = Summary {
            status: SummaryStatus::Success,
            ..Default::default()
        };
        let out = format_summary(&s);
        assert_eq!(out, "success");
    }

    #[test]
    fn test_format_summary_with_reduction() {
        let s = Summary {
            status: SummaryStatus::Success,
            input_tokens: 100,
            output_tokens: 25,
            compression_ratio: 0.25,
            ..Default::default()
        };
        let out = format_summary(&s);
        assert_eq!(out, "success");
    }

    #[test]
    fn test_format_summary_with_expansion() {
        let s = Summary {
            status: SummaryStatus::Success,
            input_tokens: 10,
            output_tokens: 40,
            compression_ratio: 4.0,
            ..Default::default()
        };
        let out = format_summary(&s);
        assert_eq!(out, "success");
    }

    #[test]
    fn test_format_summary_failure() {
        let s = sample_summary();
        let out = format_summary(&s);
        assert!(out.contains("failure"));
        assert!(out.contains("errors:5"));
        assert!(out.contains("warnings:3"));
        assert!(out.contains("main.rs:42"));
    }

    #[test]
    fn test_trim_summary_errors() {
        let s = sample_summary();
        assert_eq!(s.errors.len(), 3);
        let trimmed = trim_summary(s, 1, 5);
        assert_eq!(trimmed.errors.len(), 1);
    }

    #[test]
    fn test_trim_summary_warnings() {
        let mut s = sample_summary();
        s.warnings = (0..10).map(|i| WarningLine {
            line_num: i as u64,
            content: format!("warn {}", i),
            file: None,
            file_line: None,
        }).collect();
        let trimmed = trim_summary(s, 3, 2);
        assert_eq!(trimmed.warnings.len(), 2);
    }

    #[test]
    fn test_format_summary_json() {
        let s = sample_summary();
        let json = format_summary_json(&s);
        assert!(json.contains("\"error_count\": 5"));
        assert!(json.contains("\"warning_count\": 3"));
    }
}
