//! Slack service layer - business logic that returns data
//!
//! Functions in this module return typed data and never print.
//! They delegate to the existing submodule functions after config checks.

use std::collections::HashMap;

use anyhow::{bail, Result};

use super::auth;
use super::channels;
use super::client::SlackApi;
use super::config::{self, SlackConfig};
use super::messages;
use super::search;
use super::tidy;
use super::types::{
    AuthInfo, AuthResult, SlackChannel, SlackMessage, SlackSearchResult, SlackUser, TidySummary,
};

#[cfg(test)]
mod tests;

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
pub async fn list_channels(client: &impl SlackApi) -> Result<Vec<SlackChannel>> {
    channels::list_channels(client).await
}

/// Get channel info by ID or name
#[cfg(not(tarpaulin_include))]
pub async fn get_channel_info(client: &impl SlackApi, channel: &str) -> Result<SlackChannel> {
    let channel_id = channels::resolve_channel(client, channel).await?;
    channels::get_channel_info(client, &channel_id).await
}

/// Get message history for a channel
#[cfg(not(tarpaulin_include))]
pub async fn get_history(
    client: &impl SlackApi,
    channel: &str,
    limit: usize,
) -> Result<Vec<SlackMessage>> {
    let channel_id = channels::resolve_channel(client, channel).await?;
    messages::get_history(client, &channel_id, limit).await
}

/// Send a message to a channel
#[cfg(not(tarpaulin_include))]
pub async fn send_message(
    client: &impl SlackApi,
    channel: &str,
    text: &str,
) -> Result<(String, String)> {
    let channel_id = channels::resolve_channel(client, channel).await?;
    messages::send_message(client, &channel_id, text).await
}

/// Search messages (requires user token)
#[cfg(not(tarpaulin_include))]
pub async fn search_messages(
    client: &impl SlackApi,
    query: &str,
    count: usize,
) -> Result<SlackSearchResult> {
    search::search_messages(client, query, count).await
}

/// List users
#[cfg(not(tarpaulin_include))]
pub async fn list_users(client: &impl SlackApi) -> Result<Vec<SlackUser>> {
    channels::list_users(client).await
}

/// Build user lookup map for DM resolution
#[cfg(not(tarpaulin_include))]
pub async fn build_user_lookup(client: &impl SlackApi) -> Result<HashMap<String, String>> {
    channels::build_user_lookup(client).await
}

/// Parse an auth.test API response into structured `AuthInfo`
pub fn parse_auth_response(result: &serde_json::Value) -> AuthInfo {
    AuthInfo {
        user_id: result
            .get("user_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        user: result
            .get("user")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        team_id: result
            .get("team_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        team: result
            .get("team")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string(),
    }
}

/// Verify a token via auth.test API and return the raw response
#[cfg(not(tarpaulin_include))]
pub async fn verify_token(token: &str) -> Result<serde_json::Value> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://slack.com/api/auth.test")
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;

    let result: serde_json::Value = response.json().await?;

    if result.get("ok").and_then(serde_json::Value::as_bool) != Some(true) {
        let error = result
            .get("error")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown");
        bail!("Token validation failed: {}", error);
    }

    Ok(result)
}

/// Validate token format for bot tokens
pub fn validate_bot_token(token: &str) -> Result<()> {
    if !token.starts_with("xoxb-") {
        bail!("Invalid bot token format. Token should start with 'xoxb-'");
    }
    Ok(())
}

/// Validate token format for user tokens
pub fn validate_user_token(token: &str) -> Result<()> {
    if !token.starts_with("xoxp-") {
        bail!("Invalid user token format. Token should start with 'xoxp-'");
    }
    Ok(())
}

/// Authenticate with Slack -- handles bot token, user token, or OAuth flow
///
/// Returns an `AuthResult` indicating what happened, without printing.
#[cfg(not(tarpaulin_include))]
pub async fn authenticate(
    token: Option<&str>,
    user_token: Option<&str>,
    port: u16,
) -> Result<AuthResult> {
    if let Some(user_tok) = user_token {
        validate_user_token(user_tok)?;
        verify_token(user_tok).await?;
        config::update_user_token(user_tok)?;
        return Ok(AuthResult::UserTokenSaved);
    }

    if let Some(bot_token) = token {
        validate_bot_token(bot_token)?;
        let result = verify_token(bot_token).await?;
        let auth_info = parse_auth_response(&result);
        config::update_oauth_tokens(bot_token, &auth_info.team_id, &auth_info.team)?;
        return Ok(AuthResult::BotTokenSaved {
            team_name: auth_info.team,
        });
    }

    let result = auth::run_oauth_flow(port).await?;

    if result.success {
        Ok(AuthResult::OAuthCompleted {
            team_name: result.team_name,
        })
    } else {
        let error = result.error.unwrap_or_else(|| "Unknown error".to_string());
        bail!("Authentication failed: {}", error);
    }
}

/// Get current user info (whoami) by verifying the configured token
#[cfg(not(tarpaulin_include))]
pub async fn whoami(config: &SlackConfig) -> Result<AuthInfo> {
    let token = config
        .oauth
        .user_token
        .as_deref()
        .or(config.oauth.bot_token.as_deref())
        .ok_or_else(|| anyhow::anyhow!("No token configured"))?;

    let result = verify_token(token).await?;
    Ok(parse_auth_response(&result))
}

/// Run tidy operation and return structured results
#[cfg(not(tarpaulin_include))]
pub async fn run_tidy(
    client: &impl SlackApi,
    config: &SlackConfig,
    dry_run: bool,
) -> Result<(Vec<tidy::TidyResult>, TidySummary)> {
    let token = config
        .oauth
        .user_token
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("User token required for tidy"))?;

    let result = verify_token(token).await?;
    let auth_info = parse_auth_response(&result);

    let user_info = tidy::UserInfo {
        user_id: auth_info.user_id,
        name: auth_info.user,
        full_name: auth_info.team.clone(),
    };

    let results = tidy::tidy_channels(client, &user_info, dry_run).await?;
    let summary = compute_tidy_summary(&results);

    Ok((results, summary))
}

/// Compute summary counts from tidy results
pub fn compute_tidy_summary(results: &[tidy::TidyResult]) -> TidySummary {
    let mut marked_read = 0;
    let mut has_mentions = 0;
    let mut already_read = 0;

    for r in results {
        match &r.action {
            tidy::TidyAction::Skipped => already_read += 1,
            tidy::TidyAction::MarkedRead => marked_read += 1,
            tidy::TidyAction::HasMention(_) => has_mentions += 1,
        }
    }

    TidySummary {
        marked_read,
        has_mentions,
        already_read,
    }
}

/// Get config path for display purposes
#[must_use]
pub fn config_path() -> Option<std::path::PathBuf> {
    config::config_path()
}
