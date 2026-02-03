//! Slack configuration management
//!
//! Loads configuration from `~/.config/hu/settings.toml` with environment variable overrides.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[cfg(test)]
mod tests;

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
