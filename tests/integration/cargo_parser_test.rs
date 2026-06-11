use smol::config;
use smol::core::SummaryStatus;
use smol::parse;

/// Parse Cargo build success output.
#[test]
fn test_cargo_success_parser() {
    let output = "\
   Compiling my-app v0.1.0 (/home/user/my-app)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.42s
";

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("cargo build", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.warning_count, 0);
}

/// Parse Cargo build with errors.
#[test]
fn test_cargo_errors_parser() {
    let output = "\
   Compiling my-app v0.1.0 (/home/user/my-app)
error[E0425]: cannot find value `foo` in this scope
  --> src/main.rs:10:9
   |
10 |     let x = foo;
   |         ^^^ not found in this scope

error[E0425]: cannot find value `bar` in this scope
  --> src/main.rs:20:9
   |
20 |     let y = bar;
   |         ^^^ not found in this scope

error: could not compile `my-app` due to previous error
";

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("cargo build", output, "", &parsers, 10, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count > 0, "Should detect at least 1 error");
}

/// Parse Cargo build with warnings.
#[test]
fn test_cargo_warnings_parser() {
    let output = "\
   Compiling my-app v0.1.0 (/home/user/my-app)
warning: unused variable `x`
  --> src/main.rs:15:5
   |
15 |     let x = 42;
   |         ^^ help: if you intend to use `x`, use a variable name that starts with an underscore: `_x`

warning: unused import `std::collections::HashMap`
 --> src/lib.rs:1:5
  |
1 | use std::collections::HashMap;
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^

    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.33s
";

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("cargo build", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert!(summary.warning_count > 0, "Should detect warnings");
}

/// Parse Cargo output from fixture file.
#[test]
fn test_cargo_from_fixture_errors() {
    let content = std::fs::read_to_string("tests/fixtures/outputs/cargo_errors.txt")
        .expect("fixture file not found");

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("cargo build", &content, "", &parsers, 10, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count > 0);
}

/// Test cargo output with stderr.
#[test]
fn test_cargo_with_stderr() {
    let stdout = "\
   Compiling my-app v0.1.0 (/home/user/my-app)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.42s
";
    let stderr = "warning: unused import `std::io`";

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("cargo check", stdout, stderr, &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
}
