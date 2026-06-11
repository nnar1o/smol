use smol::config;
use smol::core::SummaryStatus;
use smol::parse;

/// Parse Go build success (empty output).
#[test]
fn test_go_success_parser() {
    let output = "";
    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("go build", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
}

/// Parse Go build with errors.
#[test]
fn test_go_failure_parser() {
    let output = "\
# github.com/user/project
src/main.go:10:2: undefined: x
src/main.go:11:9: undefined: y
cannot find package \"example.com/pkg\" in any of:
\t/usr/local/go/src/example.com/pkg (from $GOROOT)
\t/home/user/go/src/example.com/pkg (from $GOPATH)
";

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("go build ./...", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count > 0, "Should detect Go build errors");
}

/// Parse Go from fixture file.
#[test]
fn test_go_from_fixture() {
    let content = std::fs::read_to_string("tests/fixtures/outputs/go_build_errors.txt")
        .expect("fixture file not found");

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("go build ./...", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count > 0);
}

/// Test Go detection by command prefix.
#[test]
fn test_go_detection_by_command() {
    let parsers = config::load_all_parsers("/nonexistent").unwrap();

    let detected = smol::parse::detector::detect_by_command("go build -o output", &parsers);
    assert!(detected.is_some());
    assert_eq!(detected.unwrap().name, "go");
}

/// Test go vet detection.
#[test]
fn test_go_vet_detection() {
    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let detected = smol::parse::detector::detect_by_command("go vet ./...", &parsers);
    assert!(detected.is_some());
    assert_eq!(detected.unwrap().name, "go");
}
