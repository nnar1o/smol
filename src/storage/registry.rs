use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::core::{SmolError, TaskId, TaskStatus};

/// The tasks registry — an index of all known tasks.
/// Stored as registry.toml in the tasks directory.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Registry {
    pub tasks: Vec<RegistryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub id: TaskId,
    pub created_at: DateTime<Utc>,
    pub status: TaskStatus,
}

pub fn load_registry(tasks_dir: &str) -> Result<Registry, SmolError> {
    let path = Path::new(tasks_dir).join("registry.toml");
    if path.exists() {
        let content = std::fs::read_to_string(&path)?;
        toml::from_str(&content).map_err(|e| SmolError::Config(format!("Invalid registry: {}", e)))
    } else {
        Ok(Registry::default())
    }
}

pub fn save_registry(tasks_dir: &str, registry: &Registry) -> Result<(), SmolError> {
    let path = Path::new(tasks_dir).join("registry.toml");
    let content = toml::to_string_pretty(registry)
        .map_err(|e| SmolError::Config(format!("Failed to serialize registry: {}", e)))?;
    std::fs::write(&path, content)?;
    Ok(())
}
