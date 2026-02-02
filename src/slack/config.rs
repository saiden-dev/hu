//! Slack configuration management
//!
//! Loads configuration from `~/.config/hu/settings.toml` with environment variable overrides.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Slack configuration
#[derive(Debug, Clone, Default)]
pub struct SlackConfig {
    /// Default channel (e.g., "#general")
    pub default_channel: String,
    /// OAuth configuration
    pub oauth: OAuthConfig,
    /// Whether configuration is complete
    pub is_configured: bool,
}

/// OAuth 2.0 configuration for Slack
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OAuthConfig {
    /// OAuth client ID
    pub client_id: Option<String>,
    /// OAuth client secret
    pub client_secret: Option<String>,
    /// Bot token (xoxb-...)
    pub bot_token: Option<String>,
    /// User token (xoxp-...) - required for search API
    pub user_token: Option<String>,
    /// Team/workspace ID
    pub team_id: Option<String>,
    /// Team/workspace name
    pub team_name: Option<String>,
}

impl OAuthConfig {
    /// Check if OAuth is fully configured (bot token present)
    #[must_use]
    pub fn is_configured(&self) -> bool {
        self.bot_token
            .as_ref()
            .is_some_and(|t| t.starts_with("xoxb-"))
    }

    /// Check if user token is available (required for search)
    #[must_use]
    pub fn has_user_token(&self) -> bool {
        self.user_token
            .as_ref()
            .is_some_and(|t| t.starts_with("xoxp-"))
    }
}

/// Raw TOML structure for settings file
#[derive(Debug, Deserialize)]
struct SettingsFile {
    slack: Option<SlackSection>,
}

#[derive(Debug, Deserialize)]
struct SlackSection {
    default_channel: Option<String>,
    oauth: Option<OAuthSection>,
}

#[derive(Debug, Deserialize)]
struct OAuthSection {
    client_id: Option<String>,
    client_secret: Option<String>,
    bot_token: Option<String>,
    user_token: Option<String>,
    team_id: Option<String>,
    team_name: Option<String>,
}

/// Get the config file path
///
/// Uses `~/.config/hu/settings.toml` following XDG convention.
#[must_use]
pub fn config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|p| p.join(".config").join("hu").join("settings.toml"))
}

/// Load Slack configuration from settings file and environment variables
pub fn load_config() -> Result<SlackConfig> {
    let mut config = SlackConfig::default();

    // Try to load from settings file
    if let Some(path) = config_path() {
        if path.exists() {
            // debug!("Loading Slack config from {}", path.display());
            let contents = fs::read_to_string(&path).map_err(|e| {
                anyhow::anyhow!(format!("Failed to read {}: {}", path.display(), e))
            })?;

            let settings: SettingsFile = toml::from_str(&contents).map_err(|e| {
                anyhow::anyhow!(format!("Failed to parse {}: {}", path.display(), e))
            })?;

            if let Some(slack) = settings.slack {
                config.default_channel = slack.default_channel.unwrap_or_default();

                if let Some(oauth) = slack.oauth {
                    config.oauth = OAuthConfig {
                        client_id: oauth.client_id,
                        client_secret: oauth.client_secret,
                        bot_token: oauth.bot_token,
                        user_token: oauth.user_token,
                        team_id: oauth.team_id,
                        team_name: oauth.team_name,
                    };
                }
            }
        }
    }

    // Environment variable overrides
    if let Ok(token) = std::env::var("SLACK_BOT_TOKEN") {
        config.oauth.bot_token = Some(token);
    }
    if let Ok(token) = std::env::var("SLACK_USER_TOKEN") {
        config.oauth.user_token = Some(token);
    }
    if let Ok(channel) = std::env::var("SLACK_DEFAULT_CHANNEL") {
        config.default_channel = channel;
    }

    // Determine configuration status
    config.is_configured = config.oauth.is_configured();

    Ok(config)
}

/// Update OAuth tokens in the config file after successful authentication
pub fn update_oauth_tokens(bot_token: &str, team_id: &str, team_name: &str) -> Result<()> {
    let path = config_path()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine config directory".to_string()))?;

    // Read existing file
    let contents = if path.exists() {
        fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!(format!("Failed to read {}: {}", path.display(), e)))?
    } else {
        String::new()
    };

    // Parse as TOML value for modification
    let mut doc: toml::Value =
        toml::from_str(&contents).unwrap_or_else(|_| toml::Value::Table(toml::map::Map::new()));

    // Ensure slack.oauth section exists
    let table = doc
        .as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("Config is not a table".to_string()))?;

    if !table.contains_key("slack") {
        table.insert(
            "slack".to_string(),
            toml::Value::Table(toml::map::Map::new()),
        );
    }

    let slack = table
        .get_mut("slack")
        .and_then(|v| v.as_table_mut())
        .ok_or_else(|| anyhow::anyhow!("slack section is not a table".to_string()))?;

    if !slack.contains_key("oauth") {
        slack.insert(
            "oauth".to_string(),
            toml::Value::Table(toml::map::Map::new()),
        );
    }

    let oauth = slack
        .get_mut("oauth")
        .and_then(|v| v.as_table_mut())
        .ok_or_else(|| anyhow::anyhow!("slack.oauth section is not a table".to_string()))?;

    // Update tokens
    oauth.insert(
        "bot_token".to_string(),
        toml::Value::String(bot_token.to_string()),
    );
    oauth.insert(
        "team_id".to_string(),
        toml::Value::String(team_id.to_string()),
    );
    oauth.insert(
        "team_name".to_string(),
        toml::Value::String(team_name.to_string()),
    );

    // Write back
    let output = toml::to_string_pretty(&doc)
        .map_err(|e| anyhow::anyhow!(format!("Failed to serialize config: {}", e)))?;

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| anyhow::anyhow!(format!("Failed to create config directory: {}", e)))?;
    }

    fs::write(&path, output)
        .map_err(|e| anyhow::anyhow!(format!("Failed to write {}: {}", path.display(), e)))?;

    // debug!("Updated Slack OAuth tokens in {}", path.display());
    Ok(())
}

/// Update user token in the config file
pub fn update_user_token(user_token: &str) -> Result<()> {
    let path = config_path()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine config directory".to_string()))?;

    // Read existing file
    let contents = if path.exists() {
        fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!(format!("Failed to read {}: {}", path.display(), e)))?
    } else {
        String::new()
    };

    // Parse as TOML value for modification
    let mut doc: toml::Value =
        toml::from_str(&contents).unwrap_or_else(|_| toml::Value::Table(toml::map::Map::new()));

    // Ensure slack.oauth section exists
    let table = doc
        .as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("Config is not a table".to_string()))?;

    if !table.contains_key("slack") {
        table.insert(
            "slack".to_string(),
            toml::Value::Table(toml::map::Map::new()),
        );
    }

    let slack = table
        .get_mut("slack")
        .and_then(|v| v.as_table_mut())
        .ok_or_else(|| anyhow::anyhow!("slack section is not a table".to_string()))?;

    if !slack.contains_key("oauth") {
        slack.insert(
            "oauth".to_string(),
            toml::Value::Table(toml::map::Map::new()),
        );
    }

    let oauth = slack
        .get_mut("oauth")
        .and_then(|v| v.as_table_mut())
        .ok_or_else(|| anyhow::anyhow!("slack.oauth section is not a table".to_string()))?;

    // Update user token
    oauth.insert(
        "user_token".to_string(),
        toml::Value::String(user_token.to_string()),
    );

    // Write back
    let output = toml::to_string_pretty(&doc)
        .map_err(|e| anyhow::anyhow!(format!("Failed to serialize config: {}", e)))?;

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| anyhow::anyhow!(format!("Failed to create config directory: {}", e)))?;
    }

    fs::write(&path, output)
        .map_err(|e| anyhow::anyhow!(format!("Failed to write {}: {}", path.display(), e)))?;

    // debug!("Updated Slack user token in {}", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oauth_config_is_configured_with_valid_bot_token() {
        let config = OAuthConfig {
            client_id: None,
            client_secret: None,
            bot_token: Some("xoxb-12345-67890".to_string()),
            user_token: None,
            team_id: None,
            team_name: None,
        };
        assert!(config.is_configured());
    }

    #[test]
    fn test_oauth_config_is_configured_with_invalid_bot_token() {
        let config = OAuthConfig {
            client_id: None,
            client_secret: None,
            bot_token: Some("invalid-token".to_string()),
            user_token: None,
            team_id: None,
            team_name: None,
        };
        assert!(!config.is_configured());
    }

    #[test]
    fn test_oauth_config_is_configured_without_bot_token() {
        let config = OAuthConfig {
            client_id: None,
            client_secret: None,
            bot_token: None,
            user_token: None,
            team_id: None,
            team_name: None,
        };
        assert!(!config.is_configured());
    }

    #[test]
    fn test_oauth_config_has_user_token_with_valid_token() {
        let config = OAuthConfig {
            client_id: None,
            client_secret: None,
            bot_token: None,
            user_token: Some("xoxp-12345-67890".to_string()),
            team_id: None,
            team_name: None,
        };
        assert!(config.has_user_token());
    }

    #[test]
    fn test_oauth_config_has_user_token_with_invalid_token() {
        let config = OAuthConfig {
            client_id: None,
            client_secret: None,
            bot_token: None,
            user_token: Some("invalid-token".to_string()),
            team_id: None,
            team_name: None,
        };
        assert!(!config.has_user_token());
    }

    #[test]
    fn test_oauth_config_has_user_token_without_token() {
        let config = OAuthConfig {
            client_id: None,
            client_secret: None,
            bot_token: None,
            user_token: None,
            team_id: None,
            team_name: None,
        };
        assert!(!config.has_user_token());
    }

    #[test]
    fn test_config_path_returns_some() {
        // This test just verifies config_path returns Some on systems with a home dir
        let path = config_path();
        // On most systems this should return Some
        if let Some(p) = path {
            assert!(p.to_string_lossy().contains("settings.toml"));
        }
    }

    #[test]
    fn test_slack_config_default() {
        let config = SlackConfig::default();
        assert!(!config.is_configured);
        assert!(config.default_channel.is_empty());
        assert!(!config.oauth.is_configured());
    }

    #[test]
    fn test_oauth_config_default() {
        let config = OAuthConfig::default();
        assert!(config.client_id.is_none());
        assert!(config.client_secret.is_none());
        assert!(config.bot_token.is_none());
        assert!(config.user_token.is_none());
        assert!(config.team_id.is_none());
        assert!(config.team_name.is_none());
    }

    #[test]
    fn test_oauth_config_serialize_deserialize() {
        let config = OAuthConfig {
            client_id: Some("client123".to_string()),
            client_secret: Some("secret456".to_string()),
            bot_token: Some("xoxb-test".to_string()),
            user_token: Some("xoxp-test".to_string()),
            team_id: Some("T12345".to_string()),
            team_name: Some("Test Team".to_string()),
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: OAuthConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.client_id, Some("client123".to_string()));
        assert_eq!(deserialized.client_secret, Some("secret456".to_string()));
        assert_eq!(deserialized.bot_token, Some("xoxb-test".to_string()));
        assert_eq!(deserialized.user_token, Some("xoxp-test".to_string()));
        assert_eq!(deserialized.team_id, Some("T12345".to_string()));
        assert_eq!(deserialized.team_name, Some("Test Team".to_string()));
    }

    #[test]
    fn test_oauth_config_debug() {
        let config = OAuthConfig {
            client_id: Some("client123".to_string()),
            client_secret: None,
            bot_token: None,
            user_token: None,
            team_id: None,
            team_name: None,
        };

        let debug = format!("{:?}", config);
        assert!(debug.contains("OAuthConfig"));
        assert!(debug.contains("client123"));
    }

    #[test]
    fn test_oauth_config_clone() {
        let config = OAuthConfig {
            client_id: Some("client123".to_string()),
            client_secret: None,
            bot_token: Some("xoxb-test".to_string()),
            user_token: None,
            team_id: None,
            team_name: None,
        };

        let cloned = config.clone();
        assert_eq!(cloned.client_id, config.client_id);
        assert_eq!(cloned.bot_token, config.bot_token);
    }

    #[test]
    fn test_slack_config_clone() {
        let config = SlackConfig {
            default_channel: "general".to_string(),
            oauth: OAuthConfig::default(),
            is_configured: true,
        };

        let cloned = config.clone();
        assert_eq!(cloned.default_channel, "general");
        assert!(cloned.is_configured);
    }

    #[test]
    fn test_slack_config_debug() {
        let config = SlackConfig {
            default_channel: "test".to_string(),
            oauth: OAuthConfig::default(),
            is_configured: false,
        };

        let debug = format!("{:?}", config);
        assert!(debug.contains("SlackConfig"));
        assert!(debug.contains("test"));
    }

    #[test]
    fn test_settings_file_parse() {
        let toml_str = r##"
            [slack]
            default_channel = "general"

            [slack.oauth]
            client_id = "client123"
            client_secret = "secret456"
            bot_token = "xoxb-token"
            user_token = "xoxp-token"
            team_id = "T12345"
            team_name = "Test Team"
        "##;

        let settings: SettingsFile = toml::from_str(toml_str).unwrap();
        let slack = settings.slack.unwrap();
        assert_eq!(slack.default_channel, Some("general".to_string()));

        let oauth = slack.oauth.unwrap();
        assert_eq!(oauth.client_id, Some("client123".to_string()));
        assert_eq!(oauth.bot_token, Some("xoxb-token".to_string()));
        assert_eq!(oauth.team_name, Some("Test Team".to_string()));
    }

    #[test]
    fn test_settings_file_parse_empty() {
        let toml_str = "";
        let settings: SettingsFile = toml::from_str(toml_str).unwrap();
        assert!(settings.slack.is_none());
    }

    #[test]
    fn test_settings_file_parse_no_oauth() {
        let toml_str = r##"
            [slack]
            default_channel = "test"
        "##;

        let settings: SettingsFile = toml::from_str(toml_str).unwrap();
        let slack = settings.slack.unwrap();
        assert_eq!(slack.default_channel, Some("test".to_string()));
        assert!(slack.oauth.is_none());
    }

    #[test]
    fn test_settings_file_parse_partial_oauth() {
        let toml_str = r##"
            [slack.oauth]
            bot_token = "xoxb-test"
        "##;

        let settings: SettingsFile = toml::from_str(toml_str).unwrap();
        let slack = settings.slack.unwrap();
        let oauth = slack.oauth.unwrap();
        assert_eq!(oauth.bot_token, Some("xoxb-test".to_string()));
        assert!(oauth.client_id.is_none());
    }

    #[test]
    fn test_slack_section_debug() {
        let toml_str = r##"
            [slack]
            default_channel = "test"
        "##;

        let settings: SettingsFile = toml::from_str(toml_str).unwrap();
        let debug = format!("{:?}", settings);
        assert!(debug.contains("SettingsFile"));
    }

    #[test]
    fn test_oauth_section_debug() {
        let toml_str = r##"
            [slack.oauth]
            bot_token = "xoxb-test"
        "##;

        let settings: SettingsFile = toml::from_str(toml_str).unwrap();
        let debug = format!("{:?}", settings.slack.unwrap().oauth.unwrap());
        assert!(debug.contains("OAuthSection"));
    }
}
