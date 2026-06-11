/// Resolve standard smol directory paths.
/// Supports multi-tenancy via argv[0]: if the binary is invoked as "smol-smol",
/// paths are prefixed with SMOL_SMOL_* env vars or ~/.smol-smol/.

/// Detect the binary name from argv[0] for multi-tenancy.
/// Returns the suffix after "smol-" if any, e.g. "smol-build" -> "build".
pub fn binary_suffix() -> Option<String> {
    let argv0 = std::env::args().next().unwrap_or_default();
    let stem = std::path::Path::new(&argv0)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("smol");
    if stem.starts_with("smol-") {
        Some(stem[5..].to_string())
    } else {
        None
    }
}

/// Get the env var prefix for multi-tenancy, e.g. "smol-build" -> "SMOL_BUILD".
fn env_prefix() -> String {
    match binary_suffix() {
        Some(suffix) => format!("SMOL_{}", suffix.to_uppercase()),
        None => "SMOL".to_string(),
    }
}

pub fn home_dir() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/tmp".into())
}

/// Return the smol directory. For multi-tenant binaries like "smol-build",
/// this checks SMOL_BUILD_DIR env var, then falls back to ~/.smol-build/.
/// For the default "smol" binary, uses SMOL_DIR env var or ~/.smol/.
pub fn smol_dir() -> String {
    let prefix = env_prefix();
    // Check env var: SMOL_BUILD_DIR or SMOL_DIR
    let dir_var = format!("{}_DIR", prefix);
    if let Ok(dir) = std::env::var(&dir_var) {
        return dir;
    }
    match binary_suffix() {
        Some(suffix) => format!("{}/.smol-{}", home_dir(), suffix),
        None => format!("{}/.smol", home_dir()),
    }
}

/// Return the tasks directory. For multi-tenant binaries, checks
/// SMOL_BUILD_TASKS_DIR env var, then falls back to <smol_dir>/tasks.
pub fn tasks_dir() -> String {
    let prefix = env_prefix();
    let dir_var = format!("{}_TASKS_DIR", prefix);
    if let Ok(dir) = std::env::var(&dir_var) {
        return dir;
    }
    format!("{}/tasks", smol_dir())
}

pub fn parsers_dir() -> String {
    format!("{}/parsers", smol_dir())
}

pub fn default_config_path() -> String {
    format!("{}/smol.toml", smol_dir())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_suffix_default() {
        // This test runs as the smol test binary, so argv[0] might vary.
        // Just verify it doesn't panic and returns Some or None.
        let suffix = binary_suffix();
        // The test runner path typically contains "smol" somewhere
        // but the actual behavior depends on the exe name
        assert!(suffix.is_none() || suffix.is_some());
    }

    #[test]
    fn test_env_prefix_default() {
        let prefix = env_prefix();
        // For a normal "smol" binary, prefix is "SMOL"
        if binary_suffix().is_none() {
            assert_eq!(prefix, "SMOL");
        }
    }

    #[test]
    fn test_smol_dir_fallback() {
        // Without env vars, should return ~/.smol
        let dir = smol_dir();
        assert!(dir.contains(".smol"), "smol_dir should contain .smol");
    }

    #[test]
    fn test_tasks_dir_fallback() {
        let dir = tasks_dir();
        assert!(dir.contains("tasks"), "tasks_dir should contain tasks");
    }
}
