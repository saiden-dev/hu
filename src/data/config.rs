use anyhow::Result;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct DataConfig {
    pub claude_dir: PathBuf,
    pub database: PathBuf,
    pub auto_sync_interval: u64,
    pub sync_on_start: bool,
}

impl Default for DataConfig {
    fn default() -> Self {
        Self {
            claude_dir: expand_path("~/.claude"),
            database: resolve_db_path("hu.db"),
            auto_sync_interval: 300,
            sync_on_start: true,
        }
    }
}

pub fn expand_path(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    if path == "~" {
        if let Some(home) = dirs::home_dir() {
            return home;
        }
    }
    PathBuf::from(path)
}

pub fn resolve_db_path(db: &str) -> PathBuf {
    let path = PathBuf::from(db);
    if path.is_absolute() {
        return path;
    }
    if db.starts_with("~/") {
        return expand_path(db);
    }
    config_dir().join(db)
}

fn config_dir() -> PathBuf {
    config_dir_with_home(dirs::home_dir())
}

fn config_dir_with_home(home: Option<PathBuf>) -> PathBuf {
    match home {
        Some(h) => h.join(".config").join("hu"),
        None => PathBuf::from(".config/hu"),
    }
}

#[cfg(not(tarpaulin_include))]
pub fn load_data_config() -> Result<DataConfig> {
    let config_path = config_dir().join("settings.toml");
    if !config_path.exists() {
        return Ok(DataConfig::default());
    }

    let content = std::fs::read_to_string(&config_path)?;
    load_from_toml(&content)
}

pub fn load_from_toml(content: &str) -> Result<DataConfig> {
    let table: toml::Value = content.parse()?;
    let mut config = DataConfig::default();

    if let Some(general) = table.get("general") {
        if let Some(claude_dir) = general.get("claude_dir").and_then(|v| v.as_str()) {
            config.claude_dir = expand_path(claude_dir);
        }
        if let Some(database) = general.get("database").and_then(|v| v.as_str()) {
            config.database = resolve_db_path(database);
        }
    }

    if let Some(sync) = table.get("sync") {
        if let Some(interval) = sync.get("auto_sync_interval").and_then(|v| v.as_integer()) {
            config.auto_sync_interval = interval as u64;
        }
        if let Some(on_start) = sync.get("sync_on_start").and_then(|v| v.as_bool()) {
            config.sync_on_start = on_start;
        }
    }

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_tilde() {
        let home = dirs::home_dir().unwrap();
        assert_eq!(expand_path("~/foo"), home.join("foo"));
        assert_eq!(expand_path("~"), home);
    }

    #[test]
    fn expand_absolute() {
        assert_eq!(expand_path("/usr/bin"), PathBuf::from("/usr/bin"));
    }

    #[test]
    fn expand_relative() {
        assert_eq!(expand_path("foo/bar"), PathBuf::from("foo/bar"));
    }

    #[test]
    fn resolve_db_absolute() {
        assert_eq!(
            resolve_db_path("/tmp/test.db"),
            PathBuf::from("/tmp/test.db")
        );
    }

    #[test]
    fn resolve_db_tilde() {
        let home = dirs::home_dir().unwrap();
        assert_eq!(resolve_db_path("~/data.db"), home.join("data.db"));
    }

    #[test]
    fn resolve_db_relative() {
        let expected = config_dir().join("hu.db");
        assert_eq!(resolve_db_path("hu.db"), expected);
    }

    #[test]
    fn default_config() {
        let config = DataConfig::default();
        assert!(config.claude_dir.ends_with(".claude"));
        assert!(config.database.ends_with("hu.db"));
        assert_eq!(config.auto_sync_interval, 300);
        assert!(config.sync_on_start);
    }

    #[test]
    fn load_from_empty_toml() {
        let config = load_from_toml("").unwrap();
        assert_eq!(config.auto_sync_interval, 300);
        assert!(config.sync_on_start);
    }

    #[test]
    fn load_from_full_toml() {
        let toml = r#"
[general]
claude_dir = "/custom/.claude"
database = "/custom/data.db"

[sync]
auto_sync_interval = 600
sync_on_start = false
"#;
        let config = load_from_toml(toml).unwrap();
        assert_eq!(config.claude_dir, PathBuf::from("/custom/.claude"));
        assert_eq!(config.database, PathBuf::from("/custom/data.db"));
        assert_eq!(config.auto_sync_interval, 600);
        assert!(!config.sync_on_start);
    }

    #[test]
    fn load_partial_toml() {
        let toml = r#"
[sync]
auto_sync_interval = 0
"#;
        let config = load_from_toml(toml).unwrap();
        assert_eq!(config.auto_sync_interval, 0);
        assert!(config.sync_on_start); // default preserved
    }

    #[test]
    fn config_dir_with_home_some() {
        let home = PathBuf::from("/home/user");
        let result = config_dir_with_home(Some(home));
        assert_eq!(result, PathBuf::from("/home/user/.config/hu"));
    }

    #[test]
    fn config_dir_with_home_none() {
        let result = config_dir_with_home(None);
        assert_eq!(result, PathBuf::from(".config/hu"));
    }

    #[test]
    fn load_invalid_toml() {
        let result = load_from_toml("not valid toml {{{");
        assert!(result.is_err());
    }
}
