use std::collections::HashMap;
use std::path::Path;

use crate::core::{ParserConfig, SmolError};

/// Built-in parser configs bundled with the binary.
const BUILT_IN_PARSERS: &[(&str, &str)] = &[
    ("generic", include_str!("../../parsers/generic.toml")),
    ("maven", include_str!("../../parsers/maven.toml")),
    ("cargo", include_str!("../../parsers/cargo.toml")),
    ("gcc", include_str!("../../parsers/gcc.toml")),
    ("docker", include_str!("../../parsers/docker.toml")),
    ("git", include_str!("../../parsers/git.toml")),
    ("gradle", include_str!("../../parsers/gradle.toml")),
    ("npm", include_str!("../../parsers/npm.toml")),
    ("yarn", include_str!("../../parsers/yarn.toml")),
    ("go", include_str!("../../parsers/go.toml")),
    ("tsc", include_str!("../../parsers/tsc.toml")),
    ("python", include_str!("../../parsers/python.toml")),
    ("rustc", include_str!("../../parsers/rustc.toml")),
    ("make", include_str!("../../parsers/make.toml")),
    ("kubectl", include_str!("../../parsers/kubectl.toml")),
    ("terraform", include_str!("../../parsers/terraform.toml")),
    ("jest", include_str!("../../parsers/jest.toml")),
    ("eslint", include_str!("../../parsers/eslint.toml")),
    ("node", include_str!("../../parsers/node.toml")),
    ("curl", include_str!("../../parsers/curl.toml")),
    ("cmake", include_str!("../../parsers/cmake.toml")),
    ("pip", include_str!("../../parsers/pip.toml")),
    ("pnpm", include_str!("../../parsers/pnpm.toml")),
    ("vite", include_str!("../../parsers/vite.toml")),
    ("ruby", include_str!("../../parsers/ruby.toml")),
    ("gh", include_str!("../../parsers/gh.toml")),
    ("aws", include_str!("../../parsers/aws.toml")),
    ("helm", include_str!("../../parsers/helm.toml")),
    ("ansible", include_str!("../../parsers/ansible.toml")),
    ("docker-compose", include_str!("../../parsers/docker-compose.toml")),
    ("grep", include_str!("../../parsers/grep.toml")),
    ("find", include_str!("../../parsers/find.toml")),
    ("psql", include_str!("../../parsers/psql.toml")),
    ("jq", include_str!("../../parsers/jq.toml")),
    ("systemctl", include_str!("../../parsers/systemctl.toml")),
];

/// Load all parsers: built-in first, then override from filesystem parsers_dir.
pub fn load_all_parsers(parsers_dir: &str) -> Result<HashMap<String, ParserConfig>, SmolError> {
    let mut parsers = HashMap::new();

    // 1. Load built-in parsers
    for (name, toml_str) in BUILT_IN_PARSERS {
        let config: ParserConfig = toml::from_str(toml_str)
            .map_err(|e| SmolError::Config(format!("Failed to parse built-in parser '{}': {}", name, e)))?;
        parsers.insert(name.to_string(), config);
    }

    // 2. Override/user parsers from filesystem
    let parsers_path = Path::new(parsers_dir);
    if parsers_path.exists() && parsers_path.is_dir() {
        use std::fs;
        for entry in fs::read_dir(parsers_path).map_err(SmolError::Io)? {
            let entry = entry.map_err(SmolError::Io)?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                let content = fs::read_to_string(&path).map_err(SmolError::Io)?;
                let config: ParserConfig = toml::from_str(&content)
                    .map_err(|e| SmolError::Config(format!("Failed to parse '{}': {}", path.display(), e)))?;
                let name = path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                parsers.insert(name, config);
            }
        }
    }

    Ok(parsers)
}

/// Load global smol config from default locations.
pub fn load_global_config() -> Result<crate::config::GlobalConfig, SmolError> {
    // Check: ./.smol/smol.toml > ~/.smol/smol.toml > defaults
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let local_path = Path::new("./.smol/smol.toml");
    let global_path = Path::new(&home).join(".smol/smol.toml");

    let config_path = if local_path.exists() {
        local_path
    } else if global_path.exists() {
        &global_path
    } else {
        return Ok(crate::config::GlobalConfig::default());
    };

    let content = std::fs::read_to_string(config_path)
        .map_err(SmolError::Io)?;
    #[derive(serde::Deserialize)]
    struct RawConfig {
        tasks_dir: Option<String>,
        parsers_dir: Option<String>,
        max_output_bytes: Option<u64>,
        auto_wait_secs: Option<u64>,
        max_errors: Option<usize>,
        max_warnings: Option<usize>,
    }

    let raw: RawConfig = toml::from_str(&content)
        .map_err(|e| SmolError::Config(format!("Invalid smol.toml: {}", e)))?;

    Ok(crate::config::GlobalConfig {
        tasks_dir: raw.tasks_dir.unwrap_or_else(|| format!("{}/.smol/tasks", home)),
        parsers_dir: raw.parsers_dir.unwrap_or_else(|| format!("{}/.smol/parsers", home)),
        max_output_bytes: raw.max_output_bytes.unwrap_or(10 * 1024 * 1024),
        auto_wait_secs: raw.auto_wait_secs.unwrap_or(5),
        max_errors: raw.max_errors.unwrap_or(3),
        max_warnings: raw.max_warnings.unwrap_or(5),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_built_in_parsers_load() {
        // Use temp dir for parsers (empty = only built-in)
        let dir = std::env::temp_dir();
        let parsers = load_all_parsers(dir.to_str().unwrap()).unwrap();
        // Originals
        assert!(parsers.contains_key("generic"), "generic parser must exist");
        assert!(parsers.contains_key("maven"), "maven parser must exist");
        assert!(parsers.contains_key("cargo"), "cargo parser must exist");
        assert!(parsers.contains_key("gcc"), "gcc parser must exist");
        assert!(parsers.contains_key("gradle"), "gradle parser must exist");
        assert!(parsers.contains_key("npm"), "npm parser must exist");
        assert!(parsers.contains_key("go"), "go parser must exist");
        assert!(parsers.contains_key("python"), "python parser must exist");
        assert!(parsers.contains_key("make"), "make parser must exist");
        // New batch A
        assert!(parsers.contains_key("cmake"), "cmake parser must exist");
        assert!(parsers.contains_key("pip"), "pip parser must exist");
        assert!(parsers.contains_key("pnpm"), "pnpm parser must exist");
        assert!(parsers.contains_key("vite"), "vite parser must exist");
        assert!(parsers.contains_key("ruby"), "ruby parser must exist");
        // New batch B
        assert!(parsers.contains_key("gh"), "gh parser must exist");
        assert!(parsers.contains_key("aws"), "aws parser must exist");
        assert!(parsers.contains_key("helm"), "helm parser must exist");
        assert!(parsers.contains_key("ansible"), "ansible parser must exist");
        assert!(parsers.contains_key("docker-compose"), "docker-compose parser must exist");
        // New batch C
        assert!(parsers.contains_key("grep"), "grep parser must exist");
        assert!(parsers.contains_key("find"), "find parser must exist");
        assert!(parsers.contains_key("psql"), "psql parser must exist");
        assert!(parsers.contains_key("jq"), "jq parser must exist");
        assert!(parsers.contains_key("systemctl"), "systemctl parser must exist");
        // Total count
        assert_eq!(parsers.len(), 35, "total built-in parsers should be 35");
    }

    #[test]
    fn test_all_built_in_parsers_valid_toml() {
        let dir = std::env::temp_dir();
        let parsers = load_all_parsers(dir.to_str().unwrap()).unwrap();
        for (name, config) in &parsers {
            assert!(!config.name.is_empty(), "parser '{}' has empty name", name);
            assert_eq!(&config.name, name, "parser key '{}' != config.name '{}'", name, config.name);
        }
    }

    #[test]
    fn test_load_global_config_defaults() {
        // Should return defaults when no config file exists
        let config = load_global_config().unwrap();
        assert_eq!(config.max_output_bytes, 10 * 1024 * 1024);
        assert_eq!(config.auto_wait_secs, 5);
    }
}
