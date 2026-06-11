use thiserror::Error;

#[derive(Error, Debug)]
pub enum SmolError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Invalid task ID: {0}")]
    InvalidTaskId(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Command failed with exit code {0}")]
    CommandFailed(i32),

    #[error("Command timed out")]
    CommandTimedOut,

    #[error("Output exceeds max size ({0} bytes)")]
    OutputTooLarge(u64),

    #[error("{0}")]
    Other(String),
}

impl SmolError {
    pub fn config(msg: impl Into<String>) -> Self {
        SmolError::Config(msg.into())
    }

    pub fn other(msg: impl Into<String>) -> Self {
        SmolError::Other(msg.into())
    }
}
