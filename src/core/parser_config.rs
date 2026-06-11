use serde::{Deserialize, Serialize};

/// How to detect which parser to use for a given command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionConfig {
    /// Match by command prefix (e.g., "mvn" matches "mvn clean install").
    pub command_prefix: Option<String>,
    /// Fallback: heuristic detection by output content.
    #[serde(default)]
    pub heuristic: Option<HeuristicConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeuristicConfig {
    /// Regex that must match the output.
    pub regex: String,
    /// Minimum number of matching lines before we consider it a match.
    #[serde(default = "default_min_lines")]
    pub min_lines: u32,
}

fn default_min_lines() -> u32 {
    3
}

/// A single pattern entry for matching errors, warnings, or info lines.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternEntry {
    pub regex: String,
    pub severity: PatternType,
    /// If true, this pattern indicates a fatal error (build failure, etc.)
    #[serde(default)]
    pub is_fatal: bool,
    /// Named capture group for the message text (default: "message").
    #[serde(default = "default_group")]
    pub group: String,
    /// Named capture groups for file/line/column extraction.
    #[serde(default = "default_file_group")]
    pub file_group: String,
    #[serde(default = "default_line_group")]
    pub line_group: String,
    #[serde(default = "default_column_group")]
    pub column_group: String,
}

fn default_group() -> String { "message".into() }
fn default_file_group() -> String { "file".into() }
fn default_line_group() -> String { "line".into() }
fn default_column_group() -> String { "column".into() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternType {
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "info")]
    Info,
    #[serde(rename = "ignore")]
    Ignore,
}

/// Patterns for detecting the final build status.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatusPatterns {
    #[serde(default)]
    pub success: Vec<StatusPattern>,
    #[serde(default)]
    pub failure: Vec<StatusPattern>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusPattern {
    pub regex: String,
    #[serde(default = "default_group")]
    pub group: String,
}

/// How to present the summary to the user.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SummaryConfig {
    #[serde(default = "default_max_errors")]
    pub max_errors: usize,
    #[serde(default = "default_max_warnings")]
    pub max_warnings: usize,
    #[serde(default = "default_true")]
    pub show_error_lines: bool,
}

fn default_max_errors() -> usize { 3 }
fn default_max_warnings() -> usize { 5 }
fn default_true() -> bool { true }

/// A pattern for extracting structured test results from command output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestPattern {
    /// Regex with named groups for extracting test counts.
    /// Supports: total, passed, failures, errors, skipped.
    pub summary_regex: String,
    /// Optional regex for extracting individual test failures.
    /// Supports: suite, test, message.
    pub failure_regex: Option<String>,
}

/// Full configuration for a single output parser.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserConfig {
    pub name: String,
    pub detection: DetectionConfig,
    #[serde(default)]
    pub ignore_patterns: Vec<String>,
    #[serde(default)]
    pub error_patterns: Vec<PatternEntry>,
    #[serde(default)]
    pub warning_patterns: Vec<PatternEntry>,
    #[serde(default)]
    pub info_patterns: Vec<PatternEntry>,
    #[serde(default)]
    pub status_patterns: StatusPatterns,
    #[serde(default)]
    pub summary: SummaryConfig,
    /// Patterns for extracting test results from output.
    #[serde(default)]
    pub test_patterns: Vec<TestPattern>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_config_deserialize_minimal() {
        let toml_str = r#"
name = "test-parser"

[detection]
command_prefix = "test-cmd"
"#;
        let config: ParserConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.name, "test-parser");
        assert_eq!(config.detection.command_prefix.as_deref(), Some("test-cmd"));
    }

    #[test]
    fn test_parser_config_deserialize_full() {
        let toml_str = r#"
name = "maven"
description = "Apache Maven parser"

ignore_patterns = [
    "^\\[INFO\\] ---",
    "^Downloading from",
]

[detection]
command_prefix = "mvn"

[detection.heuristic]
regex = "\\[INFO\\]"
min_lines = 3

[[error_patterns]]
regex = "^\\[ERROR\\]\\s+(.*)$"
severity = "error"

[[warning_patterns]]
regex = "^\\[WARNING\\]\\s+(.*)$"
severity = "warning"

[status_patterns]
success = []
failure = []

[summary]
max_errors = 3
max_warnings = 5
"#;
        let config: ParserConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.name, "maven");
        assert_eq!(config.error_patterns.len(), 1);
        assert_eq!(config.warning_patterns.len(), 1);
        assert_eq!(config.ignore_patterns.len(), 2);
        assert_eq!(config.summary.max_errors, 3);
    }

    #[test]
    fn test_detection_config_defaults() {
        let config: DetectionConfig = toml::from_str(r#"command_prefix = "mvn""#).unwrap();
        assert_eq!(config.command_prefix.as_deref(), Some("mvn"));
        assert!(config.heuristic.is_none());
    }
}
