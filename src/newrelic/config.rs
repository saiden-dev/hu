//! New Relic configuration

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// New Relic configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NewRelicConfig {
    /// API key (NRAK-...)
    pub api_key: Option<String>,
    /// Account ID
    pub account_id: Option<i64>,
}

impl NewRelicConfig {
    /// Check if configured
    #[must_use]
    pub fn is_configured(&self) -> bool {
        self.api_key.is_some() && self.account_id.is_some()
    }
}

/// Settings file structure
#[derive(Debug, Default, Deserialize)]
struct SettingsFile {
    newrelic: Option<NewRelicConfig>,
}

/// Get path to config file
pub fn config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|p| p.join(".config").join("hu").join("settings.toml"))
}

/// Load config from settings file and environment
pub fn load_config() -> Result<NewRelicConfig> {
    let mut config = NewRelicConfig::default();

    // Load from settings file
    if let Some(path) = config_path() {
        if path.exists() {
            let contents = fs::read_to_string(&path)?;
            let settings: SettingsFile = toml::from_str(&contents)?;
            if let Some(nr) = settings.newrelic {
                config = nr;
            }
        }
    }

    // Override with environment variables
    if let Ok(key) = std::env::var("NEW_RELIC_API_KEY") {
        config.api_key = Some(key);
    }
    if let Ok(id) = std::env::var("NEW_RELIC_ACCOUNT_ID") {
        if let Ok(parsed) = id.parse() {
            config.account_id = Some(parsed);
        }
    }

    Ok(config)
}

/// Save config to file
pub fn save_config(api_key: &str, account_id: i64) -> Result<()> {
    let path = config_path().ok_or_else(|| anyhow::anyhow!("Cannot determine config directory"))?;

    let contents = if path.exists() {
        fs::read_to_string(&path)?
    } else {
        String::new()
    };

    let mut doc: toml::Value =
        toml::from_str(&contents).unwrap_or_else(|_| toml::Value::Table(toml::map::Map::new()));

    let table = doc
        .as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("Config is not a table"))?;

    if !table.contains_key("newrelic") {
        table.insert(
            "newrelic".to_string(),
            toml::Value::Table(toml::map::Map::new()),
        );
    }

    let nr = table
        .get_mut("newrelic")
        .and_then(|v| v.as_table_mut())
        .ok_or_else(|| anyhow::anyhow!("newrelic section is not a table"))?;

    nr.insert(
        "api_key".to_string(),
        toml::Value::String(api_key.to_string()),
    );
    nr.insert("account_id".to_string(), toml::Value::Integer(account_id));

    let output = toml::to_string_pretty(&doc)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&path, output)?;
    Ok(())
}
