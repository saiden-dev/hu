//! Sentry configuration
//!
//! Loads configuration from `~/.config/hu/settings.toml`

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Sentry configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SentryConfig {
    /// Auth token
    pub auth_token: Option<String>,
    /// Organization slug
    pub organization: Option<String>,
    /// Default project slug
    pub project: Option<String>,
}

impl SentryConfig {
    /// Check if configured with auth token
    #[must_use]
    pub fn is_configured(&self) -> bool {
        self.auth_token.is_some() && self.organization.is_some()
    }
}

/// Settings file structure
#[derive(Debug, Default, Deserialize)]
struct SettingsFile {
    sentry: Option<SentryConfig>,
}

/// Get path to config file
pub fn config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|p| p.join(".config").join("hu").join("settings.toml"))
}

/// Load Sentry config from settings file and environment
#[cfg(not(tarpaulin_include))]
pub fn load_config() -> Result<SentryConfig> {
    let mut config = SentryConfig::default();

    // Load from settings file
    if let Some(path) = config_path() {
        if path.exists() {
            let contents = fs::read_to_string(&path)?;
            let settings: SettingsFile = toml::from_str(&contents)?;
            if let Some(sentry) = settings.sentry {
                config = sentry;
            }
        }
    }

    // Override with environment variables
    if let Ok(token) = std::env::var("SENTRY_AUTH_TOKEN") {
        config.auth_token = Some(token);
    }
    if let Ok(org) = std::env::var("SENTRY_ORG") {
        config.organization = Some(org);
    }
    if let Ok(project) = std::env::var("SENTRY_PROJECT") {
        config.project = Some(project);
    }

    Ok(config)
}

/// Save auth token to config file
#[cfg(not(tarpaulin_include))]
pub fn save_auth_token(token: &str, org: &str) -> Result<()> {
    let path = config_path().ok_or_else(|| anyhow::anyhow!("Cannot determine config directory"))?;

    // Read existing or create new
    let contents = if path.exists() {
        fs::read_to_string(&path)?
    } else {
        String::new()
    };

    // Parse as TOML value
    let mut doc: toml::Value =
        toml::from_str(&contents).unwrap_or_else(|_| toml::Value::Table(toml::map::Map::new()));

    // Ensure sentry section exists
    let table = doc
        .as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("Config is not a table"))?;

    if !table.contains_key("sentry") {
        table.insert(
            "sentry".to_string(),
            toml::Value::Table(toml::map::Map::new()),
        );
    }

    let sentry = table
        .get_mut("sentry")
        .and_then(|v| v.as_table_mut())
        .ok_or_else(|| anyhow::anyhow!("sentry section is not a table"))?;

    sentry.insert(
        "auth_token".to_string(),
        toml::Value::String(token.to_string()),
    );
    sentry.insert(
        "organization".to_string(),
        toml::Value::String(org.to_string()),
    );

    // Write back
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
    fn test_sentry_config_is_configured_both_set() {
        let config = SentryConfig {
            auth_token: Some("token".to_string()),
            organization: Some("my-org".to_string()),
            project: None,
        };
        assert!(config.is_configured());
    }

    #[test]
    fn test_sentry_config_is_configured_only_token() {
        let config = SentryConfig {
            auth_token: Some("token".to_string()),
            organization: None,
            project: None,
        };
        assert!(!config.is_configured());
    }

    #[test]
    fn test_sentry_config_is_configured_only_org() {
        let config = SentryConfig {
            auth_token: None,
            organization: Some("my-org".to_string()),
            project: None,
        };
        assert!(!config.is_configured());
    }

    #[test]
    fn test_sentry_config_is_configured_neither() {
        let config = SentryConfig {
            auth_token: None,
            organization: None,
            project: None,
        };
        assert!(!config.is_configured());
    }

    #[test]
    fn test_sentry_config_default() {
        let config = SentryConfig::default();
        assert!(config.auth_token.is_none());
        assert!(config.organization.is_none());
        assert!(config.project.is_none());
        assert!(!config.is_configured());
    }

    #[test]
    fn test_config_path_returns_some() {
        let path = config_path();
        if let Some(p) = path {
            assert!(p.to_string_lossy().contains("settings.toml"));
        }
    }
}
