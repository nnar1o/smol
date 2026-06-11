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
