use smol::config;
use smol::core::SummaryStatus;
use smol::parse;

/// Load the built-in maven parser and parse a BUILD SUCCESS output.
#[test]
fn test_maven_success_parser() {
    let output = "\
[INFO] Scanning for projects...
[INFO]
[INFO] --- maven-compiler-plugin:3.8.1:compile (default-compile) @ my-app ---
[INFO] Compiling 15 source files to /target/classes
[INFO]
[INFO] --- maven-jar-plugin:3.1.0:jar (default-jar) @ my-app ---
[INFO] Building jar: /target/my-app-1.0.jar
[INFO]
[INFO] BUILD SUCCESS
[INFO] ------------------------------------------------------------------------
[INFO] Total time:  2.456 s
";

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("mvn clean install", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.warning_count, 0);
    assert!(summary.input_tokens > 0);
}

/// Parse Maven BUILD FAILURE output and verify errors and warnings are detected.
#[test]
fn test_maven_failure_parser() {
    let output = "\
[INFO] Scanning for projects...
[INFO]
[INFO] --- maven-compiler-plugin:3.8.1:compile (default-compile) @ my-app ---
[ERROR] /src/main/java/App.java:42: error: cannot find symbol
[ERROR] /src/main/java/App.java:43: error: cannot find symbol
[ERROR] /src/main/java/App.java:44: error: cannot find symbol
[WARNING] /src/main/java/App.java:10: unchecked cast
[WARNING] /src/main/java/App.java:11: unchecked cast
[INFO]
[INFO] ------------------------------------------------------------------------
[INFO] BUILD FAILURE
";

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("mvn compile", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert_eq!(summary.error_count, 4); // 3 [ERROR] + BUILD FAILURE
    assert_eq!(summary.warning_count, 2);
}

/// Parse Maven output with stderr content (simulating mixed output).
#[test]
fn test_maven_with_stderr() {
    let stdout = "\
[INFO] Scanning for projects...
[INFO] BUILD SUCCESS
";
    let stderr = "Picked up JAVA_TOOL_OPTIONS: -Xmx512m";

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("mvn package", stdout, stderr, &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
}

/// Parse Maven output from fixture file.
#[test]
fn test_maven_from_fixture_success() {
    let content = std::fs::read_to_string("tests/fixtures/outputs/maven_success.txt")
        .expect("fixture file not found");

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("mvn install", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.warning_count, 0);
}

/// Parse Maven failure from fixture file.
#[test]
fn test_maven_from_fixture_failure() {
    let content = std::fs::read_to_string("tests/fixtures/outputs/maven_failure.txt")
        .expect("fixture file not found");

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("mvn compile", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count > 0, "Should detect errors");
    assert!(summary.warning_count > 0, "Should detect warnings");
}

/// Parse Maven Failsafe verify output (acceptance/integration tests).
/// This covers the full mvn verify lifecycle: Surefire unit tests,
/// then Failsafe integration tests, then verify phase.
#[test]
fn test_maven_failsafe_verify_acceptance_tests() {
    let content = std::fs::read_to_string("tests/fixtures/outputs/maven_failsafe_verify.txt")
        .expect("fixture file not found");

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("mvn verify", &content, "", &parsers, 5, 10).unwrap();

    // Should detect BUILD FAILURE
    assert_eq!(summary.status, SummaryStatus::Failure);
    // Should have extracted test results from the fixture
    let tests = summary.tests.expect("acceptance test results should be present");
    // The output has two Results blocks:
    //   Surefire: 25 total, 0 failures, 0 errors, 1 skipped
    //   Failsafe: 14 total, 1 failure, 0 errors, 0 skipped
    // We use max() semantics, so total=25, failures=1, skipped=1
    assert_eq!(tests.total, 25, "total should be max across all summary lines");
    assert_eq!(tests.failed, 1, "failures should capture the failsafe failure");
    assert_eq!(tests.errors, 0, "errors not set in surefire/failsafe format");
    assert_eq!(tests.skipped, 1, "skipped from surefire");
    // Should capture the PaymentGatewayIT failure
    assert!(!tests.failures.is_empty(), "should capture individual test failures");
    // Verify the test suite name appears in failures
    let has_payment_failure = tests.failures.iter().any(|f|
        f.suite.contains("PaymentGatewayIT")
    );
    assert!(has_payment_failure, "should detect PaymentGatewayIT as a failing suite");
}
