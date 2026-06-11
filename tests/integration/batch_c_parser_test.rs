use smol::config;
use smol::core::SummaryStatus;
use smol::parse;

// ---------- grep ----------

#[test]
fn test_grep_success_parser() {
    let output = "\
src/main.rs:42:fn process_data(data: &[u8]) -> Result<()> {
src/lib.rs:15:pub fn helper_function() -> String {
src/utils.rs:8:pub fn calculate_index(x: usize) -> usize {
src/cli.rs:23:fn parse_args() -> Args {
src/models.rs:31:pub fn validate_input(input: &str) -> bool {
";

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("grep -rn function src/", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.warning_count, 0);
    assert!(summary.input_tokens > 0);
}

#[test]
fn test_grep_failure_parser() {
    let output = "\
grep: largefile.bin: memory exhausted
grep: src/secret: Permission denied
grep: /nonexistent: No such file or directory
";

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("grep -r pattern .", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count >= 3, "Should detect at least 3 errors, got {}", summary.error_count);
    assert_eq!(summary.warning_count, 0);
}

#[test]
fn test_grep_from_fixture_success() {
    let content =
        std::fs::read_to_string("tests/fixtures/outputs/grep_success.txt")
            .expect("fixture file not found");

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("grep -rn function src/", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.warning_count, 0);
}

#[test]
fn test_grep_from_fixture_failure() {
    let content =
        std::fs::read_to_string("tests/fixtures/outputs/grep_failure.txt")
            .expect("fixture file not found");

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("grep -r pattern .", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count > 0, "Should detect errors");
}

// ---------- find ----------

#[test]
fn test_find_success_parser() {
    let output = "\
.
./src
./src/main.rs
./src/lib.rs
./src/utils.rs
./src/cli.rs
./src/models.rs
./tests
./tests/test_utils.rs
./Cargo.toml
";

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("find . -name \"*.rs\"", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.warning_count, 0);
    assert!(summary.input_tokens > 0);
}

#[test]
fn test_find_failure_parser() {
    let output = "\
find: /root: Permission denied
find: /root/.bashrc: Permission denied
find: /root/.ssh: Permission denied
find: /root/.config: Permission denied
";

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("find /root", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count >= 4, "Should detect at least 4 errors, got {}", summary.error_count);
    assert_eq!(summary.warning_count, 0);
}

#[test]
fn test_find_from_fixture_success() {
    let content =
        std::fs::read_to_string("tests/fixtures/outputs/find_success.txt")
            .expect("fixture file not found");

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("find . -name \"*.rs\"", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.warning_count, 0);
}

#[test]
fn test_find_from_fixture_failure() {
    let content =
        std::fs::read_to_string("tests/fixtures/outputs/find_failure.txt")
            .expect("fixture file not found");

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("find /root", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count > 0, "Should detect errors");
}

// ---------- psql ----------

#[test]
fn test_psql_success_parser() {
    let output = "\
 id |   name   |  email           
----+----------+------------------
  1 | Alice    | alice@example.com
  2 | Bob      | bob@example.com  
  3 | Charlie  | charlie@example.com
(3 rows)
";

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("psql -c \"SELECT * FROM users\"", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.warning_count, 0);
    assert!(summary.input_tokens > 0);
}

#[test]
fn test_psql_failure_parser() {
    let output = "\
psql: error: could not connect to server: Connection refused
\tIs the server running on host \"localhost\" (127.0.0.1) and accepting
\tTCP/IP connections on port 5432?
FATAL: password authentication failed for user \"admin\"
";

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("psql -h localhost -U admin", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count >= 2, "Should detect at least 2 errors, got {}", summary.error_count);
    assert_eq!(summary.warning_count, 0);
}

#[test]
fn test_psql_from_fixture_success() {
    let content =
        std::fs::read_to_string("tests/fixtures/outputs/psql_success.txt")
            .expect("fixture file not found");

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("psql -c \"SELECT * FROM users\"", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.warning_count, 0);
}

#[test]
fn test_psql_from_fixture_failure() {
    let content =
        std::fs::read_to_string("tests/fixtures/outputs/psql_failure.txt")
            .expect("fixture file not found");

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("psql -h localhost -U admin", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count > 0, "Should detect errors");
}

// ---------- jq ----------

#[test]
fn test_jq_success_parser() {
    let output = "\
{
  \"key\": \"val\",
  \"name\": \"test\",
  \"count\": 42,
  \"tags\": [\"a\", \"b\", \"c\"]
}
";

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("jq '.' data.json", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.warning_count, 0);
    assert!(summary.input_tokens > 0);
}

#[test]
fn test_jq_failure_parser() {
    let output = "\
jq: parse error: Invalid numeric literal at line 1, column 9
";

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("jq '.' invalid.json", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count >= 1, "Should detect at least 1 error, got {}", summary.error_count);
    assert_eq!(summary.warning_count, 0);
}

#[test]
fn test_jq_from_fixture_success() {
    let content =
        std::fs::read_to_string("tests/fixtures/outputs/jq_success.txt")
            .expect("fixture file not found");

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("jq '.' data.json", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.warning_count, 0);
}

#[test]
fn test_jq_from_fixture_failure() {
    let content =
        std::fs::read_to_string("tests/fixtures/outputs/jq_failure.txt")
            .expect("fixture file not found");

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("jq '.' invalid.json", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count > 0, "Should detect errors");
}

// ---------- systemctl ----------

#[test]
fn test_systemctl_success_parser() {
    let output = "\
\u{25cf} sshd.service - OpenSSH server daemon
     Loaded: loaded (/usr/lib/systemd/system/sshd.service; enabled; preset: enabled)
     Active: active (running) since Mon 2026-06-08 10:30:00 UTC; 3 days ago
   Main PID: 1234 (sshd)
      Tasks: 1 (limit: 2345)
     Memory: 5.2M
        CPU: 2.345s
     CGroup: /system.slice/sshd.service
             \u{2514}\u{2500}1234 /usr/sbin/sshd -D
";

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("systemctl status sshd", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.warning_count, 0);
    assert!(summary.input_tokens > 0);
}

#[test]
fn test_systemctl_failure_parser() {
    let output = "\
Unit nonexistent.service could not be found.
Failed to start nonexistent.service: Unit nonexistent.service not found.
";

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("systemctl start nonexistent", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count >= 2, "Should detect at least 2 errors, got {}", summary.error_count);
    assert_eq!(summary.warning_count, 0);
}

#[test]
fn test_systemctl_from_fixture_success() {
    let content =
        std::fs::read_to_string("tests/fixtures/outputs/systemctl_success.txt")
            .expect("fixture file not found");

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("systemctl status sshd", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.warning_count, 0);
}

#[test]
fn test_systemctl_from_fixture_failure() {
    let content =
        std::fs::read_to_string("tests/fixtures/outputs/systemctl_failure.txt")
            .expect("fixture file not found");

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("systemctl start nonexistent", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count > 0, "Should detect errors");
}
