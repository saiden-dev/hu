//! Slack service layer - business logic that returns data
//!
//! Functions in this module return typed data and never print.
//! They delegate to the existing submodule functions after config checks.

use anyhow::{bail, Result};

use super::channels;
use super::client::SlackClient;
use super::config::{self, SlackConfig};
use super::messages;
use super::search;
use super::types::{SlackChannel, SlackMessage, SlackSearchResult, SlackUser};

/// Get current configuration
#[cfg(not(tarpaulin_include))]
pub fn get_config() -> Result<SlackConfig> {
    config::load_config()
}

/// Check if API is configured, return error if not
pub fn ensure_configured(config: &SlackConfig) -> Result<()> {
    if !config.is_configured {
        bail!("Slack is not configured. Run `hu slack auth` to authenticate.");
    }
    Ok(())
}

/// Check if user token is configured, return error if not
pub fn ensure_user_token(config: &SlackConfig) -> Result<()> {
    if !config.oauth.has_user_token() {
        bail!("User token required. Run `hu slack auth --user-token <token>`");
    }
    Ok(())
}

/// List all channels
#[cfg(not(tarpaulin_include))]
pub async fn list_channels(client: &SlackClient) -> Result<Vec<SlackChannel>> {
    channels::list_channels(client).await
}

/// Get channel info by ID or name
#[cfg(not(tarpaulin_include))]
pub async fn get_channel_info(client: &SlackClient, channel: &str) -> Result<SlackChannel> {
    let channel_id = channels::resolve_channel(client, channel).await?;
    channels::get_channel_info(client, &channel_id).await
}

/// Get message history for a channel
#[cfg(not(tarpaulin_include))]
pub async fn get_history(
    client: &SlackClient,
    channel: &str,
    limit: usize,
) -> Result<Vec<SlackMessage>> {
    let channel_id = channels::resolve_channel(client, channel).await?;
    messages::get_history(client, &channel_id, limit).await
}

/// Send a message to a channel
#[cfg(not(tarpaulin_include))]
pub async fn send_message(
    client: &SlackClient,
    channel: &str,
    text: &str,
) -> Result<(String, String)> {
    let channel_id = channels::resolve_channel(client, channel).await?;
    messages::send_message(client, &channel_id, text).await
}

/// Search messages (requires user token)
#[cfg(not(tarpaulin_include))]
pub async fn search_messages(
    client: &SlackClient,
    query: &str,
    count: usize,
) -> Result<SlackSearchResult> {
    search::search_messages(client, query, count).await
}

/// List users
#[cfg(not(tarpaulin_include))]
pub async fn list_users(client: &SlackClient) -> Result<Vec<SlackUser>> {
    channels::list_users(client).await
}

/// Build user lookup map for DM resolution
#[cfg(not(tarpaulin_include))]
pub async fn build_user_lookup(
    client: &SlackClient,
) -> Result<std::collections::HashMap<String, String>> {
    channels::build_user_lookup(client).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_configured_fails_when_not_configured() {
        let config = SlackConfig {
            oauth: config::OAuthConfig::default(),
            default_channel: String::new(),
            is_configured: false,
        };
        let result = ensure_configured(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not configured"));
    }

    #[test]
    fn ensure_configured_succeeds_when_configured() {
        let config = SlackConfig {
            oauth: config::OAuthConfig {
                client_id: None,
                client_secret: None,
                bot_token: Some("xoxb-test".to_string()),
                user_token: None,
                team_id: Some("T123".to_string()),
                team_name: Some("Test".to_string()),
            },
            default_channel: String::new(),
            is_configured: true,
        };
        let result = ensure_configured(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn ensure_user_token_fails_when_missing() {
        let config = SlackConfig {
            oauth: config::OAuthConfig {
                client_id: None,
                client_secret: None,
                bot_token: Some("xoxb-test".to_string()),
                user_token: None,
                team_id: None,
                team_name: None,
            },
            default_channel: String::new(),
            is_configured: true,
        };
        let result = ensure_user_token(&config);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("User token required"));
    }

    #[test]
    fn ensure_user_token_succeeds_when_present() {
        let config = SlackConfig {
            oauth: config::OAuthConfig {
                client_id: None,
                client_secret: None,
                bot_token: Some("xoxb-test".to_string()),
                user_token: Some("xoxp-test".to_string()),
                team_id: None,
                team_name: None,
            },
            default_channel: String::new(),
            is_configured: true,
        };
        let result = ensure_user_token(&config);
        assert!(result.is_ok());
    }
}
