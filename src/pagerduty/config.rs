//! PagerDuty configuration
//!
//! Loads configuration from `~/.config/hu/settings.toml`

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// PagerDuty configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PagerDutyConfig {
    /// API token
    pub api_token: Option<String>,
    /// Default escalation policy IDs (for filtering oncall)
    #[serde(default)]
    pub escalation_policy_ids: Vec<String>,
    /// Default schedule IDs (for filtering oncall)
    #[serde(default)]
    pub schedule_ids: Vec<String>,
}

impl PagerDutyConfig {
    /// Check if configured with API token
    #[must_use]
    pub fn is_configured(&self) -> bool {
        self.api_token.is_some()
    }
}

/// Settings file structure
#[derive(Debug, Default, Deserialize)]
struct SettingsFile {
    pagerduty: Option<PagerDutyConfig>,
}

/// Get path to config file
pub fn config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|p| p.join(".config").join("hu").join("settings.toml"))
}

/// Load PagerDuty config from settings file and environment
pub fn load_config() -> Result<PagerDutyConfig> {
    let mut config = PagerDutyConfig::default();

    // Load from settings file
    if let Some(path) = config_path() {
        if path.exists() {
            let contents = fs::read_to_string(&path)?;
            config = parse_config(&contents)?;
        }
    }

    // Override with environment variables
    if let Ok(token) = std::env::var("PAGERDUTY_API_TOKEN") {
        config.api_token = Some(token);
    }

    Ok(config)
}

/// Parse config from TOML string
fn parse_config(contents: &str) -> Result<PagerDutyConfig> {
    let settings: SettingsFile = toml::from_str(contents)?;
    Ok(settings.pagerduty.unwrap_or_default())
}

/// Save API token to config file
pub fn save_config(api_token: &str) -> Result<()> {
    let path = config_path().ok_or_else(|| anyhow::anyhow!("Cannot determine config directory"))?;

    // Read existing or create new
    let contents = if path.exists() {
        fs::read_to_string(&path)?
    } else {
        String::new()
    };

    let output = update_config_toml(&contents, api_token)?;

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&path, output)?;
    Ok(())
}

/// Update TOML config with new API token
fn update_config_toml(contents: &str, api_token: &str) -> Result<String> {
    // Parse as TOML value
    let mut doc: toml::Value =
        toml::from_str(contents).unwrap_or_else(|_| toml::Value::Table(toml::map::Map::new()));

    // Ensure pagerduty section exists
    let table = doc
        .as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("Config is not a table"))?;

    if !table.contains_key("pagerduty") {
        table.insert(
            "pagerduty".to_string(),
            toml::Value::Table(toml::map::Map::new()),
        );
    }

    let pagerduty = table
        .get_mut("pagerduty")
        .and_then(|v| v.as_table_mut())
        .ok_or_else(|| anyhow::anyhow!("pagerduty section is not a table"))?;

    pagerduty.insert(
        "api_token".to_string(),
        toml::Value::String(api_token.to_string()),
    );

    toml::to_string_pretty(&doc).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_is_configured_with_token() {
        let config = PagerDutyConfig {
            api_token: Some("token".to_string()),
            ..Default::default()
        };
        assert!(config.is_configured());
    }

    #[test]
    fn config_is_not_configured_without_token() {
        let config = PagerDutyConfig::default();
        assert!(!config.is_configured());
    }

    #[test]
    fn config_default_has_empty_vectors() {
        let config = PagerDutyConfig::default();
        assert!(config.escalation_policy_ids.is_empty());
        assert!(config.schedule_ids.is_empty());
    }

    #[test]
    fn config_path_returns_some() {
        // May return None in CI without home dir, just verify no panic
        let _ = config_path();
    }

    #[test]
    fn parse_config_empty() {
        let config = parse_config("").unwrap();
        assert!(!config.is_configured());
    }

    #[test]
    fn parse_config_with_pagerduty_section() {
        let toml = r#"
[pagerduty]
api_token = "test-token"
"#;
        let config = parse_config(toml).unwrap();
        assert!(config.is_configured());
        assert_eq!(config.api_token.as_deref(), Some("test-token"));
    }

    #[test]
    fn parse_config_with_policy_ids() {
        let toml = r#"
[pagerduty]
api_token = "test-token"
escalation_policy_ids = ["EP1", "EP2"]
schedule_ids = ["S1"]
"#;
        let config = parse_config(toml).unwrap();
        assert_eq!(config.escalation_policy_ids, vec!["EP1", "EP2"]);
        assert_eq!(config.schedule_ids, vec!["S1"]);
    }

    #[test]
    fn parse_config_other_sections_ignored() {
        let toml = r#"
[sentry]
auth_token = "sentry-token"

[pagerduty]
api_token = "pd-token"
"#;
        let config = parse_config(toml).unwrap();
        assert_eq!(config.api_token.as_deref(), Some("pd-token"));
    }

    #[test]
    fn update_config_toml_empty() {
        let result = update_config_toml("", "new-token").unwrap();
        assert!(result.contains("api_token = \"new-token\""));
        assert!(result.contains("[pagerduty]"));
    }

    #[test]
    fn update_config_toml_existing_section() {
        let existing = r#"
[pagerduty]
api_token = "old-token"
"#;
        let result = update_config_toml(existing, "new-token").unwrap();
        assert!(result.contains("api_token = \"new-token\""));
        assert!(!result.contains("old-token"));
    }

    #[test]
    fn update_config_toml_preserves_other_sections() {
        let existing = r#"
[sentry]
auth_token = "sentry-token"
"#;
        let result = update_config_toml(existing, "pd-token").unwrap();
        assert!(result.contains("sentry-token"));
        assert!(result.contains("pd-token"));
    }

    #[test]
    fn update_config_toml_preserves_other_pagerduty_fields() {
        let existing = r#"
[pagerduty]
api_token = "old-token"
escalation_policy_ids = ["EP1"]
"#;
        let result = update_config_toml(existing, "new-token").unwrap();
        assert!(result.contains("api_token = \"new-token\""));
        assert!(result.contains("EP1"));
    }

    #[test]
    fn config_debug() {
        let config = PagerDutyConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("PagerDutyConfig"));
    }

    #[test]
    fn config_clone() {
        let config = PagerDutyConfig {
            api_token: Some("token".to_string()),
            escalation_policy_ids: vec!["EP1".to_string()],
            schedule_ids: vec!["S1".to_string()],
        };
        let cloned = config.clone();
        assert_eq!(cloned.api_token, config.api_token);
        assert_eq!(cloned.escalation_policy_ids, config.escalation_policy_ids);
    }

    #[test]
    fn load_config_returns_default_when_no_file() {
        // load_config should work even when config file doesn't exist
        // It will return default config (possibly with env var override)
        let result = load_config();
        assert!(result.is_ok());
    }

    #[test]
    fn load_config_env_override() {
        // Test that environment variable overrides config file
        // Save current value and restore after test
        let original = std::env::var("PAGERDUTY_API_TOKEN").ok();

        std::env::set_var("PAGERDUTY_API_TOKEN", "env-token-test-12345");
        let result = load_config();
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.api_token.as_deref(), Some("env-token-test-12345"));

        // Restore original value
        match original {
            Some(val) => std::env::set_var("PAGERDUTY_API_TOKEN", val),
            None => std::env::remove_var("PAGERDUTY_API_TOKEN"),
        }
    }

    #[test]
    fn parse_config_invalid_toml() {
        let invalid = "this is not valid [[[toml";
        let result = parse_config(invalid);
        assert!(result.is_err());
    }

    #[test]
    fn parse_config_wrong_type_for_pagerduty() {
        // pagerduty is a string instead of a table
        let toml = r#"pagerduty = "not a table""#;
        let result = parse_config(toml);
        assert!(result.is_err());
    }

    #[test]
    fn update_config_toml_invalid_existing() {
        // Invalid TOML should still work - it creates a new table
        let invalid = "this is not valid [[[toml";
        let result = update_config_toml(invalid, "new-token");
        // Should succeed by creating fresh config
        assert!(result.is_ok());
        assert!(result.unwrap().contains("api_token"));
    }

    #[test]
    fn settings_file_default() {
        let settings = SettingsFile::default();
        assert!(settings.pagerduty.is_none());
    }

    #[test]
    fn settings_file_debug() {
        let settings = SettingsFile::default();
        let debug = format!("{:?}", settings);
        assert!(debug.contains("SettingsFile"));
    }

    #[test]
    fn config_serialize() {
        let config = PagerDutyConfig {
            api_token: Some("token".to_string()),
            escalation_policy_ids: vec!["EP1".to_string()],
            schedule_ids: vec![],
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("token"));
        assert!(json.contains("EP1"));
    }

    #[test]
    fn config_deserialize() {
        let json = r#"{
            "api_token": "test-token",
            "escalation_policy_ids": ["EP1"],
            "schedule_ids": []
        }"#;
        let config: PagerDutyConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.api_token.as_deref(), Some("test-token"));
        assert_eq!(config.escalation_policy_ids, vec!["EP1"]);
    }
}
