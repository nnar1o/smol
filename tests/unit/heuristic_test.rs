use std::collections::HashMap;
use smol::core::{ParserConfig, DetectionConfig, SummaryConfig, StatusPatterns};
use smol::core::parser_config::HeuristicConfig;

/// Build a minimal parser config with a heuristic.
fn heuristic_parser(name: &str, regex: &str, min_lines: u32) -> ParserConfig {
    ParserConfig {
        name: name.to_string(),
        detection: DetectionConfig {
            command_prefix: None,
            heuristic: Some(HeuristicConfig {
                regex: regex.to_string(),
                min_lines,
            }),
        },
        ignore_patterns: vec![],
        error_patterns: vec![],
        warning_patterns: vec![],
        info_patterns: vec![],
        status_patterns: StatusPatterns {
            success: vec![],
            failure: vec![],
        },
        summary: SummaryConfig {
            max_errors: 3,
            max_warnings: 5,
            show_error_lines: true,
        },
        test_patterns: vec![],
    }
}

/// Test heuristic matching with exact regex.
#[test]
fn test_heuristic_exact_match() {
    let mut parsers = HashMap::new();
    parsers.insert("test".into(), heuristic_parser("test", r"ERROR", 1));

    let output = "Line 1\nERROR: something broke\nLine 3\n";
    let detected = smol::parse::detector::detect_by_heuristic(output, &parsers);
    assert!(detected.is_some());
    assert_eq!(detected.unwrap().name, "test");
}

/// Test heuristic with minimum lines requirement not met.
#[test]
fn test_heuristic_min_lines_not_met() {
    let mut parsers = HashMap::new();
    parsers.insert("test".into(), heuristic_parser("test", r"ERROR", 5));

    let output = "ERROR: only one error line\n";
    let detected = smol::parse::detector::detect_by_heuristic(output, &parsers);
    assert!(detected.is_none());
}

/// Test heuristic with no matching lines.
#[test]
fn test_heuristic_no_match() {
    let mut parsers = HashMap::new();
    parsers.insert("test".into(), heuristic_parser("test", r"ZZZ_NOT_FOUND_ZZZ", 1));

    let output = "Some random output without matches.\n";
    let detected = smol::parse::detector::detect_by_heuristic(output, &parsers);
    assert!(detected.is_none());
}

/// Test multiple heuristics pick the best match.
#[test]
fn test_heuristic_best_match_wins() {
    let mut parsers = HashMap::new();
    parsers.insert("low".into(), heuristic_parser("low", r"INFO", 1));
    parsers.insert("high".into(), heuristic_parser("high", r"ERROR", 1));

    let output = "\
INFO: processing file 1
INFO: processing file 2
ERROR: something went wrong
";
    let detected = smol::parse::detector::detect_by_heuristic(output, &parsers);
    // ERROR has 1/3 lines, INFO has 2/3 lines
    assert!(detected.is_some());
    // The one with higher ratio wins (INFO)
    assert_eq!(detected.unwrap().name, "low", "INFO has higher match density");
}

/// Test heuristic with multi-line regex.
#[test]
fn test_heuristic_multiline_regex() {
    let mut parsers = HashMap::new();
    parsers.insert("traceback".into(), heuristic_parser("traceback", r"Traceback", 1));

    let output = "\
Some output
Traceback (most recent call last):
  File \"script.py\", line 5
    print(x)
NameError: name 'x' is not defined
";
    let detected = smol::parse::detector::detect_by_heuristic(output, &parsers);
    assert!(detected.is_some());
    assert_eq!(detected.unwrap().name, "traceback");
}

/// Test heuristic with empty output.
#[test]
fn test_heuristic_empty_output() {
    let mut parsers = HashMap::new();
    parsers.insert("test".into(), heuristic_parser("test", r"ERROR", 1));

    let detected = smol::parse::detector::detect_by_heuristic("", &parsers);
    assert!(detected.is_none());
}

/// Test heuristic with single-line output matching.
#[test]
fn test_heuristic_single_line_match() {
    let mut parsers = HashMap::new();
    parsers.insert("match".into(), heuristic_parser("match", r"^hello", 1));

    let output = "hello world";
    let detected = smol::parse::detector::detect_by_heuristic(output, &parsers);
    assert!(detected.is_some());
    assert_eq!(detected.unwrap().name, "match");
}

/// Test that regex with compilation error is silently skipped.
#[test]
fn test_heuristic_invalid_regex() {
    let mut parsers = HashMap::new();
    // Invalid regex
    parsers.insert("bad".into(), heuristic_parser("bad", r"[invalid", 1));

    let output = "some output";
    // Should not panic - invalid regex is skipped
    let detected = smol::parse::detector::detect_by_heuristic(output, &parsers);
    assert!(detected.is_none());
}

/// Test heuristic with backslash-heavy regex (common in parser configs).
#[test]
fn test_heuristic_escaped_regex() {
    let mut parsers = HashMap::new();
    // Maven-like pattern with escaped brackets
    parsers.insert("maven".into(), heuristic_parser("maven", r"\[INFO\]", 1));

    let output = "\
[INFO] Scanning for projects...
[INFO] Compiling...
[INFO] BUILD SUCCESS
";
    let detected = smol::parse::detector::detect_by_heuristic(output, &parsers);
    assert!(detected.is_some());
    assert_eq!(detected.unwrap().name, "maven");
}
