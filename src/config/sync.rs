use std::path::Path;
use std::process::Command;

/// Git sync operations for parsers and config.
pub struct ParserSync;

impl ParserSync {
    /// Initialize a git repository in the parsers directory.
    pub fn init(parsers_dir: &str) -> Result<String, String> {
        let dir = Path::new(parsers_dir);
        if !dir.exists() {
            std::fs::create_dir_all(dir)
                .map_err(|e| format!("Failed to create parsers directory: {}", e))?;
        }
        if !dir.join(".git").exists() {
            let output = Command::new("git")
                .args(["init"])
                .current_dir(dir)
                .output()
                .map_err(|e| format!("Failed to run git init: {}", e))?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("git init failed: {}", stderr));
            }
        }
        Ok(format!("Initialized git repo in {}", parsers_dir))
    }

    /// Add a git remote.
    pub fn set_remote(parsers_dir: &str, remote: &str, url: &str) -> Result<String, String> {
        let dir = Path::new(parsers_dir);
        let output = Command::new("git")
            .args(["remote", "add", remote, url])
            .current_dir(dir)
            .output()
            .map_err(|e| format!("Failed to add remote: {}", e))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("git remote add failed: {}", stderr));
        }
        Ok(format!("Added remote '{}': {}", remote, url))
    }

    /// Sync parsers (pull -> add -> commit -> push).
    pub fn sync(parsers_dir: &str, message: Option<&str>) -> Result<String, String> {
        let dir = Path::new(parsers_dir);

        if !dir.exists() {
            return Err(format!("Parsers directory does not exist: {}", parsers_dir));
        }

        // Pull latest (best-effort; may fail if no upstream)
        let _ = Command::new("git")
            .args(["pull", "--ff-only"])
            .current_dir(dir)
            .output();

        // Add all changes
        let add_output = Command::new("git")
            .args(["add", "-A"])
            .current_dir(dir)
            .output()
            .map_err(|e| format!("Git add failed: {}", e))?;
        if !add_output.status.success() {
            let stderr = String::from_utf8_lossy(&add_output.stderr);
            return Err(format!("git add failed: {}", stderr));
        }

        // Check if there's something to commit
        let status_output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(dir)
            .output()
            .map_err(|e| format!("Git status failed: {}", e))?;

        let status_str = String::from_utf8_lossy(&status_output.stdout);
        if status_str.trim().is_empty() {
            return Ok("Nothing to sync — no changes".to_string());
        }

        // Commit
        let commit_msg = message.unwrap_or("sync parsers");
        let commit_output = Command::new("git")
            .args(["commit", "-m", commit_msg])
            .current_dir(dir)
            .output()
            .map_err(|e| format!("Git commit failed: {}", e))?;
        if !commit_output.status.success() {
            let stderr = String::from_utf8_lossy(&commit_output.stderr);
            // If nothing to commit, that's fine
            if stderr.contains("nothing to commit") {
                return Ok("Nothing to commit".to_string());
            }
            return Err(format!("git commit failed: {}", stderr));
        }

        // Push (best-effort; may fail if no upstream)
        let push_output = Command::new("git")
            .args(["push"])
            .current_dir(dir)
            .output()
            .map_err(|e| format!("Git push failed: {}", e))?;
        if !push_output.status.success() {
            let stderr = String::from_utf8_lossy(&push_output.stderr);
            return Err(format!("git push failed: {}", stderr));
        }

        Ok(format!("Synced parsers in {}", parsers_dir))
    }

    /// List all parser config files (*.toml) in the parsers directory.
    pub fn list(parsers_dir: &str) -> Result<Vec<String>, String> {
        let dir = Path::new(parsers_dir);
        let mut parsers = Vec::new();

        if dir.exists() {
            for entry in std::fs::read_dir(dir).map_err(|e| format!("Cannot read dir: {}", e))? {
                let entry = entry.map_err(|e| format!("Cannot read entry: {}", e))?;
                let path = entry.path();
                if path.extension().map(|e| e == "toml").unwrap_or(false) {
                    if let Some(name) = path.file_stem().and_then(|n| n.to_str()) {
                        parsers.push(name.to_string());
                    }
                }
            }
        }

        Ok(parsers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_parsers_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let result = ParserSync::list(dir.path().to_str().unwrap()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_list_parsers_with_files() {
        let dir = tempfile::tempdir().unwrap();

        // Create some parser files
        std::fs::write(dir.path().join("rust.toml"), "").unwrap();
        std::fs::write(dir.path().join("python.toml"), "").unwrap();
        std::fs::write(dir.path().join("README.md"), "").unwrap();

        let result = ParserSync::list(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.contains(&"rust".to_string()));
        assert!(result.contains(&"python".to_string()));
    }

    #[test]
    fn test_init_creates_git_repo() {
        let dir = tempfile::tempdir().unwrap();
        let result = ParserSync::init(dir.path().to_str().unwrap()).unwrap();
        assert!(result.contains("Initialized"));
        assert!(dir.path().join(".git").exists());
    }

    #[test]
    fn test_init_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        ParserSync::init(dir.path().to_str().unwrap()).unwrap();
        // Second init should also succeed
        let result = ParserSync::init(dir.path().to_str().unwrap()).unwrap();
        assert!(result.contains("Initialized"));
    }

    #[test]
    fn test_sync_no_changes() {
        let dir = tempfile::tempdir().unwrap();
        ParserSync::init(dir.path().to_str().unwrap()).unwrap();

        // Create a file and commit it first so we have a clean state
        std::fs::write(dir.path().join("test.toml"), "key = 'value'\n").unwrap();
        let _ = ParserSync::sync(dir.path().to_str().unwrap(), Some("initial"));

        // Now sync again with no changes
        let result = ParserSync::sync(dir.path().to_str().unwrap(), Some("noop")).unwrap();
        assert!(result.contains("Nothing to sync") || result.contains("Nothing to commit"));
    }
}
