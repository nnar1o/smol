pub mod task_id;
pub mod task;
pub mod summary;
pub mod parser_config;
pub mod error;

pub use task_id::TaskId;
pub use task::{Task, TaskStatus, TaskMode, TaskMeta};
pub use summary::{Summary, SummaryStatus, ErrorLine, WarningLine, InfoLine};
pub use parser_config::{ParserConfig, DetectionConfig, PatternEntry, PatternType, StatusPatterns, StatusPattern, SummaryConfig};
pub use error::SmolError;
