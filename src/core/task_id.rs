use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// 8-character base62 task identifier (0-9, a-z, A-Z)
/// ~218 trillion possible values — collision-resistant for practical use.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct TaskId(String);

impl TaskId {
    /// Generate a new random task ID.
    pub fn new() -> Self {
        const BASE62: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let mut rng = rand::thread_rng();
        let id: String = (0..8).map(|_| {
            let idx = rng.gen_range(0..62);
            BASE62[idx] as char
        }).collect();
        Self(id)
    }

    /// Create a TaskId from a string without validation (for deserialization).
    pub fn from_raw(raw: String) -> Self {
        Self(raw)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for TaskId {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 8 {
            return Err("TaskId must be exactly 8 characters".into());
        }
        if !s.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err("TaskId must be alphanumeric (base62)".into());
        }
        Ok(Self(s.to_string()))
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id_length() {
        let id = TaskId::new();
        assert_eq!(id.as_str().len(), 8);
    }

    #[test]
    fn test_task_id_alphanumeric() {
        let id = TaskId::new();
        assert!(id.as_str().chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn test_task_id_from_str_valid() {
        let id = TaskId::from_str("a3f9k2X7").unwrap();
        assert_eq!(id.as_str(), "a3f9k2X7");
    }

    #[test]
    fn test_task_id_from_str_invalid_short() {
        assert!(TaskId::from_str("abc").is_err());
    }

    #[test]
    fn test_task_id_from_str_invalid_chars() {
        assert!(TaskId::from_str("a3f9k2x_").is_err());
    }

    #[test]
    fn test_task_id_uniqueness() {
        let ids: Vec<TaskId> = (0..100).map(|_| TaskId::new()).collect();
        let mut unique = ids.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(ids.len(), unique.len(), "task IDs should be unique");
    }

    #[test]
    fn test_task_id_display() {
        let id = TaskId::from_raw("Test1234".into());
        assert_eq!(format!("{}", id), "Test1234");
    }
}
