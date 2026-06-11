use std::collections::HashMap;
use regex::Regex;

use crate::core::ParserConfig;

/// Detect the parser to use based on the command name.
/// Tries exact match first, then prefix match (e.g., "mvn" matches "mvn clean install").
pub fn detect_by_command(command: &str, parsers: &HashMap<String, ParserConfig>) -> Option<ParserConfig> {
    let cmd_name = command.split_whitespace().next()?;

    for config in parsers.values() {
        if let Some(ref prefix) = config.detection.command_prefix {
            if cmd_name == prefix {
                return Some(config.clone());
            }
        }
    }

    // Second pass: prefix match (e.g., "mvn" matches "./mvnw")
    for config in parsers.values() {
        if let Some(ref prefix) = config.detection.command_prefix {
            if cmd_name.ends_with(prefix) || cmd_name.contains(prefix) {
                return Some(config.clone());
            }
        }
    }

    None
}

/// Detect the parser to use based on output content heuristics.
/// Runs each parser's heuristic regex against the output and picks the best match.
pub fn detect_by_heuristic(output: &str, parsers: &HashMap<String, ParserConfig>) -> Option<ParserConfig> {
    let mut best: Option<(f64, ParserConfig)> = None;

    for config in parsers.values() {
        if let Some(ref heuristic) = config.detection.heuristic {
            if let Ok(re) = Regex::new(&heuristic.regex) {
                let matches = re.find_iter(output).count() as f64;
                let ratio = matches / output.lines().count().max(1) as f64;
                if matches >= heuristic.min_lines as f64 && ratio > 0.01 {
                    // Score based on match density
                    let score = ratio;
                    if best.as_ref().map_or(true, |(s, _)| score > *s) {
                        best = Some((score, config.clone()));
                    }
                }
            }
        }
    }

    best.map(|(_, config)| config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::DetectionConfig;

    fn make_parser(name: &str, prefix: Option<&str>) -> ParserConfig {
        ParserConfig {
            name: name.into(),
            detection: DetectionConfig {
                command_prefix: prefix.map(|s| s.into()),
                heuristic: None,
            },
            ignore_patterns: vec![],
            error_patterns: vec![],
            warning_patterns: vec![],
            info_patterns: vec![],
            status_patterns: crate::core::StatusPatterns {
                success: vec![],
                failure: vec![],
            },
            summary: crate::core::SummaryConfig {
                max_errors: 3,
                max_warnings: 5,
                show_error_lines: true,
            },
        }
    }

    #[test]
    fn test_detect_by_command_exact() {
        let mut parsers = HashMap::new();
        parsers.insert("maven".into(), make_parser("maven", Some("mvn")));
        parsers.insert("cargo".into(), make_parser("cargo", Some("cargo")));

        let detected = detect_by_command("mvn clean install", &parsers);
        assert!(detected.is_some());
        assert_eq!(detected.unwrap().name, "maven");
    }

    #[test]
    fn test_detect_by_command_no_match() {
        let mut parsers = HashMap::new();
        parsers.insert("maven".into(), make_parser("maven", Some("mvn")));

        let detected = detect_by_command("ls -la", &parsers);
        assert!(detected.is_none());
    }

    #[test]
    fn test_detect_by_heuristic_simple() {
        let mut parsers = HashMap::new();
        parsers.insert("generic".into(), make_parser("generic", None)); // no heuristic

        let detected = detect_by_heuristic("some random output", &parsers);
        assert!(detected.is_none());
    }
}
