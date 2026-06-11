use smol::config;
use smol::core::SummaryStatus;
use smol::parse;

// ---------- cmake ----------

#[test]
fn test_cmake_success_parser() {
    let output = "\
-- The C compiler identification is GNU 11.4.0
-- The CXX compiler identification is GNU 11.4.0
-- Detecting C compiler ABI info
-- Detecting C compiler ABI info - done
-- Check for working C compiler: /usr/bin/gcc
-- Check for working C compiler: /usr/bin/gcc - works
-- Configuring done
-- Generating done
-- Build files have been written to: /home/user/build
";

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("cmake ..", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.warning_count, 0);
    assert!(summary.input_tokens > 0);
}

#[test]
fn test_cmake_failure_parser() {
    let output = "\
-- The C compiler identification is GNU 11.4.0
CMake Error at CMakeLists.txt:15 (find_package):
  By not providing \"FindOpenSSL.cmake\" in CMAKE_MODULE_PATH this project
  has asked CMake to find a package configuration file provided by
  \"OpenSSL\", but CMake did not find one.
CMake Error: Could not find OpenSSL
-- Configuring incomplete, errors occurred!
";

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("cmake ..", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count >= 2, "Should detect at least 2 errors, got {}", summary.error_count);
    assert_eq!(summary.warning_count, 0);
}

#[test]
fn test_cmake_from_fixture_success() {
    let content =
        std::fs::read_to_string("tests/fixtures/outputs/cmake_success.txt")
            .expect("fixture file not found");

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("cmake ..", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.warning_count, 0);
}

#[test]
fn test_cmake_from_fixture_failure() {
    let content =
        std::fs::read_to_string("tests/fixtures/outputs/cmake_failure.txt")
            .expect("fixture file not found");

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("cmake ..", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count > 0, "Should detect errors");
}

// ---------- pip ----------

#[test]
fn test_pip_success_parser() {
    let output = "\
Collecting requests
  Downloading requests-2.31.0-py3-none-any.whl (62 kB)
Installing collected packages: urllib3, idna, certifi, charset-normalizer, requests
Successfully installed certifi-2024.2.2 charset-normalizer-3.3.2 idna-3.6 requests-2.31.0 urllib3-2.1.0
";

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("pip install requests", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.warning_count, 0);
}

#[test]
fn test_pip_failure_parser() {
    let output = "\
Looking in indexes: https://pypi.org/simple, https://pypi.org/simple
ERROR: Could not find a version that satisfies the requirement nonexistent-package==999.0.0 (from versions: none)
ERROR: No matching distribution found for nonexistent-package==999.0.0
";

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("pip install nonexistent-package", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count >= 2, "Should detect at least 2 errors, got {}", summary.error_count);
    assert_eq!(summary.warning_count, 0);
}

#[test]
fn test_pip_from_fixture_success() {
    let content =
        std::fs::read_to_string("tests/fixtures/outputs/pip_success.txt")
            .expect("fixture file not found");

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("pip install requests", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.warning_count, 0);
}

#[test]
fn test_pip_from_fixture_failure() {
    let content =
        std::fs::read_to_string("tests/fixtures/outputs/pip_failure.txt")
            .expect("fixture file not found");

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("pip install nonexistent-package", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count > 0, "Should detect errors");
}

// ---------- pnpm ----------

#[test]
fn test_pnpm_success_parser() {
    let output = "\
Packages: +15
Progress: resolved 42, reused 38, downloaded 4, added 15, done
. prepare$ husky install
. prepare: Done
Done in 3.2s
";

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("pnpm install", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.warning_count, 0);
}

#[test]
fn test_pnpm_failure_parser() {
    let output = "\
Packages: +12
WARN deprecated left-pad@1.3.0: Legacy package
ERR_PNPM CONFLICTING_PEER_DEPENDENCY
Conflicting peer dependency: react@19.0.0 for react-dom@19.0.0
pnpm: Pnpm cannot find package react-dom from /home/user/project
";

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("pnpm install", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count >= 2, "Should detect at least 2 errors, got {}", summary.error_count);
    assert!(summary.warning_count >= 1, "Should detect at least 1 warning, got {}", summary.warning_count);
}

#[test]
fn test_pnpm_from_fixture_success() {
    let content =
        std::fs::read_to_string("tests/fixtures/outputs/pnpm_success.txt")
            .expect("fixture file not found");

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("pnpm install", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.warning_count, 0);
}

#[test]
fn test_pnpm_from_fixture_failure() {
    let content =
        std::fs::read_to_string("tests/fixtures/outputs/pnpm_failure.txt")
            .expect("fixture file not found");

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("pnpm install", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count > 0, "Should detect errors");
    assert!(summary.warning_count > 0, "Should detect warnings");
}

// ---------- vite ----------

#[test]
fn test_vite_success_parser() {
    let output = "\
  vite v5.4.2 building for production...
  transforming...
  ✓ 142 modules transformed.
  rendering...
  ✓ built in 2.45s
";

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("vite build", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.warning_count, 0);
}

#[test]
fn test_vite_failure_parser() {
    let output = "\
  vite v5.4.2 building for production...
  transforming...
  ✓ 42 modules transformed.
  ✗ ERROR: Build failed in 1.23s
  src/components/App.tsx:23:12: error: Type 'string' is not assignable to type 'number'
  error: Build failed with 1 error
";

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("vite build", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count >= 1, "Should detect at least 1 error, got {}", summary.error_count);
}

#[test]
fn test_vite_from_fixture_success() {
    let content =
        std::fs::read_to_string("tests/fixtures/outputs/vite_success.txt")
            .expect("fixture file not found");

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("vite build", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.warning_count, 0);
}

#[test]
fn test_vite_from_fixture_failure() {
    let content =
        std::fs::read_to_string("tests/fixtures/outputs/vite_failure.txt")
            .expect("fixture file not found");

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("vite build", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count > 0, "Should detect errors");
}

// ---------- ruby ----------

#[test]
fn test_ruby_success_parser() {
    let output = "\
Fetching source index from https://rubygems.org/
Resolving dependencies...
Installing rake 13.1.0
Using bundler 2.5.6
Bundle complete! 4 Gemfile dependencies, 4 gems now installed.
";

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("ruby bundle install", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.warning_count, 0);
}

#[test]
fn test_ruby_failure_parser() {
    let output = "\
app/models/user.rb:15: syntax error, unexpected tIDENTIFIER, expecting '}'
  validates :name, presence: true
                 ^
app/models/user.rb:18: syntax error, unexpected end-of-input, expecting '}'
Gem::LoadError: Could not find gem 'pg (~> 1.5)' in locally installed gems.
";

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("ruby -c app/models/user.rb", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count >= 3, "Should detect at least 3 errors, got {}", summary.error_count);
    assert_eq!(summary.warning_count, 0);
}

#[test]
fn test_ruby_from_fixture_success() {
    let content =
        std::fs::read_to_string("tests/fixtures/outputs/ruby_success.txt")
            .expect("fixture file not found");

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("ruby bundle install", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.warning_count, 0);
}

#[test]
fn test_ruby_from_fixture_failure() {
    let content =
        std::fs::read_to_string("tests/fixtures/outputs/ruby_failure.txt")
            .expect("fixture file not found");

    let parsers = config::load_all_parsers("/nonexistent").unwrap();
    let summary = parse::parse_output("ruby -c app/models/user.rb", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count > 0, "Should detect errors");
}
