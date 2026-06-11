use regex::Regex;

use crate::core::{Summary, SummaryStatus, ErrorLine, WarningLine};

/// Generic fallback parser for unknown commands.
/// Detects errors/warnings by common patterns found in most build tools and scripts.
pub fn parse_generic(output: &str, max_errors: usize, max_warnings: usize) -> Summary {
    let mut summary = Summary::new();

    // Common error patterns (GCC/Clang style, generic ERROR, etc.)
    let error_patterns: Vec<Regex> = vec![
        Regex::new(r"^(.+?):(\d+):(\d+):\s+error:\s+(.+)$"),
        Regex::new(r"^(.+?):(\d+):\s+error:\s+(.+)$"),
        Regex::new(r"^(.+?)\((\d+)\)\s*:\s*error\s+(.+)$"),
        Regex::new(r"^(?:\[)?\s*(?:ERROR|Error|error)\b[\]:]?\s*(.+)$"),
        Regex::new(r"^(?:.*\b)?(?:FAIL|Failed|failure)\b"),
        Regex::new(r"^Traceback \(most recent call last\)"),
        Regex::new(r"^error\[E\d+\]:\s+(.+)$"),
    ].into_iter().filter_map(|r| r.ok()).collect();

    // Common warning patterns
    let warning_patterns: Vec<Regex> = vec![
        Regex::new(r"^(.+?):(\d+):(\d+):\s+warning:\s+(.+)$"),
        Regex::new(r"^(.+?):(\d+):\s+warning:\s+(.+)$"),
        Regex::new(r"^(?:\[)?\s*(?:WARNING|Warning|warning)\b[\]:]?\s*(.+)$"),
    ].into_iter().filter_map(|r| r.ok()).collect();

    for (line_num, line) in output.lines().enumerate() {
        let line_num = line_num as u64 + 1;

        // Check errors
        for re in &error_patterns {
            if let Some(caps) = re.captures(line) {
                let file = caps.get(1).map(|m| m.as_str().to_string());
                let file_line = caps.get(2).and_then(|m| m.as_str().parse::<u64>().ok());
                let column = caps.get(3).and_then(|m| {
                    // Check if this is actually a column or part of the message
                    let s = m.as_str();
                    if s.chars().all(|c| c.is_ascii_digit()) {
                        s.parse::<u64>().ok()
                    } else {
                        None
                    }
                });
                // The message is in the last capture group
                let last_group = caps.len() - 1;
                let content = caps.get(last_group).map_or(line, |m| m.as_str());

                summary.error_count += 1;
                summary.errors.push(ErrorLine {
                    line_num,
                    content: content.to_string(),
                    file,
                    file_line,
                    column,
                });
                break;
            }
        }

        // Check warnings (if not already matched as error)
        for re in &warning_patterns {
            if let Some(caps) = re.captures(line) {
                let file = caps.get(1).map(|m| m.as_str().to_string());
                let file_line = caps.get(2).and_then(|m| m.as_str().parse::<u64>().ok());
                let last_group = caps.len() - 1;
                let content = caps.get(last_group).map_or(line, |m| m.as_str());

                summary.warning_count += 1;
                summary.warnings.push(WarningLine {
                    line_num,
                    content: content.to_string(),
                    file,
                    file_line,
                });
                break;
            }
        }
    }

    // Determine status
    summary.status = if summary.error_count > 0 {
        SummaryStatus::Failure
    } else {
        SummaryStatus::Unknown
    };

    summary.total_lines = output.lines().count() as u64;
    summary.total_chars = output.len() as u64;

    // Apply limits
    summary = crate::parse::summarizer::trim_summary(summary, max_errors, max_warnings);

    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generic_parse_gcc_error() {
        let output = "test.c:42:5: error: expected ';' before 'return'";
        let summary = parse_generic(output, 3, 5);
        assert_eq!(summary.status, SummaryStatus::Failure);
        assert_eq!(summary.error_count, 1);
        assert_eq!(summary.errors[0].file.as_deref(), Some("test.c"));
        assert_eq!(summary.errors[0].file_line, Some(42));
    }

    #[test]
    fn test_generic_parse_no_errors() {
        let output = "All good!\nBuild completed successfully.";
        let summary = parse_generic(output, 3, 5);
        assert_eq!(summary.error_count, 0);
        assert_eq!(summary.warning_count, 0);
    }

    #[test]
    fn test_generic_parse_warning() {
        let output = "src/lib.rs:15: warning: unused variable";
        let summary = parse_generic(output, 3, 5);
        assert_eq!(summary.warning_count, 1);
    }

    #[test]
    fn test_generic_parse_error_limit() {
        let output = (1..=10).map(|i| format!("file.rs:{}: error: error {}", i, i)).collect::<Vec<_>>().join("\n");
        let summary = parse_generic(&output, 3, 5);
        assert_eq!(summary.error_count, 10);
        assert_eq!(summary.errors.len(), 3); // trimmed to max_errors
    }
}
