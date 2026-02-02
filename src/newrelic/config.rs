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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_newrelic_config_is_configured_both_set() {
        let config = NewRelicConfig {
            api_key: Some("NRAK-12345".to_string()),
            account_id: Some(12345),
        };
        assert!(config.is_configured());
    }

    #[test]
    fn test_newrelic_config_is_configured_only_api_key() {
        let config = NewRelicConfig {
            api_key: Some("NRAK-12345".to_string()),
            account_id: None,
        };
        assert!(!config.is_configured());
    }

    #[test]
    fn test_newrelic_config_is_configured_only_account_id() {
        let config = NewRelicConfig {
            api_key: None,
            account_id: Some(12345),
        };
        assert!(!config.is_configured());
    }

    #[test]
    fn test_newrelic_config_is_configured_neither() {
        let config = NewRelicConfig {
            api_key: None,
            account_id: None,
        };
        assert!(!config.is_configured());
    }

    #[test]
    fn test_newrelic_config_default() {
        let config = NewRelicConfig::default();
        assert!(config.api_key.is_none());
        assert!(config.account_id.is_none());
        assert!(!config.is_configured());
    }

    #[test]
    fn test_config_path_returns_some() {
        let path = config_path();
        if let Some(p) = path {
            assert!(p.to_string_lossy().contains("settings.toml"));
        }
    }

    #[test]
    fn test_newrelic_config_serialization() {
        let config = NewRelicConfig {
            api_key: Some("NRAK-test123".to_string()),
            account_id: Some(99999),
        };

        let serialized = serde_json::to_string(&config).unwrap();
        assert!(serialized.contains("api_key"));
        assert!(serialized.contains("NRAK-test123"));
        assert!(serialized.contains("account_id"));
        assert!(serialized.contains("99999"));
    }

    #[test]
    fn test_newrelic_config_deserialization() {
        let json = r#"{"api_key":"NRAK-abc","account_id":12345}"#;
        let config: NewRelicConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.api_key, Some("NRAK-abc".to_string()));
        assert_eq!(config.account_id, Some(12345));
    }

    #[test]
    fn test_newrelic_config_deserialization_null_fields() {
        let json = r#"{"api_key":null,"account_id":null}"#;
        let config: NewRelicConfig = serde_json::from_str(json).unwrap();
        assert!(config.api_key.is_none());
        assert!(config.account_id.is_none());
        assert!(!config.is_configured());
    }

    #[test]
    fn test_newrelic_config_deserialization_empty_object() {
        let json = r#"{}"#;
        let config: NewRelicConfig = serde_json::from_str(json).unwrap();
        assert!(config.api_key.is_none());
        assert!(config.account_id.is_none());
    }

    #[test]
    fn test_newrelic_config_clone() {
        let config = NewRelicConfig {
            api_key: Some("NRAK-xyz".to_string()),
            account_id: Some(54321),
        };
        let cloned = config.clone();
        assert_eq!(cloned.api_key, config.api_key);
        assert_eq!(cloned.account_id, config.account_id);
    }

    #[test]
    fn test_newrelic_config_debug() {
        let config = NewRelicConfig {
            api_key: Some("NRAK-key".to_string()),
            account_id: Some(11111),
        };
        let debug = format!("{:?}", config);
        assert!(debug.contains("NewRelicConfig"));
        assert!(debug.contains("api_key"));
        assert!(debug.contains("account_id"));
    }

    #[test]
    fn test_settings_file_deserialization_with_newrelic() {
        let toml = r#"
[newrelic]
api_key = "NRAK-fromfile"
account_id = 777
"#;
        let settings: SettingsFile = toml::from_str(toml).unwrap();
        assert!(settings.newrelic.is_some());
        let nr = settings.newrelic.unwrap();
        assert_eq!(nr.api_key, Some("NRAK-fromfile".to_string()));
        assert_eq!(nr.account_id, Some(777));
    }

    #[test]
    fn test_settings_file_deserialization_empty() {
        let toml = "";
        let settings: SettingsFile = toml::from_str(toml).unwrap();
        assert!(settings.newrelic.is_none());
    }

    #[test]
    fn test_settings_file_deserialization_without_newrelic() {
        let toml = r#"
[sentry]
auth_token = "secret"
"#;
        let settings: SettingsFile = toml::from_str(toml).unwrap();
        assert!(settings.newrelic.is_none());
    }

    #[test]
    fn test_settings_file_debug() {
        let settings = SettingsFile { newrelic: None };
        let debug = format!("{:?}", settings);
        assert!(debug.contains("SettingsFile"));
    }

    #[test]
    fn test_settings_file_default() {
        let settings = SettingsFile::default();
        assert!(settings.newrelic.is_none());
    }

    #[test]
    fn test_config_path_hu_directory() {
        if let Some(path) = config_path() {
            // Should be in ~/.config/hu/settings.toml
            let path_str = path.to_string_lossy();
            assert!(path_str.contains(".config"));
            assert!(path_str.contains("hu"));
            assert!(path_str.ends_with("settings.toml"));
        }
    }
}
