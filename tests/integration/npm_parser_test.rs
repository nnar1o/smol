use smol::config;
use smol::core::SummaryStatus;
use smol::parse;

/// Parse npm install success output.
#[test]
fn test_npm_success_parser() {
    let output = "\
npm notice created a lockfile as package-lock.json
added 123 packages from 456 contributors and audited 789 packages in 12.345s
found 0 vulnerabilities
";

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("npm install", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
}

/// Parse npm install failure output (npm ERR!).
#[test]
fn test_npm_failure_parser() {
    let output = "\
npm ERR! code ENOENT
npm ERR! syscall open
npm ERR! path /home/user/project/package.json
npm ERR! errno -2
npm ERR! enoent ENOENT: no such file or directory, open '/home/user/project/package.json'
npm ERR! enoent This is related to npm not being able to find a file.
npm ERR! A complete log of this run can be found in:
npm ERR!     /home/user/.npm/_logs/2024-01-01T00_00_00_000Z-debug-0.log
";

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("npm install", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count > 0);
}

/// Parse npm from fixture file.
#[test]
fn test_npm_from_fixture() {
    let content = std::fs::read_to_string("tests/fixtures/outputs/npm_install.txt")
        .expect("fixture file not found");

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("npm install", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
}

/// Test npm detection by command prefix.
#[test]
fn test_npm_detection_by_command() {
    let parsers = config::load_all_parsers("/nonexistent").unwrap();

    let detected = smol::parse::detector::detect_by_command("npm run build", &parsers);
    assert!(detected.is_some());
    assert_eq!(detected.unwrap().name, "npm");
}
