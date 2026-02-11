//! Slack channel operations
//!
//! List channels, get channel info, and resolve channel names to IDs.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use tokio::time::sleep;

use super::client::SlackClient;
use super::config::config_path;
use super::types::{SlackChannel, SlackUser};

#[cfg(test)]
mod tests;

/// Cache expiry time (1 hour)
const CACHE_EXPIRY_SECS: u64 = 3600;

/// Cached user lookup data
#[derive(Serialize, Deserialize)]
struct UserCache {
    /// Timestamp when cache was created
    created: u64,
    /// User ID to username mapping
    users: HashMap<String, String>,
}

/// Get path to user cache file
fn user_cache_path() -> Option<PathBuf> {
    config_path().map(|p| p.with_file_name("slack_users_cache.json"))
}

/// Response from conversations.list API
#[derive(Deserialize)]
struct ConversationsListResponse {
    channels: Vec<ChannelResponse>,
    response_metadata: Option<ResponseMetadata>,
}

/// Response from conversations.info API
#[derive(Deserialize)]
struct ConversationsInfoResponse {
    channel: ChannelResponse,
}

/// Response from users.list API
#[derive(Deserialize)]
struct UsersListResponse {
    members: Vec<UserResponse>,
}

/// Raw channel data from API
#[derive(Deserialize)]
struct ChannelResponse {
    id: String,
    name: String,
    is_private: Option<bool>,
    is_member: Option<bool>,
    topic: Option<TopicResponse>,
    purpose: Option<TopicResponse>,
    num_members: Option<u32>,
    created: Option<i64>,
}

/// Raw user data from API
#[derive(Deserialize)]
struct UserResponse {
    id: String,
    team_id: Option<String>,
    name: String,
    real_name: Option<String>,
    is_bot: Option<bool>,
    deleted: Option<bool>,
    tz: Option<String>,
}

/// Topic or purpose field
#[derive(Deserialize)]
struct TopicResponse {
    value: String,
}

/// Pagination metadata
#[derive(Deserialize)]
struct ResponseMetadata {
    next_cursor: Option<String>,
}

impl From<ChannelResponse> for SlackChannel {
    fn from(r: ChannelResponse) -> Self {
        Self {
            id: r.id,
            name: r.name,
            is_private: r.is_private.unwrap_or(false),
            is_member: r.is_member.unwrap_or(false),
            topic: r.topic.map(|t| t.value).filter(|s| !s.is_empty()),
            purpose: r.purpose.map(|p| p.value).filter(|s| !s.is_empty()),
            num_members: r.num_members,
            created: r.created.unwrap_or(0),
        }
    }
}

impl From<UserResponse> for SlackUser {
    fn from(r: UserResponse) -> Self {
        Self {
            id: r.id,
            team_id: r.team_id,
            name: r.name,
            real_name: r.real_name,
            is_bot: r.is_bot.unwrap_or(false),
            deleted: r.deleted.unwrap_or(false),
            tz: r.tz,
        }
    }
}

/// List all accessible channels
#[cfg(not(tarpaulin_include))]
pub async fn list_channels(client: &SlackClient) -> Result<Vec<SlackChannel>> {
    let mut all_channels = Vec::new();
    let mut cursor: Option<String> = None;
    let mut first_request = true;

    loop {
        // Rate limit: delay between paginated requests (Tier 2 = ~20 req/min)
        if !first_request {
            sleep(Duration::from_millis(500)).await;
        }
        first_request = false;

        let mut params = vec![
            ("types", "public_channel"),
            ("exclude_archived", "true"),
            ("limit", "200"),
        ];

        let cursor_str;
        if let Some(ref c) = cursor {
            cursor_str = c.clone();
            params.push(("cursor", &cursor_str));
        }

        let response: ConversationsListResponse = client
            .get_with_params("conversations.list", &params)
            .await?;

        all_channels.extend(response.channels.into_iter().map(SlackChannel::from));

        // Check for more pages
        match response.response_metadata.and_then(|m| m.next_cursor) {
            Some(c) if !c.is_empty() => cursor = Some(c),
            _ => break,
        }
    }

    // Sort by name
    all_channels.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(all_channels)
}

/// Get detailed info for a specific channel
#[cfg(not(tarpaulin_include))]
pub async fn get_channel_info(client: &SlackClient, channel_id: &str) -> Result<SlackChannel> {
    let response: ConversationsInfoResponse = client
        .get_with_params("conversations.info", &[("channel", channel_id)])
        .await?;

    Ok(SlackChannel::from(response.channel))
}

/// Resolve a channel name (with or without #) to a channel ID
#[cfg(not(tarpaulin_include))]
pub async fn resolve_channel(client: &SlackClient, name_or_id: &str) -> Result<String> {
    // If it already looks like an ID (channel, group, DM, or user), return it
    // C = public channel, G = private channel, D = DM, U = user (for DM)
    if name_or_id.starts_with('C')
        || name_or_id.starts_with('G')
        || name_or_id.starts_with('D')
        || name_or_id.starts_with('U')
    {
        return Ok(name_or_id.to_string());
    }

    // Strip leading # if present
    let name = name_or_id.trim_start_matches('#');

    // List channels and find by name
    let channels = list_channels(client).await?;
    channels
        .iter()
        .find(|c| c.name == name)
        .map(|c| c.id.clone())
        .ok_or_else(|| anyhow::anyhow!("Channel not found: {}", name))
}

/// List all users in the workspace
#[cfg(not(tarpaulin_include))]
pub async fn list_users(client: &SlackClient) -> Result<Vec<SlackUser>> {
    let response: UsersListResponse = client.get("users.list").await?;

    let users: Vec<SlackUser> = response
        .members
        .into_iter()
        .map(SlackUser::from)
        .filter(|u| !u.deleted && !u.is_bot)
        .collect();

    Ok(users)
}

/// Build a lookup map from user ID to username (with caching)
#[cfg(not(tarpaulin_include))]
pub async fn build_user_lookup(client: &SlackClient) -> Result<HashMap<String, String>> {
    // Try to load from cache first
    if let Some(cached) = load_user_cache() {
        return Ok(cached);
    }

    // Fetch from API
    let users = list_users(client).await?;
    let lookup: HashMap<String, String> = users.into_iter().map(|u| (u.id, u.name)).collect();

    // Save to cache
    save_user_cache(&lookup);

    Ok(lookup)
}

/// Load user cache if valid
#[cfg(not(tarpaulin_include))]
fn load_user_cache() -> Option<HashMap<String, String>> {
    let path = user_cache_path()?;
    let contents = fs::read_to_string(&path).ok()?;
    let cache: UserCache = serde_json::from_str(&contents).ok()?;

    // Check if cache is expired
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()?
        .as_secs();

    if now - cache.created > CACHE_EXPIRY_SECS {
        return None;
    }

    Some(cache.users)
}

/// Save user lookup to cache
#[cfg(not(tarpaulin_include))]
fn save_user_cache(users: &HashMap<String, String>) {
    let Some(path) = user_cache_path() else {
        return;
    };

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let cache = UserCache {
        created: now,
        users: users.clone(),
    };

    if let Ok(json) = serde_json::to_string(&cache) {
        let _ = fs::write(&path, json);
    }
}
