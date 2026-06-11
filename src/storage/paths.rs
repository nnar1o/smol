/// Resolve standard smol directory paths.
pub fn home_dir() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/tmp".into())
}

pub fn smol_dir() -> String {
    format!("{}/.smol", home_dir())
}

pub fn tasks_dir() -> String {
    format!("{}/.smol/tasks", home_dir())
}

pub fn parsers_dir() -> String {
    format!("{}/.smol/parsers", home_dir())
}

pub fn default_config_path() -> String {
    format!("{}/.smol/smol.toml", home_dir())
}
