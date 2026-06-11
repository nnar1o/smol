use smol::parse::generic;
use smol::core::SummaryStatus;

/// Test generic parser with GCC-style errors.
#[test]
fn test_generic_parse_gcc_style() {
    let output = "test.c:42:5: error: expected ';' before 'return'";
    let summary = generic::parse_generic(output, 3, 5);

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert_eq!(summary.error_count, 1);
    assert_eq!(summary.errors[0].file.as_deref(), Some("test.c"));
    assert_eq!(summary.errors[0].file_line, Some(42));
}

/// Test generic parser with no errors or warnings.
#[test]
fn test_generic_parse_no_errors() {
    let output = "All good!\nBuild completed successfully.";
    let summary = generic::parse_generic(output, 3, 5);

    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.warning_count, 0);
}

/// Test generic parser with warning patterns.
#[test]
fn test_generic_parse_warning() {
    let output = "src/lib.rs:15: warning: unused variable";
    let summary = generic::parse_generic(output, 3, 5);

    assert_eq!(summary.warning_count, 1);
}

/// Test generic parser with ERROR lines.
#[test]
fn test_generic_parse_error_keyword() {
    let output = "ERROR: something went wrong\n[ERROR] build failed";
    let summary = generic::parse_generic(output, 3, 5);

    assert_eq!(summary.error_count, 2);
}

/// Test generic parser with Python traceback.
#[test]
fn test_generic_parse_traceback() {
    let output = "\
Traceback (most recent call last):
  File \"script.py\", line 5, in <module>
    print(x)
NameError: name 'x' is not defined
";
    let summary = generic::parse_generic(output, 5, 5);

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count > 0);
}

/// Test generic parser error limit.
#[test]
fn test_generic_parse_error_limit() {
    let output = (1..=10)
        .map(|i| format!("file.rs:{}: error: error {}", i, i))
        .collect::<Vec<_>>()
        .join("\n");
    let summary = generic::parse_generic(&output, 3, 5);

    // error_count should be 10 but errors.len() should be 3 (trimmed)
    assert_eq!(summary.error_count, 10);
    assert_eq!(summary.errors.len(), 3);
}

/// Test generic parser warning limit.
#[test]
fn test_generic_parse_warning_limit() {
    let output = (1..=10)
        .map(|i| format!("warning: warning number {}", i))
        .collect::<Vec<_>>()
        .join("\n");
    let summary = generic::parse_generic(&output, 3, 3);

    assert_eq!(summary.warning_count, 10);
    assert_eq!(summary.warnings.len(), 3);
}

/// Test generic parser with mixed errors and warnings.
#[test]
fn test_generic_parse_mixed() {
    let output = "\
file.rs:10: error: type mismatch
file.rs:15: warning: unused variable
file.rs:20: error: missing semicolon
file.rs:25: warning: deprecated function
";
    let summary = generic::parse_generic(output, 5, 5);

    assert_eq!(summary.error_count, 2);
    assert_eq!(summary.warning_count, 2);
    assert_eq!(summary.status, SummaryStatus::Failure);
}

/// Test generic parser with "FAIL" keyword.
#[test]
fn test_generic_parse_fail_keyword() {
    let output = "FAIL: build did not complete";
    let summary = generic::parse_generic(output, 5, 5);

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert_eq!(summary.error_count, 1);
}

/// Test generic parser with "Failed" keyword.
#[test]
fn test_generic_parse_failed_keyword() {
    let output = "Build Failed";
    let summary = generic::parse_generic(output, 5, 5);

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert_eq!(summary.error_count, 1);
}
