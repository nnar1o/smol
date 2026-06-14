use smol::core::{Summary, SummaryStatus, ErrorLine, WarningLine};
use smol::parse::summarizer;

/// Test formatting an empty summary.
#[test]
fn test_format_empty_summary() {
    let s = Summary::default();
    let out = summarizer::format_summary(&s);
    assert_eq!(out, "done");
}

/// Test formatting a success summary with errors but no warnings.
#[test]
fn test_format_success_with_errors() {
    let mut s = Summary {
        status: SummaryStatus::Success,
        error_count: 3,
        ..Default::default()
    };
    s.errors.push(ErrorLine {
        line_num: 1,
        content: "error 1".into(),
        file: Some("test.rs".into()),
        file_line: Some(42),
        column: None,
    });
    let out = summarizer::format_summary(&s);
    assert!(out.contains("success"));
    assert!(out.contains("errors:3"));
    assert!(out.contains("test.rs:42"));
}

/// Test formatting with very large token counts.
#[test]
fn test_format_large_tokens() {
    let s = Summary {
        status: SummaryStatus::Success,
        input_tokens: 1_000_000,
        output_tokens: 50_000,
        ..Default::default()
    };
    let out = summarizer::format_summary(&s);
    assert!(out.starts_with("success"));
}

/// Test formatting where tokens expand (output > input).
#[test]
fn test_format_token_expansion() {
    let s = Summary {
        status: SummaryStatus::Failure,
        input_tokens: 10,
        output_tokens: 50,
        ..Default::default()
    };
    let out = summarizer::format_summary(&s);
    assert!(out.starts_with("failure"));
}

/// Test trim_summary with empty summary.
#[test]
fn test_trim_empty_summary() {
    let s = Summary::default();
    let trimmed = summarizer::trim_summary(s, 3, 5);
    assert_eq!(trimmed.errors.len(), 0);
    assert_eq!(trimmed.warnings.len(), 0);
}

/// Test trim_summary with errors only.
#[test]
fn test_trim_errors_only() {
    let mut s = Summary::default();
    for i in 0..10 {
        s.errors.push(ErrorLine {
            line_num: i,
            content: format!("error {}", i),
            file: None,
            file_line: None,
            column: None,
        });
    }
    s.error_count = 10;
    let trimmed = summarizer::trim_summary(s, 2, 5);
    assert_eq!(trimmed.errors.len(), 2);
    assert_eq!(trimmed.warnings.len(), 0);
}

/// Test trim_summary with warnings only.
#[test]
fn test_trim_warnings_only() {
    let mut s = Summary::default();
    for i in 0..10 {
        s.warnings.push(WarningLine {
            line_num: i,
            content: format!("warning {}", i),
            file: None,
            file_line: None,
        });
    }
    s.warning_count = 10;
    let trimmed = summarizer::trim_summary(s, 3, 3);
    assert_eq!(trimmed.warnings.len(), 3);
    assert_eq!(trimmed.errors.len(), 0);
}

/// Test format_summary_json produces valid JSON.
#[test]
fn test_format_summary_json() {
    let s = Summary::default();
    let json = summarizer::format_summary_json(&s);
    assert!(json.starts_with('{'));
    assert!(json.ends_with('}'));
    assert!(json.contains("\"status\""));
    assert!(json.contains("\"error_count\""));
    assert!(json.contains("\"warning_count\""));
    assert!(json.contains("\"total_lines\""));
}

/// Test format_summary_json with populated summary.
#[test]
fn test_format_summary_json_populated() {
    let s = Summary {
        status: SummaryStatus::Success,
        error_count: 2,
        warning_count: 1,
        total_lines: 50,
        total_chars: 1000,
        input_tokens: 250,
        output_tokens: 10,
        compression_ratio: 0.04,
        ..Default::default()
    };
    let json = summarizer::format_summary_json(&s);
    assert!(json.contains("\"error_count\": 2"));
    assert!(json.contains("\"warning_count\": 1"));
    assert!(json.contains("\"input_tokens\": 250"));
    assert!(json.contains("\"compression_ratio\": 0.04"));
}

/// Test formatting summary with no file info in errors.
#[test]
fn test_format_errors_without_file() {
    let mut s = Summary {
        status: SummaryStatus::Failure,
        error_count: 1,
        ..Default::default()
    };
    s.errors.push(ErrorLine {
        line_num: 1,
        content: "generic error".into(),
        file: None,
        file_line: None,
        column: None,
    });
    let out = summarizer::format_summary(&s);
    assert!(out.contains("- generic error"));
}

/// Test formatting summary with multiple error files.
#[test]
fn test_format_multiple_error_files() {
    let mut s = Summary {
        status: SummaryStatus::Failure,
        error_count: 3,
        ..Default::default()
    };
    s.errors.push(ErrorLine {
        line_num: 1,
        content: "not found".into(),
        file: Some("main.rs".into()),
        file_line: Some(10),
        column: None,
    });
    s.errors.push(ErrorLine {
        line_num: 2,
        content: "type mismatch".into(),
        file: Some("lib.rs".into()),
        file_line: Some(25),
        column: Some(5),
    });
    s.errors.push(ErrorLine {
        line_num: 3,
        content: "missing semicolon".into(),
        file: None,
        file_line: None,
        column: None,
    });
    let out = summarizer::format_summary(&s);
    assert!(out.contains("main.rs:10"));
    assert!(out.contains("lib.rs:25"));
    assert!(out.contains("- missing semicolon"));
}
