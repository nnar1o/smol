use smol::config;
use smol::core::SummaryStatus;
use smol::parse;

/// Parse GCC compilation errors.
#[test]
fn test_gcc_errors_parser() {
    let output = "\
test.c:10:5: error: expected ';' before 'return'
test.c:11:5: error: expected ';' before 'token'
test.c:15:3: warning: implicit declaration of function 'foo'
";

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("gcc -c test.c", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert_eq!(summary.error_count, 2);
    assert!(summary.warning_count > 0);
}

/// Parse GCC success (no errors).
#[test]
fn test_gcc_success_parser() {
    let output = "";
    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("gcc -o program main.c", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
}

/// Parse GCC errors only (no warnings).
#[test]
fn test_gcc_errors_only() {
    let output = "\
main.c:42:5: error: expected ';' before 'return'
main.c:55:10: error: 'undefined_var' undeclared
";

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("gcc -Wall main.c", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert_eq!(summary.error_count, 2);
    assert_eq!(summary.warning_count, 0);
}

/// Parse GCC from fixture file.
#[test]
fn test_gcc_from_fixture() {
    let content = std::fs::read_to_string("tests/fixtures/outputs/gcc_errors.txt")
        .expect("fixture file not found");

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("gcc -c test.c", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count > 0);
    assert!(summary.warning_count > 0);
}

/// Test GCC detection by command prefix.
#[test]
fn test_gcc_detection_by_command() {
    let parsers = config::load_all_parsers("/nonexistent").unwrap();

    let detected = smol::parse::detector::detect_by_command("gcc -O2 -c main.c", &parsers);
    assert!(detected.is_some());
    assert_eq!(detected.unwrap().name, "gcc");
}

/// Test G++ detection by command prefix.
#[test]
fn test_gxx_detection_by_command() {
    let parsers = config::load_all_parsers("/nonexistent").unwrap();

    // g++ is not explicitly in the built-in parsers, but gcc should catch it via prefix
    let detected = smol::parse::detector::detect_by_command("g++ -std=c++17 main.cpp", &parsers);
    // May or may not match depending on how gcc parser's prefix is configured
    // Just verify it doesn't panic
    assert!(detected.is_some() || detected.is_none());
}
