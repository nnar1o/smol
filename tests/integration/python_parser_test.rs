use smol::config;
use smol::core::SummaryStatus;
use smol::parse;

/// Parse Python script success output.
#[test]
fn test_python_success_parser() {
    let output = "Hello, World!\n";
    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("python script.py", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
}

/// Parse Python script with Traceback error.
#[test]
fn test_python_traceback_parser() {
    let output = "\
Traceback (most recent call last):
  File \"/home/user/script.py\", line 5, in <module>
    print(x)
NameError: name 'x' is not defined
";

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("python script.py", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count > 0, "Should detect Traceback error");
}

/// Parse Python error from fixture (traceback).
#[test]
fn test_python_from_fixture() {
    let content = std::fs::read_to_string("tests/fixtures/outputs/python_traceback.txt")
        .expect("fixture file not found");

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("python3 script.py", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count > 0);
}

/// Test Python detection by command prefix.
#[test]
fn test_python_detection_by_command() {
    let parsers = config::load_all_parsers("/nonexistent").unwrap();

    let detected = smol::parse::detector::detect_by_command("python3 -m pytest", &parsers);
    assert!(detected.is_some());
    assert_eq!(detected.unwrap().name, "python");
}

/// Test Python detection for python2 variant.
#[test]
fn test_python2_detection() {
    let parsers = config::load_all_parsers("/nonexistent").unwrap();

    let detected = smol::parse::detector::detect_by_command("python2 script.py", &parsers);
    assert!(detected.is_some());
    assert_eq!(detected.unwrap().name, "python");
}

/// Test Python with stderr output (tracebacks often go to stderr).
#[test]
fn test_python_traceback_stderr() {
    let stdout = "some stdout output\n";
    let stderr = "\
Traceback (most recent call last):
  File \"script.py\", line 5, in <module>
    print(x)
NameError: name 'x' is not defined
";

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("python script.py", stdout, stderr, &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count > 0);
}
