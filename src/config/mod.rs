pub mod loader;
pub mod sync;

use std::collections::HashMap;
use crate::core::ParserConfig;
use crate::core::SmolError;

/// The global smol configuration, loaded from smol.toml.
#[derive(Debug, Clone)]
pub struct GlobalConfig {
    /// Directory for task storage (default: ~/.smol/tasks)
    pub tasks_dir: String,
    /// Directory for parser configs (default: ~/.smol/parsers)
    pub parsers_dir: String,
    /// Max output size per task in bytes (default: 10MB)
    pub max_output_bytes: u64,
    /// Auto-mode wait time in seconds (default: 5)
    pub auto_wait_secs: u64,
    /// Max lines in summary per error/warning (default: 3 errors, 5 warnings)
    pub max_errors: usize,
    pub max_warnings: usize,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            tasks_dir: String::new(),
            parsers_dir: String::new(),
            max_output_bytes: 10 * 1024 * 1024,
            auto_wait_secs: 5,
            max_errors: 3,
            max_warnings: 5,
        }
    }
}

/// Load all parser configurations from built-in and filesystem.
pub fn load_all_parsers(parsers_dir: &str) -> Result<HashMap<String, ParserConfig>, SmolError> {
    loader::load_all_parsers(parsers_dir)
}

/// Load the global config.
pub fn load_global_config() -> Result<GlobalConfig, SmolError> {
    loader::load_global_config()
}
