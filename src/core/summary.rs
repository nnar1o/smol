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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SummaryStatus {
    Success,
    Failure,
    Unknown,
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
        }
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
}
