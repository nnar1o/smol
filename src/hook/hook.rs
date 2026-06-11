use std::fs;

pub struct HookManager;

impl HookManager {
    /// Install hooks for the given host.
    pub fn setup(host: &str) -> Result<String, String> {
        match host {
            "opencode" => Self::install_opencode_plugin(),
            "claude" | "claude-code" => Self::install_claude_hook(),
            _ => Err(format!("Unknown host: {}. Supported: opencode, claude", host)),
        }
    }

    /// Remove hooks for a specific host, or all if None.
    pub fn uninstall(host: Option<&str>) -> Result<String, String> {
        match host {
            Some("opencode") => Self::remove_opencode_plugin(),
            Some("claude") | Some("claude-code") => Self::remove_claude_hook(),
            Some(h) => Err(format!("Unknown host: {}", h)),
            None => {
                Self::remove_opencode_plugin().ok();
                Self::remove_claude_hook().ok();
                Ok("Removed hooks from all supported hosts".to_string())
            }
        }
    }

    /// List supported hosts.
    pub fn list_hosts() -> Vec<&'static str> {
        vec!["opencode", "claude"]
    }

    fn install_opencode_plugin() -> Result<String, String> {
        let plugin_dir = dirs::home_dir()
            .ok_or("Cannot find home directory")?
            .join(".config/opencode/plugins");

        fs::create_dir_all(&plugin_dir)
            .map_err(|e| format!("Cannot create plugin dir: {}", e))?;

        let plugin_path = plugin_dir.join("smol.js");
        let plugin_content = include_str!("opencode_plugin.js");

        fs::write(&plugin_path, plugin_content)
            .map_err(|e| format!("Cannot write plugin: {}", e))?;

        Ok(format!("Installed smol plugin for OpenCode at {:?}", plugin_path))
    }

    fn remove_opencode_plugin() -> Result<String, String> {
        let plugin_path = dirs::home_dir()
            .ok_or("Cannot find home directory")?
            .join(".config/opencode/plugins/smol.js");

        if plugin_path.exists() {
            fs::remove_file(&plugin_path)
                .map_err(|e| format!("Cannot remove plugin: {}", e))?;
        }

        Ok("Removed OpenCode plugin".to_string())
    }

    fn install_claude_hook() -> Result<String, String> {
        let hook_dir = dirs::home_dir()
            .ok_or("Cannot find home directory")?
            .join(".claude/hooks/pre-tool-use");

        fs::create_dir_all(&hook_dir)
            .map_err(|e| format!("Cannot create hook dir: {}", e))?;

        let hook_path = hook_dir.join("smol.sh");
        let hook_content = include_str!("claude_hook.sh");

        fs::write(&hook_path, hook_content)
            .map_err(|e| format!("Cannot write hook: {}", e))?;

        // Make executable on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&hook_path, fs::Permissions::from_mode(0o755))
                .map_err(|e| format!("Cannot set permissions: {}", e))?;
        }

        Ok(format!("Installed smol hook for Claude Code at {:?}", hook_path))
    }

    fn remove_claude_hook() -> Result<String, String> {
        let hook_path = dirs::home_dir()
            .ok_or("Cannot find home directory")?
            .join(".claude/hooks/pre-tool-use/smol.sh");

        if hook_path.exists() {
            fs::remove_file(&hook_path)
                .map_err(|e| format!("Cannot remove hook: {}", e))?;
        }

        Ok("Removed Claude Code hook".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_hosts() {
        let hosts = HookManager::list_hosts();
        assert!(hosts.contains(&"opencode"));
        assert!(hosts.contains(&"claude"));
    }

    #[test]
    fn test_setup_unknown_host() {
        let result = HookManager::setup("unknown-host");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown host"));
    }

    #[test]
    fn test_uninstall_unknown_host() {
        let result = HookManager::uninstall(Some("unknown-host"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown host"));
    }

    #[test]
    fn test_uninstall_none() {
        // Should not error when removing hooks that don't exist
        let result = HookManager::uninstall(None);
        assert!(result.is_ok());
    }
}
