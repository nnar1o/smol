use std::collections::HashMap;
use smol::core::{ParserConfig, DetectionConfig, SummaryConfig, StatusPatterns};
use smol::core::parser_config::HeuristicConfig;
use smol::parse::detector;

/// Helper to create a simple parser config for testing.
fn make_parser(name: &str, prefix: Option<&str>, heuristic_regex: Option<&str>) -> ParserConfig {
    ParserConfig {
        name: name.to_string(),
        detection: DetectionConfig {
            command_prefix: prefix.map(|s| s.to_string()),
            heuristic: heuristic_regex.map(|re| HeuristicConfig {
                regex: re.to_string(),
                min_lines: 1,
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

/// Test detection by exact command prefix match.
#[test]
fn test_detect_by_command_exact() {
    let mut parsers = HashMap::new();
    parsers.insert("maven".into(), make_parser("maven", Some("mvn"), None));
    parsers.insert("cargo".into(), make_parser("cargo", Some("cargo"), None));

    let detected = detector::detect_by_command("mvn clean install", &parsers);
    assert!(detected.is_some());
    assert_eq!(detected.unwrap().name, "maven");
}

/// Test detection by command prefix when there are multiple parsers.
#[test]
fn test_detect_by_command_with_multiple_parsers() {
    let mut parsers = HashMap::new();
    parsers.insert("gcc".into(), make_parser("gcc", Some("gcc"), None));
    parsers.insert("gxx".into(), make_parser("g++", Some("g++"), None));
    parsers.insert("clang".into(), make_parser("clang", Some("clang"), None));

    let detected = detector::detect_by_command("clang -O2 -c test.c", &parsers);
    assert!(detected.is_some());
    assert_eq!(detected.unwrap().name, "clang");
}

/// Test detection returns None for unknown commands.
#[test]
fn test_detect_by_command_no_match() {
    let mut parsers = HashMap::new();
    parsers.insert("maven".into(), make_parser("maven", Some("mvn"), None));

    let detected = detector::detect_by_command("ls -la", &parsers);
    assert!(detected.is_none());
}

/// Test prefix matching: "mvn" should match "./mvnw".
#[test]
fn test_detect_by_command_prefix_match() {
    let mut parsers = HashMap::new();
    parsers.insert("maven".into(), make_parser("maven", Some("mvn"), None));

    let detected = detector::detect_by_command("./mvnw clean install", &parsers);
    assert!(detected.is_some());
    assert_eq!(detected.unwrap().name, "maven");
}

/// Test heuristic detection from output content.
#[test]
fn test_detect_by_heuristic_maven() {
    let mut parsers = HashMap::new();
    parsers.insert("maven".into(), make_parser("maven", None, Some(r"\[INFO\]")));

    let output = "\
[INFO] Scanning for projects...
[INFO] Compiling 15 source files
[INFO] BUILD SUCCESS
";
    let detected = detector::detect_by_heuristic(output, &parsers);
    assert!(detected.is_some());
    assert_eq!(detected.unwrap().name, "maven");
}

/// Test heuristic detection with GCC-style output.
#[test]
fn test_detect_by_heuristic_gcc() {
    let mut parsers = HashMap::new();
    parsers.insert("gcc".into(), make_parser("gcc", None, Some(r"error:")));
    parsers.insert("cargo".into(), make_parser("cargo", None, Some(r"error\[E\d+\]")));

    let output = "\
test.c:10:5: error: expected ';' before 'return'
test.c:11:5: error: use of undeclared identifier 'foo'
";
    let detected = detector::detect_by_heuristic(output, &parsers);
    assert!(detected.is_some());
    assert_eq!(detected.unwrap().name, "gcc");
}

/// Test heuristic returns None when no heuristic patterns match.
#[test]
fn test_detect_by_heuristic_no_match() {
    let mut parsers = HashMap::new();
    parsers.insert("generic".into(), make_parser("generic", None, Some(r"ZZZ_PATTERN_ZZZ")));

    let output = "Some random output without any matching pattern.";
    let detected = detector::detect_by_heuristic(output, &parsers);
    assert!(detected.is_none());
}

/// Test heuristic detection with cargo error patterns.
#[test]
fn test_detect_by_heuristic_cargo() {
    let mut parsers = HashMap::new();
    parsers.insert("cargo".into(), make_parser("cargo", None, Some(r"error\[E\d+\]")));

    let output = "\
error[E0425]: cannot find value `foo` in this scope
  --> src/main.rs:10:9
error[E0425]: cannot find value `bar` in this scope
  --> src/main.rs:20:9
";
    let detected = detector::detect_by_heuristic(output, &parsers);
    assert!(detected.is_some());
    assert_eq!(detected.unwrap().name, "cargo");
}

/// Test that command detection takes priority over heuristic.
#[test]
fn test_command_detection_takes_priority() {
    let mut parsers = HashMap::new();
    parsers.insert("maven".into(), make_parser("maven", Some("mvn"), None));
    parsers.insert("generic".into(), make_parser("generic", None, Some(r"\[INFO\]")));

    let detected = detector::detect_by_command("mvn compile", &parsers);
    assert!(detected.is_some());
    assert_eq!(detected.unwrap().name, "maven");

    // Heuristic would match generic on the same output, but command detection is separate
    let output = "\
[INFO] Scanning for projects...
[INFO] BUILD SUCCESS
";
    let heuristic = detector::detect_by_heuristic(output, &parsers);
    assert!(heuristic.is_some());
    assert_eq!(heuristic.unwrap().name, "generic");
}

/// Test detection with empty command.
#[test]
fn test_detect_by_command_empty() {
    let parsers = HashMap::new();
    let detected = detector::detect_by_command("", &parsers);
    assert!(detected.is_none());
}

/// Test detection with empty parsers.
#[test]
fn test_detect_by_command_empty_parsers() {
    let parsers = HashMap::new();
    let detected = detector::detect_by_command("mvn compile", &parsers);
    assert!(detected.is_none());
}

/// Test heuristic detection with empty output.
#[test]
fn test_detect_by_heuristic_empty_output() {
    let mut parsers = HashMap::new();
    parsers.insert("cargo".into(), make_parser("cargo", None, Some(r"error\[E\d+\]")));

    let detected = detector::detect_by_heuristic("", &parsers);
    assert!(detected.is_none());
}
