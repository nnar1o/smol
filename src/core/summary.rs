use serde::{Deserialize, Serialize};

/// A single error line found in the output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorLine {
    pub line_num: u64,
    pub content: String,
    pub file: Option<String>,
    pub file_line: Option<u64>,
    pub column: Option<u64>,
}

/// A single warning line found in the output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarningLine {
    pub line_num: u64,
    pub content: String,
    pub file: Option<String>,
    pub file_line: Option<u64>,
}

/// A single information/diagnostic line (non-error, non-warning).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfoLine {
    pub line_num: u64,
    pub content: String,
}

/// A single test failure with identifying info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFailure {
    pub suite: String,
    pub test: Option<String>,
    pub message: String,
}

/// Aggregated test results extracted from command output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub total: u32,
    pub passed: u32,
    pub failed: u32,
    pub errors: u32,
    pub skipped: u32,
    pub failures: Vec<TestFailure>,
}

/// Parsed summary of a command's output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub status: SummaryStatus,
    pub error_count: u32,
    pub warning_count: u32,
    pub info_count: u32,
    pub total_lines: u64,
    pub total_chars: u64,
    pub errors: Vec<ErrorLine>,
    pub warnings: Vec<WarningLine>,
    pub infos: Vec<InfoLine>,
    pub truncated: bool,
    /// The first N lines that were truncated (if truncation happened).
    pub truncated_count: Option<u64>,
    /// Estimated tokens in the raw command output.
    pub input_tokens: usize,
    /// Estimated tokens in the formatted summary.
    pub output_tokens: usize,
    /// Ratio of output_tokens / input_tokens (0.0–1.0).
    pub compression_ratio: f64,
    /// Extracted test results, if tests were detected in the output.
    pub tests: Option<TestResult>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SummaryStatus {
    Success,
    Failure,
    Unknown,
}

impl SummaryStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SummaryStatus::Success => "success",
            SummaryStatus::Failure => "failure",
            SummaryStatus::Unknown => "done",
        }
    }
}

impl Summary {
    pub fn new() -> Self {
        Self {
            status: SummaryStatus::Unknown,
            error_count: 0,
            warning_count: 0,
            info_count: 0,
            total_lines: 0,
            total_chars: 0,
            errors: Vec::new(),
            warnings: Vec::new(),
            infos: Vec::new(),
            truncated: false,
            truncated_count: None,
            input_tokens: 0,
            output_tokens: 0,
            compression_ratio: 1.0,
            tests: None,
        }
    }

    /// Estimate token count using a heuristic that accounts for
    /// symbol density in code: alphanumeric chars avg ~4/token,
    /// non-alphanumeric symbols avg ~2/token, whitespace is free.
    pub fn estimate_tokens(text: &str) -> usize {
        if text.is_empty() {
            return 0;
        }
        let alnum = text.chars().filter(|c| c.is_alphanumeric()).count();
        let symbols = text.chars().filter(|c| !c.is_alphanumeric() && !c.is_whitespace()).count();
        (alnum / 4 + symbols / 2).max(1)
    }
}

impl Default for Summary {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summary_defaults() {
        let s = Summary::new();
        assert_eq!(s.status, SummaryStatus::Unknown);
        assert_eq!(s.error_count, 0);
        assert_eq!(s.warning_count, 0);
        assert!(!s.truncated);
    }

    #[test]
    fn test_summary_status_equality() {
        assert_eq!(SummaryStatus::Success, SummaryStatus::Success);
        assert_ne!(SummaryStatus::Success, SummaryStatus::Failure);
    }

    #[test]
    fn test_summary_status_as_str() {
        assert_eq!(SummaryStatus::Success.as_str(), "success");
        assert_eq!(SummaryStatus::Failure.as_str(), "failure");
        assert_eq!(SummaryStatus::Unknown.as_str(), "done");
    }

    #[test]
    fn test_estimate_tokens_empty() {
        assert_eq!(Summary::estimate_tokens(""), 0);
    }

    #[test]
    fn test_estimate_tokens_short() {
        // alnum=3, symbols=0 → 3/4 + 0/2 = 0 → max(1,0) = 1
        assert_eq!(Summary::estimate_tokens("abc"), 1);
    }

    #[test]
    fn test_estimate_tokens_exact() {
        // alnum=4, symbols=0 → 4/4 + 0/2 = 1
        assert_eq!(Summary::estimate_tokens("abcd"), 1);
        // alnum=8, symbols=0 → 8/4 + 0/2 = 2
        assert_eq!(Summary::estimate_tokens("abcdefgh"), 2);
    }

    #[test]
    fn test_estimate_tokens_with_symbols() {
        // alnum=4 (abcd), symbols=4((+)/) → 4/4 + 4/2 = 1 + 2 = 3
        assert_eq!(Summary::estimate_tokens("a(b + c) / d"), 3);
    }

    #[test]
    fn test_summary_new_has_token_defaults() {
        let s = Summary::new();
        assert_eq!(s.input_tokens, 0);
        assert_eq!(s.output_tokens, 0);
        assert!((s.compression_ratio - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_compression_ratio_calculation() {
        let mut s = Summary::new();
        s.input_tokens = 100;
        s.output_tokens = 25;
        s.compression_ratio = 25.0 / 100.0;
        assert!((s.compression_ratio - 0.25).abs() < f64::EPSILON);
    }
}
