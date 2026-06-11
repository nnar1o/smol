use smol::config;
use smol::config::GlobalConfig;
use tempfile::TempDir;

/// Test that loading all parsers loads built-in parsers.
#[test]
fn test_load_all_parsers_includes_builtins() {
    let temp = TempDir::new().unwrap();
    let parsers = config::load_all_parsers(temp.path().to_str().unwrap()).unwrap();

    assert!(parsers.contains_key("generic"), "generic parser must exist");
    assert!(parsers.contains_key("maven"), "maven parser must exist");
    assert!(parsers.contains_key("cargo"), "cargo parser must exist");
    assert!(parsers.contains_key("gcc"), "gcc parser must exist");
    assert!(parsers.contains_key("npm"), "npm parser must exist");
    assert!(parsers.contains_key("python"), "python parser must exist");
    assert!(parsers.contains_key("go"), "go parser must exist");
    assert!(parsers.contains_key("gradle"), "gradle parser must exist");
    assert!(parsers.contains_key("make"), "make parser must exist");
    assert!(parsers.contains_key("jest"), "jest parser must exist");
    assert!(parsers.contains_key("docker"), "docker parser must exist");
    assert!(parsers.contains_key("git"), "git parser must exist");
    assert!(parsers.contains_key("tsc"), "tsc parser must exist");
    assert!(parsers.contains_key("rustc"), "rustc parser must exist");
    assert!(parsers.contains_key("kubectl"), "kubectl parser must exist");
    assert!(parsers.contains_key("terraform"), "terraform parser must exist");
    assert!(parsers.contains_key("eslint"), "eslint parser must exist");
    assert!(parsers.contains_key("node"), "node parser must exist");
    assert!(parsers.contains_key("curl"), "curl parser must exist");
    assert!(parsers.contains_key("yarn"), "yarn parser must exist");
}

/// Test that each built-in parser has a valid name and detection config.
#[test]
fn test_builtin_parsers_have_valid_config() {
    let temp = TempDir::new().unwrap();
    let parsers = config::load_all_parsers(temp.path().to_str().unwrap()).unwrap();

    for (name, parser) in &parsers {
        assert_eq!(&parser.name, name, "Parser key should match its name field");
        assert!(!parser.name.is_empty(), "Parser name should not be empty");
        // Each parser should have at least one detection method
        assert!(
            parser.detection.command_prefix.is_some() || parser.detection.heuristic.is_some(),
            "Parser '{}' should have at least one detection method",
            name
        );
    }
}

/// Test loading parsers from a custom directory overrides built-ins.
#[test]
fn test_custom_parser_directory() {
    let temp = TempDir::new().unwrap();
    let parser_dir = temp.path().join("parsers");
    std::fs::create_dir_all(&parser_dir).unwrap();

    // Write a custom cargo parser
    let custom_toml = r#"
name = "cargo"
[detection]
command_prefix = "my-cargo"
[status_patterns]
success = []
failure = []
[summary]
"#;
    std::fs::write(parser_dir.join("cargo.toml"), custom_toml).unwrap();

    let parsers = config::load_all_parsers(parser_dir.to_str().unwrap()).unwrap();
    assert!(parsers.contains_key("cargo"));
    assert_eq!(
        parsers["cargo"].detection.command_prefix.as_deref(),
        Some("my-cargo"),
        "Custom parser should override the built-in"
    );
}

/// Test GlobalConfig defaults.
#[test]
fn test_global_config_defaults() {
    let config = GlobalConfig::default();
    assert_eq!(config.max_output_bytes, 10 * 1024 * 1024);
    assert_eq!(config.auto_wait_secs, 5);
    assert_eq!(config.max_errors, 3);
    assert_eq!(config.max_warnings, 5);
    assert!(config.tasks_dir.is_empty());
    assert!(config.parsers_dir.is_empty());
}

/// Test that loading global config without any config file returns defaults.
#[test]
fn test_load_global_config_defaults() {
    let config = config::load_global_config().unwrap();
    assert_eq!(config.max_output_bytes, 10 * 1024 * 1024);
    assert_eq!(config.auto_wait_secs, 5);
}
