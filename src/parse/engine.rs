use regex::Regex;

use crate::core::{ParserConfig, Summary, SummaryStatus, ErrorLine, WarningLine, InfoLine, SmolError};

/// Run a parser config against the combined output.
/// Returns a Summary with all matched errors, warnings, info lines.
pub fn run_parser(config: &ParserConfig, output: &str) -> Result<Summary, SmolError> {
    let mut summary = Summary::new();

    // Pre-compile all regexes
    let ignore_regexes: Vec<Regex> = config.ignore_patterns.iter()
        .filter_map(|p| Regex::new(p).ok())
        .collect();
    let error_regexes: Vec<(Regex, &crate::core::PatternEntry)> = config.error_patterns.iter()
        .filter_map(|p| Regex::new(&p.regex).ok().map(|r| (r, p)))
        .collect();
    let warning_regexes: Vec<(Regex, &crate::core::PatternEntry)> = config.warning_patterns.iter()
        .filter_map(|p| Regex::new(&p.regex).ok().map(|r| (r, p)))
        .collect();
    let info_regexes: Vec<(Regex, &crate::core::PatternEntry)> = config.info_patterns.iter()
        .filter_map(|p| Regex::new(&p.regex).ok().map(|r| (r, p)))
        .collect();
    let success_regexes: Vec<Regex> = config.status_patterns.success.iter()
        .filter_map(|p| Regex::new(&p.regex).ok())
        .collect();
    let failure_regexes: Vec<Regex> = config.status_patterns.failure.iter()
        .filter_map(|p| Regex::new(&p.regex).ok())
        .collect();

    let mut has_success = false;
    let mut has_failure = false;

    for (line_num, line) in output.lines().enumerate() {
        let line_num = line_num as u64 + 1;

        // Check ignore patterns first
        if ignore_regexes.iter().any(|r| r.is_match(line)) {
            continue;
        }

        // Check error patterns
        let mut matched = false;
        for (re, pattern) in &error_regexes {
            if let Some(caps) = re.captures(line) {
                let content = caps.name(&pattern.group).map_or(line, |m| m.as_str());
                let file = caps.name(&pattern.file_group).map(|m| m.as_str().to_string());
                let file_line = caps.name(&pattern.line_group).and_then(|m| m.as_str().parse::<u64>().ok());
                let column = caps.name(&pattern.column_group).and_then(|m| m.as_str().parse::<u64>().ok());

                summary.error_count += 1;
                summary.errors.push(ErrorLine {
                    line_num,
                    content: content.to_string(),
                    file,
                    file_line,
                    column,
                });
                matched = true;
                if pattern.is_fatal {
                    has_failure = true;
                }
                break;
            }
        }
        if matched { continue; }

        // Check warning patterns
        for (re, pattern) in &warning_regexes {
            if let Some(caps) = re.captures(line) {
                let content = caps.name(&pattern.group).map_or(line, |m| m.as_str());
                let file = caps.name(&pattern.file_group).map(|m| m.as_str().to_string());
                let file_line = caps.name(&pattern.line_group).and_then(|m| m.as_str().parse::<u64>().ok());

                summary.warning_count += 1;
                summary.warnings.push(WarningLine {
                    line_num,
                    content: content.to_string(),
                    file,
                    file_line,
                });
                matched = true;
                break;
            }
        }
        if matched { continue; }

        // Check info patterns
        for (re, _pattern) in &info_regexes {
            if re.is_match(line) {
                summary.info_count += 1;
                summary.infos.push(InfoLine {
                    line_num,
                    content: line.to_string(),
                });
                matched = true;
                break;
            }
        }
        if matched { continue; }

        // Check status patterns
        if !has_success && success_regexes.iter().any(|r| r.is_match(line)) {
            has_success = true;
        }
        if !has_failure && failure_regexes.iter().any(|r| r.is_match(line)) {
            has_failure = true;
        }
    }

    // Determine final status
    summary.status = if has_failure {
        SummaryStatus::Failure
    } else if has_success {
        SummaryStatus::Success
    } else if summary.error_count > 0 {
        SummaryStatus::Failure
    } else {
        SummaryStatus::Success
    };

    summary.total_lines = output.lines().count() as u64;
    summary.total_chars = output.len() as u64;

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::PatternType;

    fn maven_config() -> ParserConfig {
        ParserConfig {
            name: "maven".into(),
            detection: crate::core::DetectionConfig {
                command_prefix: Some("mvn".into()),
                heuristic: None,
            },
            ignore_patterns: vec![
                "^\\[INFO\\] ---".to_string(),
                "^Downloading".to_string(),
            ],
            error_patterns: vec![
                crate::core::PatternEntry {
                    regex: "^\\[ERROR\\]\\s+(.*)$".into(),
                    severity: PatternType::Error,
                    is_fatal: false,
                    group: "message".into(),
                    file_group: "file".into(),
                    line_group: "line".into(),
                    column_group: "column".into(),
                },
                crate::core::PatternEntry {
                    regex: "BUILD FAILURE".into(),
                    severity: PatternType::Error,
                    is_fatal: true,
                    group: "message".into(),
                    file_group: "file".into(),
                    line_group: "line".into(),
                    column_group: "column".into(),
                },
            ],
            warning_patterns: vec![
                crate::core::PatternEntry {
                    regex: "^\\[WARNING\\]\\s+(.*)$".into(),
                    severity: PatternType::Warning,
                    is_fatal: false,
                    group: "message".into(),
                    file_group: "file".into(),
                    line_group: "line".into(),
                    column_group: "column".into(),
                },
            ],
            info_patterns: vec![],
            status_patterns: crate::core::StatusPatterns {
                success: vec![
                    crate::core::StatusPattern { regex: "BUILD SUCCESS".into(), group: "message".into() },
                ],
                failure: vec![
                    crate::core::StatusPattern { regex: "BUILD FAILURE".into(), group: "message".into() },
                ],
            },
            summary: crate::core::SummaryConfig {
                max_errors: 3,
                max_warnings: 5,
                show_error_lines: true,
            },
        }
    }

    #[test]
    fn test_parse_maven_success() {
        let config = maven_config();
        let output = "\
[INFO] Scanning for projects...
[INFO] --- maven-compiler-plugin ---
[INFO] BUILD SUCCESS
";
        let summary = run_parser(&config, output).unwrap();
        assert_eq!(summary.status, SummaryStatus::Success);
        assert_eq!(summary.error_count, 0);
        assert_eq!(summary.warning_count, 0);
    }

    #[test]
    fn test_parse_maven_failure() {
        let config = maven_config();
        let output = "\
[INFO] Scanning for projects...
[ERROR] Failed to execute goal...
[WARNING] Some deprecated API
[ERROR] Compilation failure
[WARNING] Another warning
BUILD FAILURE
";
        let summary = run_parser(&config, output).unwrap();
        assert_eq!(summary.status, SummaryStatus::Failure);
        assert_eq!(summary.error_count, 3);  // 2x [ERROR] + 1x BUILD FAILURE
        assert_eq!(summary.warning_count, 2);
    }

    #[test]
    fn test_parse_ignore_patterns() {
        let config = maven_config();
        let output = "\
[INFO] --- maven-compiler-plugin ---
Downloading from central: https://...
[INFO] BUILD SUCCESS
";
        let summary = run_parser(&config, output).unwrap();
        // Ignored lines should not appear anywhere
        assert_eq!(summary.status, SummaryStatus::Success);
    }
}
