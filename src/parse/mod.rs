pub mod detector;
pub mod engine;
pub mod generic;
pub mod summarizer;

use std::collections::HashMap;

use crate::core::{ParserConfig, SmolError, Summary};

/// Parse command output using the best matching parser.
/// First tries detection by command name, then by output heuristics,
/// and falls back to the generic parser.
pub fn parse_output(
    command: &str,
    stdout: &str,
    stderr: &str,
    parsers: &HashMap<String, ParserConfig>,
    max_errors: usize,
    max_warnings: usize,
) -> Result<Summary, SmolError> {
    let combined = if stderr.is_empty() { stdout.to_string() } else { format!("{}\n{}", stdout, stderr) };

    // 1. Try detection by command prefix
    let parser = detector::detect_by_command(command, parsers);

    // 2. If no match, try heuristic detection from output
    let parser = parser.or_else(|| detector::detect_by_heuristic(&combined, parsers));

    // 3. Fall back to generic
    let parser = parser.unwrap_or_else(|| parsers.get("generic").cloned().unwrap());

    // 4. Run the engine
    let mut summary = engine::run_parser(&parser, &combined)?;

    // Apply max_errors/max_warnings trimming for display
    summary = summarizer::trim_summary(summary, max_errors, max_warnings);

    Ok(summary)
}
